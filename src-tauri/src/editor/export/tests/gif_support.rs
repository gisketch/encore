//! GIF-only test double, kept out of the already-tracked `support.rs` (at
//! its committed SCC baseline) so this ticket's fake-runner branching
//! lands in a file with room for it.

use super::super::GifRunner;
use crate::packager::RunnerFailure;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PaletteGenCall {
    pub filter: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PaletteUseCall {
    pub filter: String,
    pub palette_matches_prior_call: bool,
}

/// Records each pass's exact `filter` argument (for the "used the shared
/// 640px/10fps filter chain" assertion) and writes placeholder bytes at
/// the requested output, standing in for what real ffmpeg would produce.
pub(super) struct FakeGifRunner {
    pub palettegen_calls: Mutex<Vec<PaletteGenCall>>,
    pub paletteuse_calls: Mutex<Vec<PaletteUseCall>>,
    last_palette_output: Mutex<Option<PathBuf>>,
    pub fail_palettegen: bool,
}

impl FakeGifRunner {
    pub(super) fn new() -> Self {
        Self {
            palettegen_calls: Mutex::new(Vec::new()),
            paletteuse_calls: Mutex::new(Vec::new()),
            last_palette_output: Mutex::new(None),
            fail_palettegen: false,
        }
    }

    pub(super) fn failing() -> Self {
        Self {
            fail_palettegen: true,
            ..Self::new()
        }
    }
}

impl GifRunner for FakeGifRunner {
    fn palettegen(&self, _input: &Path, filter: &str, palette: &Path) -> Result<(), RunnerFailure> {
        if self.fail_palettegen {
            return Err(RunnerFailure::Failed);
        }
        self.palettegen_calls.lock().unwrap().push(PaletteGenCall {
            filter: filter.to_string(),
        });
        fs::write(palette, b"fake-palette").unwrap();
        *self.last_palette_output.lock().unwrap() = Some(palette.to_path_buf());
        Ok(())
    }

    fn paletteuse(
        &self,
        _input: &Path,
        palette: &Path,
        filter: &str,
        output: &Path,
    ) -> Result<(), RunnerFailure> {
        let palette_matches_prior_call = self
            .last_palette_output
            .lock()
            .unwrap()
            .as_deref()
            .is_some_and(|prior| prior == palette);
        self.paletteuse_calls.lock().unwrap().push(PaletteUseCall {
            filter: filter.to_string(),
            palette_matches_prior_call,
        });
        fs::write(output, b"GIF89a-fake-gif-bytes").unwrap();
        Ok(())
    }
}
