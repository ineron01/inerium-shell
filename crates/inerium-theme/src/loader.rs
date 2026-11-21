//! Theme discovery, directory scanning, and TOML deserialization.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::{ThemeConfig, ThemeMeta};

pub const EMBEDDED_DRAGON: &str = include_str!("../../../themes/kanagawa-dragon.toml");
pub const EMBEDDED_INERIUM_LIGHT: &str = include_str!("../../../themes/inerium-light.toml");

/// Deserializes a theme from TOML string.
pub fn parse_theme_toml(content: &str) -> Result<ThemeConfig, String> {
    toml::from_str::<ThemeConfig>(content).map_err(|e| format!("Failed to parse theme TOML: {e}"))
}

pub fn get_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn scan_dir_for_themes(dir: &Path, map: &mut BTreeMap<String, ThemeConfig>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "toml")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(c) = parse_theme_toml(&content)
            {
                map.insert(c.theme.id.clone(), c);
            }
        }
    }
}

pub fn load_available_themes() -> BTreeMap<String, ThemeConfig> {
    let mut map = BTreeMap::new();

    // 1. Embedded fallbacks (Default Dark & Default Light)
    if let Ok(c) = parse_theme_toml(EMBEDDED_DRAGON) {
        map.insert(c.theme.id.clone(), c);
    }
    if let Ok(c) = parse_theme_toml(EMBEDDED_INERIUM_LIGHT) {
        map.insert(c.theme.id.clone(), c);
    }

    let home = get_home();
    let mut theme_roots = vec![
        PathBuf::from("/usr/share/inerium-shell/themes"),
        PathBuf::from("/usr/share/inerium/themes"),
        home.join(".local/share/inerium/themes"),
    ];

    if let Ok(dir) = std::env::var("INERIUM_SHELL_DIR")
        && !dir.is_empty()
    {
        theme_roots.push(PathBuf::from(dir).join("themes"));
    }

    theme_roots.push(home.join(".config/inerium/themes"));

    for root in &theme_roots {
        scan_dir_for_themes(root, &mut map);
    }

    map
}

pub fn get_extra_theme_roots() -> Vec<PathBuf> {
    let home = get_home();
    let mut roots = vec![
        PathBuf::from("/usr/share/inerium-shell/themes/extra"),
        PathBuf::from("/usr/share/inerium/themes/extra"),
        home.join(".local/share/inerium/themes/extra"),
    ];
    if let Ok(dir) = std::env::var("INERIUM_SHELL_DIR")
        && !dir.is_empty()
    {
        roots.push(PathBuf::from(dir).join("themes/extra"));
    }
    roots
}

pub fn load_all_themes() -> BTreeMap<String, ThemeConfig> {
    let mut map = load_available_themes();
    for root in get_extra_theme_roots() {
        scan_dir_for_themes(&root, &mut map);
    }
    map
}

pub fn install_extra_theme(id: &str) -> Result<ThemeConfig, String> {
    let home = get_home();
    let dest_dir = home.join(".config/inerium/themes");
    let dest_file = dest_dir.join(format!("{id}.toml"));

    if dest_file.exists()
        && let Ok(content) = fs::read_to_string(&dest_file)
        && let Ok(cfg) = parse_theme_toml(&content)
    {
        return Ok(cfg);
    }

    for root in get_extra_theme_roots() {
        let src = root.join(format!("{id}.toml"));
        if src.exists() {
            let _ = fs::create_dir_all(&dest_dir);
            if let Err(e) = fs::copy(&src, &dest_file) {
                return Err(format!("{}: {e}", inerium_i18n::tr("Failed to copy theme file", "Не удалось скопировать файл темы")));
            }
            if let Ok(content) = fs::read_to_string(&dest_file)
                && let Ok(cfg) = parse_theme_toml(&content)
            {
                return Ok(cfg);
            }
        }
    }

    let err_template = inerium_i18n::tr(
        "Theme '{}' not found in extra palettes.",
        "Тема '{}' не найдена в дополнительных палитрах."
    );
    Err(err_template.replace("{}", id))
}

pub fn resolve_theme_alias(input_id: &str) -> &str {
    match input_id {
        "dark" | "dragon" => "kanagawa-dragon",
        "light" => "inerium-light",
        "lotus" => "kanagawa-lotus",
        "wave" => "kanagawa-wave",
        "mocha" => "catppuccin-mocha",
        "latte" => "catppuccin-latte",
        "tokyo" | "tokyonight" => "tokyo-night",
        "gruvbox" => "gruvbox-dark",
        other => other,
    }
}

pub fn get_current_theme_id() -> String {
    let home = get_home();
    let cur_json = home.join(".config/inerium/current.json");
    if let Ok(content) = fs::read_to_string(&cur_json)
        && let Ok(meta) = serde_json::from_str::<ThemeMeta>(&content)
    {
        return meta.id;
    }

    // Fallback: gsettings color-scheme
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        if s.contains("prefer-light") {
            return "inerium-light".to_string();
        }
    }

    "kanagawa-dragon".to_string()
}

pub fn load_theme_by_id(id: &str) -> Result<ThemeConfig, String> {
    let resolved = resolve_theme_alias(id);
    let available = load_available_themes();
    if let Some(cfg) = available.get(resolved) {
        return Ok(cfg.clone());
    }
    install_extra_theme(resolved)
}

pub fn get_fallback_theme(dark: bool) -> ThemeConfig {
    let id = if dark { "kanagawa-dragon" } else { "inerium-light" };
    load_theme_by_id(id).unwrap_or_else(|_| {
        let raw = if dark { EMBEDDED_DRAGON } else { EMBEDDED_INERIUM_LIGHT };
        parse_theme_toml(raw).expect("Embedded fallback theme must be valid TOML")
    })
}
