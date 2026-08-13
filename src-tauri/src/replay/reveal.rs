use std::path::Path;

pub(super) fn reveal_in_finder(file: &Path) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(file)
            .status()
            .ok()
            .filter(|status| status.success())
            .map(|_| ())
            .ok_or(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = file;
        Err(())
    }
}
