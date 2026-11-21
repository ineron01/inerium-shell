//! Theme mode detection (Dark vs Light).

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeMode::Dark)
    }

    pub fn is_light(&self) -> bool {
        matches!(self, ThemeMode::Light)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }
}

/// Detects the currently active theme mode.
/// First checks `~/.config/inerium/current.json`.
/// Falls back to GNOME/GSettings color-scheme.
pub fn detect_theme() -> ThemeMode {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let current_path = home.join(".config/inerium/current.json");
        if let Ok(content) = std::fs::read_to_string(current_path)
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(variant) = val.get("variant").and_then(|v| v.as_str())
        {
            if variant == "light" {
                return ThemeMode::Light;
            } else {
                return ThemeMode::Dark;
            }
        }
    }

    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        if s.contains("prefer-light") {
            return ThemeMode::Light;
        }
    }

    ThemeMode::Dark
}

/// Returns whether dark mode is currently active.
pub fn is_dark_theme() -> bool {
    detect_theme().is_dark()
}
