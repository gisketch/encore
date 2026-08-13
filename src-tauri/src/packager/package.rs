use super::{
    display_name, metadata_json, workspace::cleanup_interrupted_workspaces, FfmpegRunner,
    PackageRequest, RunnerFailure, SavedReplay, METADATA_FILENAME, REPLAY_FILENAME,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

static PARTIAL_NONCE: AtomicU64 = AtomicU64::new(1);
const MAX_NAME_SUFFIX: u32 = 10_000;

pub(crate) trait PackageFileSystem: Send + Sync {
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn create_dir(&self, path: &Path) -> io::Result<()>;
    fn create_lock(&self, path: &Path) -> io::Result<()>;
    fn try_exists(&self, path: &Path) -> io::Result<bool>;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn file_len(&self, path: &Path) -> io::Result<u64>;
    fn sync_file(&self, path: &Path) -> io::Result<()>;
    fn sync_dir(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

pub(crate) struct SystemPackageFileSystem;

impl PackageFileSystem for SystemPackageFileSystem {
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn create_lock(&self, path: &Path) -> io::Result<()> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.sync_all()
    }

    fn try_exists(&self, path: &Path) -> io::Result<bool> {
        path.try_exists()
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect()
    }

    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn file_len(&self, path: &Path) -> io::Result<u64> {
        Ok(fs::metadata(path)?.len())
    }

    fn sync_file(&self, path: &Path) -> io::Result<()> {
        File::open(path)?.sync_all()
    }

    fn sync_dir(&self, path: &Path) -> io::Result<()> {
        File::open(path)?.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }
}

pub(crate) struct ReplayPackager {
    destination: PathBuf,
    runner: Arc<dyn FfmpegRunner>,
    files: Arc<dyn PackageFileSystem>,
}

impl ReplayPackager {
    pub(crate) fn new(destination: PathBuf, runner: Arc<dyn FfmpegRunner>) -> Self {
        Self::with_files(destination, runner, Arc::new(SystemPackageFileSystem))
    }

    pub(crate) fn with_files(
        destination: PathBuf,
        runner: Arc<dyn FfmpegRunner>,
        files: Arc<dyn PackageFileSystem>,
    ) -> Self {
        Self {
            destination,
            runner,
            files,
        }
    }

    pub(crate) fn package(&self, request: &PackageRequest<'_>) -> Result<SavedReplay, String> {
        self.validate_inputs(request)?;
        self.files
            .create_dir_all(&self.destination)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        cleanup_interrupted_workspaces(self.files.as_ref(), &self.destination)?;

        for suffix in 0..=MAX_NAME_SUFFIX {
            let display_name = display_name(request.created_at_unix_ms, suffix)?;
            let completed = self.destination.join(&display_name);
            if self
                .files
                .try_exists(&completed)
                .map_err(|_| "export_destination_unavailable".to_string())?
            {
                continue;
            }

            let lock = self
                .destination
                .join(format!(".{display_name}.lock-{}", std::process::id()));
            match self.files.create_lock(&lock) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err("export_destination_unavailable".into()),
            }
            let partial = self.destination.join(format!(
                ".{display_name}.partial-{}-{}",
                std::process::id(),
                PARTIAL_NONCE.fetch_add(1, Ordering::Relaxed)
            ));
            let mut workspace =
                match PartialWorkspace::new(self.files.as_ref(), partial, lock.clone()) {
                    Ok(workspace) => workspace,
                    Err(code) => {
                        let _ = self.files.remove_file(&lock);
                        return Err(code);
                    }
                };

            if self
                .files
                .try_exists(&completed)
                .map_err(|_| "export_destination_unavailable".to_string())?
            {
                continue;
            }
            self.write_bundle(request, workspace.directory())?;
            if self
                .files
                .try_exists(&completed)
                .map_err(|_| "export_destination_unavailable".to_string())?
            {
                continue;
            }
            match self.files.rename(workspace.directory(), &completed) {
                Ok(()) => {}
                Err(_) if self.files.try_exists(&completed).unwrap_or(false) => continue,
                Err(_) => return Err("export_destination_unavailable".into()),
            }
            workspace.mark_published();
            self.files
                .sync_dir(&self.destination)
                .map_err(|_| "export_destination_unavailable".to_string())?;
            return Ok(SavedReplay {
                id: request.replay_id.to_owned(),
                display_name,
                replay_file: completed.join(REPLAY_FILENAME),
                metadata_file: completed.join(METADATA_FILENAME),
                bundle_directory: completed,
            });
        }
        Err("export_destination_unavailable".into())
    }

    fn validate_inputs(&self, request: &PackageRequest<'_>) -> Result<(), String> {
        let segments = request.lease.segments();
        let Some(first_segment) = segments.first() else {
            return Err("export_incompatible_segments".into());
        };
        let known_source = segments
            .iter()
            .filter_map(|segment| segment.source_id())
            .next();
        if known_source.is_some_and(|source| {
            segments
                .iter()
                .filter_map(|segment| segment.source_id())
                .any(|candidate| candidate != source)
        }) {
            return Err("export_incompatible_segments".into());
        }

        let expected = self.inspect(&first_segment.path)?;
        if !expected.is_supported_h264_video() {
            return Err("export_incompatible_segments".into());
        }
        for segment in &segments[1..] {
            let actual = self.inspect(&segment.path)?;
            if actual != expected {
                return Err("export_incompatible_segments".into());
            }
        }
        Ok(())
    }

    fn inspect(&self, path: &Path) -> Result<super::BundleMediaDescription, String> {
        self.runner.inspect(path).map_err(|failure| match failure {
            RunnerFailure::Unavailable => "export_ffmpeg_unavailable".into(),
            RunnerFailure::Failed => "export_incompatible_segments".into(),
        })
    }

    fn write_bundle(&self, request: &PackageRequest<'_>, partial: &Path) -> Result<(), String> {
        let manifest = partial.join("concat.txt");
        self.files
            .write_synced(&manifest, &concat_manifest(request))
            .map_err(|_| "export_destination_unavailable".to_string())?;
        let replay = partial.join(REPLAY_FILENAME);
        self.runner
            .concat(&manifest, &replay)
            .map_err(|failure| match failure {
                RunnerFailure::Unavailable => String::from("export_ffmpeg_unavailable"),
                RunnerFailure::Failed => String::from("export_concat_failed"),
            })?;
        self.files
            .remove_file(&manifest)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        if self
            .files
            .file_len(&replay)
            .ok()
            .filter(|size| *size > 0)
            .is_none()
        {
            return Err("export_concat_failed".into());
        }
        self.files
            .sync_file(&replay)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        let metadata = metadata_json(request)?;
        self.files
            .write_synced(&partial.join(METADATA_FILENAME), &metadata)
            .map_err(|_| "export_metadata_failed".to_string())?;
        self.files
            .sync_dir(partial)
            .map_err(|_| "export_destination_unavailable".to_string())
    }
}

fn concat_manifest(request: &PackageRequest<'_>) -> Vec<u8> {
    request
        .lease
        .segments()
        .iter()
        .map(|segment| {
            let path = segment.path.to_string_lossy();
            format!(
                "file '{}'\n",
                path.replace('\\', "\\\\").replace('\'', "'\\\\''")
            )
        })
        .collect::<String>()
        .into_bytes()
}

struct PartialWorkspace<'a> {
    files: &'a dyn PackageFileSystem,
    directory: PathBuf,
    lock: PathBuf,
    published: bool,
}

impl<'a> PartialWorkspace<'a> {
    fn new(
        files: &'a dyn PackageFileSystem,
        directory: PathBuf,
        lock: PathBuf,
    ) -> Result<Self, String> {
        files
            .create_dir(&directory)
            .map_err(|_| "export_destination_unavailable".to_string())?;
        Ok(Self {
            files,
            directory,
            lock,
            published: false,
        })
    }

    fn directory(&self) -> &Path {
        &self.directory
    }

    fn mark_published(&mut self) {
        self.published = true;
    }
}

impl Drop for PartialWorkspace<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = self.files.remove_dir_all(&self.directory);
        }
        let _ = self.files.remove_file(&self.lock);
    }
}
