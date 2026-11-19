//! Desktop utilities: Standard folder opening via desktop environment handler.

use std::path::Path;
use std::process::Command;

/// Opens a directory in the user's default desktop handler (file manager or terminal).
pub fn open_folder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", path.display()));
    }

    // Prefer gio open if available (clean DBus activation without zombie processes)
    if let Ok(mut child) = Command::new("gio").arg("open").arg(path).spawn() {
        let _ = child.wait();
        return Ok(());
    }

    // Standard fallback to xdg-open
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to open directory {}: {e}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_folder_nonexistent() {
        let nonexistent = Path::new("/nonexistent_path_inerium_test_12345");
        assert!(open_folder(nonexistent).is_err());
    }
}
