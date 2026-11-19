//! Desktop utilities: Dynamic file manager & folder opening without terminal hijacking.

use std::path::Path;
use std::process::Command;

/// Known terminal emulators and CLI launchers that shouldn't open a folder directly
const TERMINAL_BINARIES: &[&str] = &[
    "kitty",
    "kitty-open",
    "alacritty",
    "foot",
    "ghostty",
    "wezterm",
    "xterm",
    "konsole",
    "gnome-terminal",
    "xfce4-terminal",
    "nvim",
    "vim",
    "nano",
];

/// Known graphical file managers to look for if default association is missing or hijacked
const GUI_FILE_MANAGERS: &[&str] = &[
    "dolphin",
    "nautilus",
    "thunar",
    "nemo",
    "pcmanfm-qt",
    "pcmanfm",
    "caja",
];

/// Check if a binary or desktop handler name refers to a terminal/editor
pub fn is_terminal_handler(handler: &str) -> bool {
    let lower = handler.to_lowercase();
    TERMINAL_BINARIES.iter().any(|term| lower.contains(term))
}

/// Check if an executable exists in PATH using pure standard library
pub fn is_in_path(bin: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full = dir.join(bin);
            if full.is_file() {
                return true;
            }
        }
    }
    false
}

/// Find the first available GUI file manager binary in PATH
pub fn find_installed_file_manager() -> Option<&'static str> {
    GUI_FILE_MANAGERS.iter().copied().find(|&fm| is_in_path(fm))
}

/// Opens a directory in the user's graphical file manager without hardcoding or terminal hijacking.
pub fn open_folder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", path.display()));
    }

    // 1. Check user's preferred default for inode/directory
    let default_app = Command::new("xdg-mime")
        .args(["query", "default", "inode/directory"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // 2. If user already has a valid graphical file manager configured, try gio / xdg-open
    if !default_app.is_empty() && !is_terminal_handler(&default_app) {
        if let Ok(mut child) = Command::new("gio").arg("open").arg(path).spawn() {
            let _ = child.wait();
            return Ok(());
        }
        if let Ok(mut child) = Command::new("xdg-open").arg(path).spawn() {
            let _ = child.wait();
            return Ok(());
        }
    }

    // 3. Fallback: discover any installed graphical file manager dynamically
    if let Some(fm) = find_installed_file_manager() {
        Command::new(fm)
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to launch {fm}: {e}"))?;
        return Ok(());
    }

    // 4. Last resort: xdg-open
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
    fn test_terminal_detection() {
        assert!(is_terminal_handler("kitty-open.desktop"));
        assert!(is_terminal_handler("Alacritty.desktop"));
        assert!(!is_terminal_handler("org.kde.dolphin.desktop"));
        assert!(!is_terminal_handler("org.gnome.Nautilus.desktop"));
        assert!(!is_terminal_handler("thunar.desktop"));
    }
}
