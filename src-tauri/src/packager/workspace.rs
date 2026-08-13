use super::PackageFileSystem;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub(crate) fn cleanup_interrupted_workspaces(
    files: &dyn PackageFileSystem,
    destination: &Path,
) -> Result<(), String> {
    let entries = files
        .read_dir(destination)
        .map_err(|_| "export_destination_unavailable".to_string())?;
    for path in entries {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(pid) = interrupted_workspace_pid(name) else {
            continue;
        };
        if process_is_running(pid) {
            continue;
        }
        let cleanup = if name.contains(".partial-") {
            files.remove_dir_all(&path)
        } else {
            files.remove_file(&path)
        };
        cleanup.map_err(|_| "export_destination_unavailable".to_string())?;
    }
    Ok(())
}

fn interrupted_workspace_pid(name: &str) -> Option<u32> {
    let name = name.strip_prefix(".Encore Replay ")?;
    let (display, suffix) = name
        .split_once(".partial-")
        .or_else(|| name.split_once(".lock-"))?;
    valid_display_timestamp(display)?;
    let mut numbers = suffix.split('-');
    let pid = numbers.next()?.parse().ok()?;
    match (numbers.next(), numbers.next()) {
        (None, None) => Some(pid), // lock-PID
        (Some(nonce), None) if nonce.parse::<u64>().is_ok() && name.contains(".partial-") => {
            Some(pid)
        }
        _ => None,
    }
}

fn valid_display_timestamp(value: &str) -> Option<()> {
    let (timestamp, suffix) = if value.len() == 19 {
        (value, None)
    } else {
        (value.get(..19)?, value.get(19..)?.strip_prefix(' '))
    };
    if !timestamp
        .chars()
        .enumerate()
        .all(|(index, character)| match index {
            4 | 7 => character == '-',
            10 => character == ' ',
            13 | 16 => character == '.',
            _ => character.is_ascii_digit(),
        })
    {
        return None;
    }
    if suffix.is_some_and(|suffix| suffix.parse::<u32>().is_err()) {
        return None;
    }
    Some(())
}

fn process_is_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_accepts_only_generated_workspace_names() {
        assert_eq!(
            interrupted_workspace_pid(".Encore Replay 2026-08-12 10.20.30.partial-42-9"),
            Some(42)
        );
        assert_eq!(
            interrupted_workspace_pid(".Encore Replay 2026-08-12 10.20.30 2.lock-42"),
            Some(42)
        );
        assert_eq!(
            interrupted_workspace_pid(".Encore Replay notes.lock-42"),
            None
        );
        assert_eq!(
            interrupted_workspace_pid(".Encore Replay 2026-08-12 10.20.30.lock"),
            None
        );
    }
}
