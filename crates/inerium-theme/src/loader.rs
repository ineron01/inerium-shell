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
    let mut map = BTreeMap::new();
    for root in get_extra_theme_roots() {
        scan_dir_for_themes(&root, &mut map);
    }
    // Themes from load_available_themes (system and user's ~/.config/inerium/themes)
    // take precedence over extra themes with identical IDs.
    map.extend(load_available_themes());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_theme_overrides_extra_theme() {
        let temp_base = std::env::temp_dir().join(format!("inerium-test-theme-{}", std::process::id()));
        let extra_dir = temp_base.join("extra");
        let user_dir = temp_base.join("user");
        let _ = fs::create_dir_all(&extra_dir);
        let _ = fs::create_dir_all(&user_dir);

        let mut extra_cfg = parse_theme_toml(EMBEDDED_DRAGON).unwrap();
        extra_cfg.theme.id = "custom-override".to_string();
        extra_cfg.theme.name = "Stock Extra Theme".to_string();

        let mut user_cfg = extra_cfg.clone();
        user_cfg.theme.name = "User Overridden Theme".to_string();

        fs::write(extra_dir.join("custom-override.toml"), toml::to_string(&extra_cfg).unwrap()).unwrap();
        fs::write(user_dir.join("custom-override.toml"), toml::to_string(&user_cfg).unwrap()).unwrap();

        let mut map = BTreeMap::new();
        // 1. Scan extra themes first
        scan_dir_for_themes(&extra_dir, &mut map);
        assert_eq!(
            map.get("custom-override").map(|c| c.theme.name.as_str()),
            Some("Stock Extra Theme")
        );

        // 2. Scan user themes second (representing load_available_themes extending over extra)
        scan_dir_for_themes(&user_dir, &mut map);
        assert_eq!(
            map.get("custom-override").map(|c| c.theme.name.as_str()),
            Some("User Overridden Theme")
        );

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_all_system_and_extra_themes_validity() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let themes_dir = repo_root.join("themes");
        let extra_dir = themes_dir.join("extra");

        let mut theme_paths = vec![
            themes_dir.join("kanagawa-dragon.toml"),
            themes_dir.join("inerium-light.toml"),
            themes_dir.join("template.toml.example"),
        ];

        for entry in fs::read_dir(&extra_dir).expect("read extra dir").flatten() {
            if entry.path().extension().is_some_and(|e| e == "toml") {
                theme_paths.push(entry.path());
            }
        }

        assert!(theme_paths.len() >= 10, "Expected at least 10 themes to validate");

        for path in theme_paths {
            let filename = path.file_name().unwrap().to_string_lossy();
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read theme file {}: {e}", path.display()));

            let theme = parse_theme_toml(&content)
                .unwrap_or_else(|e| panic!("Failed to parse theme {}: {e}", path.display()));

            // 1. Meta validation
            assert!(!theme.theme.id.trim().is_empty(), "Empty id in {filename}");
            assert!(!theme.theme.name.trim().is_empty(), "Empty name in {filename}");
            assert!(!theme.theme.author.trim().is_empty(), "Empty author in {filename}");
            assert!(
                theme.theme.variant == "dark" || theme.theme.variant == "light",
                "Variant must be dark or light in {filename}, got: {}",
                theme.theme.variant
            );

            // 2. Typography validation: standard font size must be 11.0
            let typo = theme.typography.as_ref().unwrap_or_else(|| panic!("Missing typography in {filename}"));
            assert_eq!(
                typo.get_size(),
                11.0,
                "Font size in {filename} must be 11.0, got: {}",
                typo.get_size()
            );
            assert!(!typo.get_ui_font().is_empty(), "Empty UI font in {filename}");
            assert!(!typo.get_mono_font().is_empty(), "Empty Mono font in {filename}");

            // 3. Palette validation: verify required colors are not empty
            assert!(!theme.palette.bg_canvas.is_empty(), "Empty bg_canvas in {filename}");
            assert!(!theme.palette.bg_surface.is_empty(), "Empty bg_surface in {filename}");
            assert!(!theme.palette.border_subtle.is_empty(), "Empty border_subtle in {filename}");
            assert!(!theme.palette.border_active.is_empty(), "Empty border_active in {filename}");
            assert!(!theme.palette.text_primary.is_empty(), "Empty text_primary in {filename}");
            assert!(!theme.palette.text_heading.is_empty(), "Empty text_heading in {filename}");
            assert!(!theme.palette.accent_green.is_empty(), "Empty accent_green in {filename}");
            assert!(!theme.palette.accent_blue.is_empty(), "Empty accent_blue in {filename}");
            assert!(!theme.palette.accent_amber.is_empty(), "Empty accent_amber in {filename}");
            assert!(!theme.palette.accent_red.is_empty(), "Empty accent_red in {filename}");
            assert!(!theme.palette.accent_aqua.is_empty(), "Empty accent_aqua in {filename}");
            assert!(!theme.palette.accent_purple.is_empty(), "Empty accent_purple in {filename}");

            // 4. Emitter validation: make sure emitters do not panic
            let _adwaita = crate::emit::emit_libadwaita_css(&theme);
            let _waybar = crate::emit::emit_waybar_css(&theme);
            let _rofi = crate::emit::emit_rofi_rasi(&theme);
            let kitty = crate::emit::emit_kitty_conf(&theme);
            let _mako = crate::emit::emit_mako_config(&theme);
            let _hypr = crate::emit::emit_hyprland_lua(&theme);
            let _qtct = crate::emit::emit_qtct_colors(&theme);
            let _kde = crate::emit::emit_kde_colors(&theme);

            assert!(kitty.contains("font_size 11.0"), "Kitty config for {filename} should emit font_size 11.0");
        }
    }
}
