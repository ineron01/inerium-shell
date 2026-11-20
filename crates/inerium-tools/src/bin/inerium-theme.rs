use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const EMBEDDED_DRAGON: &str = include_str!("../../../../themes/kanagawa-dragon.toml");
pub const EMBEDDED_INERIUM_LIGHT: &str = include_str!("../../../../themes/inerium-light.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub theme: ThemeMeta,
    pub palette: Palette,
    #[serde(default)]
    pub typography: Option<Typography>,
    #[serde(default)]
    pub display: Option<DisplayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Typography {
    #[serde(default)]
    pub font_ui: Option<String>,
    #[serde(default)]
    pub font_mono: Option<String>,
    #[serde(default)]
    pub font_size: Option<f32>,
}

impl Typography {
    pub fn get_ui_font(&self) -> &str {
        self.font_ui.as_deref().unwrap_or("JetBrainsMono Nerd Font")
    }

    pub fn get_mono_font(&self) -> &str {
        self.font_mono.as_deref().unwrap_or("JetBrainsMono Nerd Font")
    }

    pub fn get_size(&self) -> f32 {
        self.font_size.unwrap_or(11.0)
    }

    pub fn to_css_font_family(&self) -> String {
        let ui = self.get_ui_font();
        if ui.contains("Nerd Font") {
            format!("\"{}\", monospace", ui)
        } else {
            format!("\"{}\", \"JetBrainsMono Nerd Font\", sans-serif, monospace", ui)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayConfig {
    pub temperature: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub author: String,
    pub upstream: Option<String>,
    pub variant: String, // "dark" | "light"
    pub license: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg_canvas: String,
    pub bg_surface: String,
    #[serde(default)]
    pub bg_surface_alt: Option<String>,
    #[serde(default)]
    pub bg_surface_hover: Option<String>,
    #[serde(default)]
    pub bg_surface_active: Option<String>,

    pub border_subtle: String,
    pub border_active: String,
    #[serde(default)]
    pub border_active_secondary: Option<String>,

    pub text_primary: String,
    pub text_heading: String,
    #[serde(default)]
    pub text_muted: Option<String>,
    #[serde(default)]
    pub text_subtle: Option<String>,
    #[serde(default)]
    pub text_inverted: Option<String>,

    pub accent_green: String,
    #[serde(default)]
    pub accent_green_bg: Option<String>,
    #[serde(default)]
    pub accent_green_border: Option<String>,

    pub accent_blue: String,
    #[serde(default)]
    pub accent_blue_bg: Option<String>,
    #[serde(default)]
    pub accent_blue_border: Option<String>,

    pub accent_amber: String,
    #[serde(default)]
    pub accent_amber_bg: Option<String>,
    #[serde(default)]
    pub accent_amber_border: Option<String>,

    pub accent_red: String,
    #[serde(default)]
    pub accent_red_bg: Option<String>,
    #[serde(default)]
    pub accent_red_border: Option<String>,

    pub accent_aqua: String,
    #[serde(default)]
    pub accent_aqua_bg: Option<String>,
    #[serde(default)]
    pub accent_aqua_border: Option<String>,

    pub accent_purple: String,
    #[serde(default)]
    pub accent_purple_bg: Option<String>,
    #[serde(default)]
    pub accent_purple_border: Option<String>,

    #[serde(default)]
    pub accent_primary: Option<String>,
    #[serde(default)]
    pub accent_orange: Option<String>,

    #[serde(default)]
    pub slider_fill: Option<String>,
    #[serde(default)]
    pub slider_fill_secondary: Option<String>,
    #[serde(default)]
    pub slider_trough: Option<String>,
    #[serde(default)]
    pub slider_thumb: Option<String>,

    #[serde(default)]
    pub shadow: Option<String>,
    #[serde(default)]
    pub modal_backdrop: Option<String>,
}

impl Palette {
    pub fn get_primary(&self) -> &str {
        self.accent_primary.as_deref().unwrap_or(&self.accent_blue)
    }

    pub fn get_orange(&self) -> &str {
        self.accent_orange.as_deref().unwrap_or(&self.accent_amber)
    }

    pub fn get_primary_bg(&self) -> String {
        let rgba = Rgba::parse(self.get_primary());
        format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
    }

    pub fn get_primary_border(&self) -> String {
        let rgba = Rgba::parse(self.get_primary());
        format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
    }

    pub fn get_slider_fill(&self) -> &str {
        self.slider_fill.as_deref().unwrap_or_else(|| self.get_primary())
    }

    pub fn get_slider_fill_secondary(&self) -> &str {
        self.slider_fill_secondary.as_deref().unwrap_or(&self.accent_amber)
    }

    pub fn get_slider_trough(&self) -> String {
        self.slider_trough.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).with_alpha(26).to_css_rgba()
        })
    }

    pub fn get_slider_thumb(&self) -> &str {
        self.slider_thumb.as_deref().unwrap_or(&self.text_heading)
    }

    pub fn get_border_active_secondary(&self) -> &str {
        self.border_active_secondary.as_deref().unwrap_or(&self.border_active)
    }

    pub fn get_surface_alt(&self) -> String {
        self.bg_surface_alt.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).darken(0.06).to_hex_rgb()
        })
    }

    pub fn get_surface_hover(&self) -> String {
        self.bg_surface_hover.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).lighten(0.08).to_hex_rgb()
        })
    }

    pub fn get_surface_active(&self) -> String {
        self.bg_surface_active.clone().unwrap_or_else(|| {
            Rgba::parse(&self.bg_surface).lighten(0.15).to_hex_rgb()
        })
    }

    pub fn get_text_muted(&self) -> String {
        self.text_muted.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).darken(0.40).to_hex_rgb()
        })
    }

    pub fn get_text_subtle(&self) -> String {
        self.text_subtle.clone().unwrap_or_else(|| {
            Rgba::parse(&self.text_primary).darken(0.20).to_hex_rgb()
        })
    }

    pub fn get_text_inverted(&self) -> &str {
        self.text_inverted.as_deref().unwrap_or(&self.bg_canvas)
    }

    pub fn get_green_bg(&self) -> String {
        self.accent_green_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_green);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_green_border(&self) -> String {
        self.accent_green_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_green);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_blue_bg(&self) -> String {
        self.accent_blue_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_blue);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_blue_border(&self) -> String {
        self.accent_blue_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_blue);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_amber_bg(&self) -> String {
        self.accent_amber_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_amber);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_amber_border(&self) -> String {
        self.accent_amber_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_amber);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_red_bg(&self) -> String {
        self.accent_red_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_red);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_red_border(&self) -> String {
        self.accent_red_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_red);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_aqua_bg(&self) -> String {
        self.accent_aqua_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_aqua);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_aqua_border(&self) -> String {
        self.accent_aqua_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_aqua);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_purple_bg(&self) -> String {
        self.accent_purple_bg.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_purple);
            format!("rgba({}, {}, {}, 0.16)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_purple_border(&self) -> String {
        self.accent_purple_border.clone().unwrap_or_else(|| {
            let rgba = Rgba::parse(&self.accent_purple);
            format!("rgba({}, {}, {}, 0.35)", rgba.0, rgba.1, rgba.2)
        })
    }

    pub fn get_shadow(&self) -> &str {
        self.shadow.as_deref().unwrap_or("rgba(0, 0, 0, 0.85)")
    }

    pub fn get_modal_backdrop(&self) -> &str {
        self.modal_backdrop.as_deref().unwrap_or("rgba(5, 5, 7, 0.75)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Rgba(0, 0, 0, 255);
        }

        // Named colors
        if s.eq_ignore_ascii_case("transparent") {
            return Rgba(0, 0, 0, 0);
        } else if s.eq_ignore_ascii_case("black") {
            return Rgba(0, 0, 0, 255);
        } else if s.eq_ignore_ascii_case("white") {
            return Rgba(255, 255, 255, 255);
        }

        // Hex formats (with or without '#')
        let hex_str = s.strip_prefix('#').unwrap_or(s);
        let is_pure_hex = hex_str.chars().all(|c| c.is_ascii_hexdigit());

        if is_pure_hex {
            match hex_str.len() {
                3 => {
                    let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap_or(0);
                    return Rgba(r * 17, g * 17, b * 17, 255);
                }
                4 => {
                    let r = u8::from_str_radix(&hex_str[0..1], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[1..2], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[2..3], 16).unwrap_or(0);
                    let a = u8::from_str_radix(&hex_str[3..4], 16).unwrap_or(15);
                    return Rgba(r * 17, g * 17, b * 17, a * 17);
                }
                6 => {
                    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
                    return Rgba(r, g, b, 255);
                }
                8 => {
                    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap_or(0);
                    let a = u8::from_str_radix(&hex_str[6..8], 16).unwrap_or(255);
                    return Rgba(r, g, b, a);
                }
                _ => {}
            }
        }

        // Functional rgb(...) and rgba(...) handling
        if s.starts_with("rgb") {
            let inner = s
                .strip_prefix("rgba(")
                .or_else(|| s.strip_prefix("rgb("))
                .and_then(|str_val| str_val.strip_suffix(')'));

            if let Some(inner) = inner {
                let normalized = inner.replace([',', '/'], " ");
                let parts: Vec<&str> = normalized.split_whitespace().collect();

                if parts.len() >= 3 {
                    let r = parts[0].parse::<u8>().unwrap_or(0);
                    let g = parts[1].parse::<u8>().unwrap_or(0);
                    let b = parts[2].parse::<u8>().unwrap_or(0);
                    let a = if parts.len() >= 4 {
                        let a_str = parts[3].trim_end_matches('%');
                        if let Ok(alpha_float) = a_str.parse::<f32>() {
                            if parts[3].ends_with('%') {
                                (alpha_float.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8
                            } else if alpha_float <= 1.0 {
                                (alpha_float.clamp(0.0, 1.0) * 255.0).round() as u8
                            } else {
                                alpha_float.clamp(0.0, 255.0).round() as u8
                            }
                        } else {
                            parts[3].parse::<u8>().unwrap_or(255)
                        }
                    } else {
                        255
                    };
                    return Rgba(r, g, b, a);
                }
            }
        }

        Rgba(0, 0, 0, 255)
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        Rgba(self.0, self.1, self.2, alpha)
    }

    pub fn with_alpha_f32(&self, alpha: f32) -> Self {
        Rgba(self.0, self.1, self.2, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    pub fn to_css_rgba(&self) -> String {
        if self.3 == 255 {
            self.to_hex_rgb()
        } else {
            let a = (self.3 as f32 / 255.0 * 100.0).round() / 100.0;
            format!("rgba({}, {}, {}, {:.2})", self.0, self.1, self.2, a)
        }
    }

    pub fn to_hex_rgb(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    pub fn to_hex_rgba(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.0, self.1, self.2, self.3)
    }

    pub fn to_hypr(&self) -> String {
        format!("rgba({:02x}{:02x}{:02x}{:02x})", self.0, self.1, self.2, self.3)
    }

    pub fn to_kde_rgb(&self) -> String {
        format!("{},{},{}", self.0, self.1, self.2)
    }

    pub fn lighten(&self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let r = (self.0 as f32 + (255.0 - self.0 as f32) * f).round().clamp(0.0, 255.0) as u8;
        let g = (self.1 as f32 + (255.0 - self.1 as f32) * f).round().clamp(0.0, 255.0) as u8;
        let b = (self.2 as f32 + (255.0 - self.2 as f32) * f).round().clamp(0.0, 255.0) as u8;
        Rgba(r, g, b, self.3)
    }

    pub fn darken(&self, factor: f32) -> Self {
        let f = (1.0 - factor.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let r = (self.0 as f32 * f).round().clamp(0.0, 255.0) as u8;
        let g = (self.1 as f32 * f).round().clamp(0.0, 255.0) as u8;
        let b = (self.2 as f32 * f).round().clamp(0.0, 255.0) as u8;
        Rgba(r, g, b, self.3)
    }
}

fn get_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn scan_dir_for_themes(dir: &Path, map: &mut BTreeMap<String, ThemeConfig>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "toml")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(c) = toml::from_str::<ThemeConfig>(&content)
            {
                map.insert(c.theme.id.clone(), c);
            }
        }
    }
}

pub fn load_available_themes() -> BTreeMap<String, ThemeConfig> {
    let mut map = BTreeMap::new();

    // 1. Embedded fallbacks (Default Dark & Default Light)
    if let Ok(c) = toml::from_str::<ThemeConfig>(EMBEDDED_DRAGON) {
        map.insert(c.theme.id.clone(), c);
    }
    if let Ok(c) = toml::from_str::<ThemeConfig>(EMBEDDED_INERIUM_LIGHT) {
        map.insert(c.theme.id.clone(), c);
    }

    let home = get_home();
    let theme_roots = [
        home.join("Repos/inerium-shell/themes"),
        home.join(".config/inerium/themes"),
    ];

    for root in &theme_roots {
        scan_dir_for_themes(root, &mut map);
    }

    map
}

pub fn get_extra_theme_roots() -> Vec<PathBuf> {
    let home = get_home();
    vec![
        home.join("Repos/inerium-shell/themes/extra"),
        home.join(".local/share/inerium/themes/extra"),
        PathBuf::from("/usr/share/inerium/themes/extra"),
        PathBuf::from("/usr/share/inerium-shell/themes/extra"),
    ]
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
        && let Ok(cfg) = toml::from_str::<ThemeConfig>(&content)
    {
        return Ok(cfg);
    }

    for root in get_extra_theme_roots() {
        let src = root.join(format!("{id}.toml"));
        if src.exists() {
            let _ = fs::create_dir_all(&dest_dir);
            if let Err(e) = fs::copy(&src, &dest_file) {
                return Err(format!("Failed to copy theme file: {e}"));
            }
            if let Ok(content) = fs::read_to_string(&dest_file)
                && let Ok(cfg) = toml::from_str::<ThemeConfig>(&content)
            {
                return Ok(cfg);
            }
        }
    }

    Err(format!("Тема '{id}' не найдена в дополнительных палитрах."))
}

fn resolve_theme_alias(input_id: &str) -> &str {
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

// ============================================================================
// CONFIG EMITTERS
// ============================================================================

fn emit_hyprland_lua(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    format!(
r#"-- Generated by inerium-theme for {} (author: {})
-- Upstream: {}

INERIUM_BASE                    = "{}"
INERIUM_SURFACE                 = "{}"
INERIUM_BORDER                  = "{}"
INERIUM_BORDER_ACTIVE           = "{}"
INERIUM_BORDER_ACTIVE_SECONDARY = "{}"
INERIUM_TEXT                    = "{}"
INERIUM_MUTED                   = "{}"

INERIUM_GREEN                   = "{}"
INERIUM_BLUE                    = "{}"
INERIUM_AMBER                   = "{}"
INERIUM_RED                     = "{}"
INERIUM_PURPLE                  = "{}"
INERIUM_ACCENT_PRIMARY          = "{}"
INERIUM_SHADOW                  = "{}"

INERIUM_DRAGON_GRAY             = "{}"
INERIUM_DRAGON_BLUE             = "{}"
"#,
        theme.theme.name,
        theme.theme.author,
        theme.theme.upstream.as_deref().unwrap_or("N/A"),
        Rgba::parse(&p.bg_canvas).to_hypr(),
        Rgba::parse(&p.bg_surface).to_hypr(),
        Rgba::parse(&p.border_subtle).to_hypr(),
        Rgba::parse(&p.border_active).to_hypr(),
        Rgba::parse(p.get_border_active_secondary()).to_hypr(),
        Rgba::parse(&p.text_primary).to_hypr(),
        Rgba::parse(&p.get_text_muted()).to_hypr(),
        Rgba::parse(&p.accent_green).to_hypr(),
        Rgba::parse(&p.accent_blue).to_hypr(),
        Rgba::parse(&p.accent_amber).to_hypr(),
        Rgba::parse(&p.accent_red).to_hypr(),
        Rgba::parse(&p.accent_purple).to_hypr(),
        Rgba::parse(p.get_primary()).to_hypr(),
        Rgba::parse(p.get_shadow()).to_hypr(),
        Rgba::parse(&p.border_subtle).to_hypr(),
        Rgba::parse(p.get_primary()).to_hypr(),
    )
}

fn emit_waybar_css(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let typo_css = if let Some(t) = &theme.typography {
        let waybar_size = (t.get_size() * 1.14).clamp(11.0, 16.0);
        format!(
            "\n* {{\n    font-family: {};\n    font-size: {:.1}px;\n}}\n",
            t.to_css_font_family(),
            waybar_size
        )
    } else {
        String::new()
    };
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
@define-color bg                      {};
@define-color surface                 {};
@define-color hover                   {};
@define-color line                    {};
@define-color fg                      {};
@define-color fg-bright               {};
@define-color muted                   {};
@define-color subtle                  {};
@define-color green                   {};
@define-color blue                    {};
@define-color aqua                    {};
@define-color yellow                  {};
@define-color orange                  {};
@define-color red                     {};
@define-color violet                  {};
@define-color primary                 {};
@define-color slider_fill             {};
@define-color slider_fill_secondary   {};
{typo_css}"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.get_surface_hover(),
        p.border_subtle,
        p.text_primary,
        p.text_heading,
        p.get_text_muted(),
        p.get_text_subtle(),
        p.accent_green,
        p.accent_blue,
        p.accent_aqua,
        p.accent_amber,
        p.get_orange(),
        p.accent_red,
        p.accent_purple,
        p.get_primary(),
        p.get_slider_fill(),
        p.get_slider_fill_secondary(),
    )
}

fn emit_rofi_rasi(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let font_str = if let Some(t) = &theme.typography {
        format!("    font:         \"{} {:.1}\";\n", t.get_ui_font(), t.get_size())
    } else {
        String::new()
    };
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
* {{
{font_str}    bg:           {};
    surface:      {};
    surface-hi:   {};
    line:         {};
    line-soft:    {};
    fg:           {};
    fg-bright:    {};
    muted:        {};
    green:        {};
    green-soft:   {};
    green-line:   {};
    red:          {};
    primary:      {};
    primary-soft: {};
    primary-line: {};
}}
"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.get_surface_hover(),
        p.border_active,
        p.border_subtle,
        p.text_primary,
        p.text_heading,
        p.get_text_muted(),
        p.accent_green,
        p.get_green_bg(),
        p.get_green_border(),
        p.accent_red,
        p.get_primary(),
        p.get_primary_bg(),
        p.get_primary_border(),
    )
}

fn emit_kitty_conf(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let bg = Rgba::parse(&p.bg_canvas).to_hex_rgb();
    let fg = Rgba::parse(&p.text_primary).to_hex_rgb();
    let sel_bg = Rgba::parse(&p.get_surface_hover()).to_hex_rgb();
    let green = Rgba::parse(&p.accent_green).to_hex_rgb();
    let blue = Rgba::parse(&p.accent_blue).to_hex_rgb();
    let red = Rgba::parse(&p.accent_red).to_hex_rgb();
    let amber = Rgba::parse(&p.accent_amber).to_hex_rgb();
    let aqua = Rgba::parse(&p.accent_aqua).to_hex_rgb();
    let purple = Rgba::parse(&p.accent_purple).to_hex_rgb();
    let muted = Rgba::parse(&p.get_text_muted()).to_hex_rgb();
    let heading = Rgba::parse(&p.text_heading).to_hex_rgb();
    let primary = Rgba::parse(p.get_primary()).to_hex_rgb();

    let bright_red = Rgba::parse(&p.accent_red).lighten(0.18).to_hex_rgb();
    let bright_green = Rgba::parse(&p.accent_green).lighten(0.18).to_hex_rgb();
    let bright_amber = Rgba::parse(&p.accent_amber).lighten(0.18).to_hex_rgb();
    let bright_blue = Rgba::parse(&p.accent_blue).lighten(0.18).to_hex_rgb();
    let bright_purple = Rgba::parse(&p.accent_purple).lighten(0.18).to_hex_rgb();
    let bright_aqua = Rgba::parse(&p.accent_aqua).lighten(0.18).to_hex_rgb();

    let font_directives = if let Some(t) = &theme.typography {
        format!(
            "font_family {}\nfont_size {:.1}\n\n",
            t.get_mono_font(),
            t.get_size()
        )
    } else {
        String::new()
    };

    format!(
r#"# Generated by inerium-theme for {} (author: {})

{font_directives}foreground            {}
background            {}
selection_foreground  {}
selection_background  {}

cursor                {}
cursor_text_color     {}

url_color             {}

# Tabs
active_tab_foreground   {}
active_tab_background   {}
inactive_tab_foreground {}
inactive_tab_background {}

# Borders
active_border_color   {}
inactive_border_color {}

# Normal colors
color0 {}
color1 {}
color2 {}
color3 {}
color4 {}
color5 {}
color6 {}
color7 {}

# Bright colors
color8 {}
color9 {}
color10 {}
color11 {}
color12 {}
color13 {}
color14 {}
color15 {}
"#,
        theme.theme.name,
        theme.theme.author,
        fg,
        bg,
        fg,
        sel_bg,
        fg,
        bg,
        primary,
        bg,
        primary,
        muted,
        sel_bg,
        primary,
        sel_bg,
        bg,
        red,
        green,
        amber,
        blue,
        purple,
        aqua,
        fg,
        muted,
        bright_red,
        bright_green,
        bright_amber,
        bright_blue,
        bright_purple,
        bright_aqua,
        heading,
    )
}

fn emit_mako_config(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let bg = Rgba::parse(&p.bg_canvas).to_hex_rgba();
    let text = Rgba::parse(&p.text_primary).to_hex_rgb();
    let text_subtle = Rgba::parse(&p.get_text_subtle()).to_hex_rgb();
    let text_muted = Rgba::parse(&p.get_text_muted()).to_hex_rgb();
    let border = Rgba::parse(&p.border_subtle).to_hex_rgba();
    let red = Rgba::parse(&p.accent_red).to_hex_rgb();
    let primary_progress = Rgba::parse(&p.get_primary_bg()).to_hex_rgba();

    let font_ui = theme.typography.as_ref().map(|t| t.get_ui_font()).unwrap_or("JetBrainsMono Nerd Font");
    let font_size = theme.typography.as_ref().map(|t| t.get_size()).unwrap_or(10.0);

    format!(
r#"# Generated by inerium-theme for {} (author: {})
font={} {:.0}
width=360
background-color={}
text-color={}
border-color={}
border-size=1
border-radius=16
padding=12,14
margin=8,8,0,8
outer-margin=4,0,0,0
icon-border-radius=10
max-icon-size=40
icons=1
actions=1
progress-color=over {}
format=<b>%s</b>\n<span color="{}">%b</span>
default-timeout=5000
layer=overlay
anchor=top-right

history=0
max-history=0

[urgency=low]
border-color={}
format=<span color="{}"><b>%s</b></span>\n<span color="{}">%b</span>
default-timeout=3000

[urgency=normal]
default-timeout=5000

[urgency=high]
border-color={}
format=<span color="{}"><b>%s</b></span>\n%b
default-timeout=0

[mode=dnd]
invisible=1

[mode=do-not-disturb]
invisible=1
"#,
        theme.theme.name,
        theme.theme.author,
        font_ui,
        font_size,
        bg,
        text,
        border,
        primary_progress,
        text_subtle,
        border,
        text_subtle,
        text_muted,
        red,
        red,
    )
}

fn emit_gtk_vars(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let typo_css = if let Some(t) = &theme.typography {
        format!(
            "\n.popup-container, .cc, window.popup-window {{\n    font-family: {};\n}}\n",
            t.to_css_font_family()
        )
    } else {
        String::new()
    };
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
@define-color cc_bg                   {};
@define-color cc_surface              {};
@define-color cc_surface_hover        {};
@define-color cc_surface_active       {};
@define-color cc_line                 {};
@define-color cc_line_light           {};
@define-color cc_fg                   {};
@define-color cc_fg_bright            {};
@define-color cc_fg_muted             {};
@define-color cc_fg_subtle            {};
@define-color cc_fg_on_accent         {};

@define-color cc_green                {};
@define-color cc_green_bg             {};
@define-color cc_green_border         {};

@define-color cc_blue                 {};
@define-color cc_blue_bg              {};
@define-color cc_blue_border          {};

@define-color cc_amber                {};
@define-color cc_amber_bg             {};
@define-color cc_amber_border         {};

@define-color cc_red                  {};
@define-color cc_red_bg               {};
@define-color cc_red_border           {};

@define-color cc_aqua                 {};
@define-color cc_aqua_bg              {};
@define-color cc_aqua_border          {};

@define-color cc_violet               {};
@define-color cc_violet_bg            {};
@define-color cc_violet_border        {};

@define-color cc_shadow               {};
@define-color cc_modal_backdrop       {};
@define-color cc_slider_trough        {};
@define-color cc_slider_thumb         {};
@define-color cc_orange               {};
@define-color cc_primary              {};
@define-color cc_primary_bg           {};
@define-color cc_primary_border       {};
@define-color cc_slider_fill          {};
@define-color cc_slider_fill_secondary {};
{typo_css}"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.get_surface_hover(),
        p.get_surface_active(),
        p.border_subtle,
        p.border_active,
        p.text_primary,
        p.text_heading,
        p.get_text_muted(),
        p.get_text_subtle(),
        p.get_text_inverted(),
        p.accent_green,
        p.get_green_bg(),
        p.get_green_border(),
        p.accent_blue,
        p.get_blue_bg(),
        p.get_blue_border(),
        p.accent_amber,
        p.get_amber_bg(),
        p.get_amber_border(),
        p.accent_red,
        p.get_red_bg(),
        p.get_red_border(),
        p.accent_aqua,
        p.get_aqua_bg(),
        p.get_aqua_border(),
        p.accent_purple,
        p.get_purple_bg(),
        p.get_purple_border(),
        p.get_shadow(),
        p.get_modal_backdrop(),
        p.get_slider_trough(),
        p.get_slider_thumb(),
        p.get_orange(),
        p.get_primary(),
        p.get_primary_bg(),
        p.get_primary_border(),
        p.get_slider_fill(),
        p.get_slider_fill_secondary(),
    )
}

fn emit_libadwaita_css(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let primary = p.get_primary();
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
@define-color window_bg_color {};
@define-color window_fg_color {};
@define-color view_bg_color {};
@define-color view_fg_color {};
@define-color headerbar_bg_color {};
@define-color headerbar_fg_color {};
@define-color headerbar_backdrop_color {};
@define-color card_bg_color {};
@define-color card_fg_color {};
@define-color popover_bg_color {};
@define-color popover_fg_color {};
@define-color dialog_bg_color {};
@define-color dialog_fg_color {};
@define-color borders {};
@define-color accent_color {};
@define-color accent_bg_color {};
@define-color accent_fg_color {};
@define-color destructive_color {};
@define-color destructive_bg_color {};
@define-color destructive_fg_color {};
"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.text_primary,
        p.bg_surface,
        p.text_primary,
        p.bg_surface,
        p.text_primary,
        p.bg_canvas,
        p.bg_surface,
        p.text_primary,
        p.bg_surface,
        p.text_primary,
        p.bg_surface,
        p.text_primary,
        p.border_subtle,
        primary,
        primary,
        p.get_text_inverted(),
        p.accent_red,
        p.accent_red,
        p.text_primary,
    )
}

fn apply_display_temperature(temp_kelvin: u32) {
    let _ = Command::new("pkill")
        .args(["-x", "hyprsunset"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if temp_kelvin < 6500 {
        let _ = Command::new("setsid")
            .args(["hyprsunset", "-t", &temp_kelvin.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

fn run_theme_hooks(theme: &ThemeConfig) {
    let home = get_home();
    let hooks_dir = home.join(".config/inerium/hooks.d");
    if let Ok(entries) = fs::read_dir(&hooks_dir) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if path.is_file() {
                // Ignore backup/disabled files and markdown
                if let Some(ext) = path.extension().and_then(|x| x.to_str())
                    && matches!(ext, "bak" | "old" | "disabled" | "md" | "txt")
                {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && (name.starts_with('.') || name.ends_with('~'))
                {
                    continue;
                }
                // Check executable bit (owner, group, or other)
                if let Ok(meta) = fs::metadata(&path)
                    && meta.permissions().mode() & 0o111 == 0
                {
                    continue;
                }
                let _ = Command::new(&path)
                    .args([
                        &theme.theme.variant,
                        &theme.theme.id,
                        &theme.theme.name,
                        &theme.theme.author,
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
        }
    }
}

fn emit_kde_colors(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let bg_canvas = Rgba::parse(&p.bg_canvas).to_kde_rgb();
    let bg_surface = Rgba::parse(&p.bg_surface).to_kde_rgb();
    let bg_hover = Rgba::parse(&p.get_surface_hover()).to_kde_rgb();
    let text = Rgba::parse(&p.text_primary).to_kde_rgb();
    let text_muted = Rgba::parse(&p.get_text_muted()).to_kde_rgb();
    let green = Rgba::parse(&p.accent_green).to_kde_rgb();
    let blue = Rgba::parse(&p.accent_blue).to_kde_rgb();
    let red = Rgba::parse(&p.accent_red).to_kde_rgb();
    let amber = Rgba::parse(&p.accent_amber).to_kde_rgb();
    let purple = Rgba::parse(&p.accent_purple).to_kde_rgb();
    let primary = Rgba::parse(p.get_primary()).to_kde_rgb();

    format!(
r#"# Generated by inerium-theme for {} (author: {})
[ColorEffects:Disabled]
Color=56,56,56
ColorAmount=0
ColorEffect=0
ContrastAmount=0.65
ContrastEffect=1
IntensityAmount=0.1
IntensityEffect=2

[ColorEffects:Inactive]
ChangeSelectionColor=true
Color=112,111,110
ColorAmount=0.025
ColorEffect=2
ContrastAmount=0.1
ContrastEffect=2
Enable=false
IntensityAmount=0
IntensityEffect=0

[Colors:Button]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:Complementary]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:Header]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:Selection]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:Tooltip]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:View]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[Colors:Window]
BackgroundAlternate={}
BackgroundNormal={}
DecorationFocus={}
DecorationHover={}
ForegroundActive={}
ForegroundInactive={}
ForegroundLink={}
ForegroundNegative={}
ForegroundNeutral={}
ForegroundNormal={}
ForegroundPositive={}
ForegroundVisited={}

[General]
ColorScheme={}
Name={}
"#,
        theme.theme.name, theme.theme.author,
        bg_hover, bg_surface, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_canvas, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_hover, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_surface, bg_canvas, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, primary, primary, primary, text_muted, blue, red, amber, text, green, purple,
        theme.theme.name, theme.theme.name,
    )
}

fn emit_hyprlock_conf(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let card = Rgba::parse(&p.bg_surface);
    let border = Rgba::parse(&p.border_subtle);
    let heading = Rgba::parse(&p.text_heading);
    let amber = Rgba::parse(&p.accent_amber);
    let muted = Rgba::parse(&p.get_text_muted());
    let surface_alt = Rgba::parse(&p.get_surface_alt());
    let text = Rgba::parse(&p.text_primary);
    let green = Rgba::parse(&p.accent_green);
    let red = Rgba::parse(&p.accent_red);

    format!(
r#"# Generated by inerium-theme for {} (author: {})
$LOCK_CARD_COLOR = rgba({}, {}, {}, 0.92)
$LOCK_BORDER_COLOR = rgba({}, {}, {}, 0.10)
$LOCK_CLOCK_COLOR = rgba({}, {}, {}, 1.0)
$LOCK_DATE_COLOR = rgba({}, {}, {}, 1.0)
$LOCK_USER_COLOR = rgba({}, {}, {}, 1.0)
$LOCK_INPUT_BG = rgba({}, {}, {}, 1.0)
$LOCK_INPUT_BORDER = rgba({}, {}, {}, 0.12)
$LOCK_INPUT_FG = rgba({}, {}, {}, 1.0)
$LOCK_CHECK_COLOR = rgba({}, {}, {}, 0.85)
$LOCK_FAIL_COLOR = rgba({}, {}, {}, 0.90)
$LOCK_CAPS_COLOR = rgba({}, {}, {}, 0.90)
$LOCK_SUBTEXT_COLOR = rgba({}, {}, {}, 1.0)
$LOCK_PLACEHOLDER_COLOR = #{}
$LOCK_FAIL_TEXT_COLOR = #{}
$LOCK_CHECK_TEXT_COLOR = #{}
"#,
        theme.theme.name, theme.theme.author,
        card.0, card.1, card.2,
        border.0, border.1, border.2,
        heading.0, heading.1, heading.2,
        amber.0, amber.1, amber.2,
        muted.0, muted.1, muted.2,
        surface_alt.0, surface_alt.1, surface_alt.2,
        border.0, border.1, border.2,
        text.0, text.1, text.2,
        green.0, green.1, green.2,
        red.0, red.1, red.2,
        amber.0, amber.1, amber.2,
        muted.0, muted.1, muted.2,
        muted.to_hex_rgb(),
        red.to_hex_rgb(),
        green.to_hex_rgb(),
    )
}

fn emit_greeter_css(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let card = Rgba::parse(&p.bg_surface);
    let border = Rgba::parse(&p.border_subtle);
    let primary = Rgba::parse(p.get_primary());
    let red = Rgba::parse(&p.accent_red);
    let canvas = Rgba::parse(&p.bg_canvas);
    let surface_alt = Rgba::parse(&p.get_surface_alt());

    let bg_card_rgba = format!("rgba({}, {}, {}, 0.92)", card.0, card.1, card.2);
    let bg_end_rgba = format!("rgba({}, {}, {}, 0.85)", card.0, card.1, card.2);
    let card_border = format!("rgba({}, {}, {}, 0.10)", border.0, border.1, border.2);
    let input_border = format!("rgba({}, {}, {}, 0.12)", border.0, border.1, border.2);
    let btn_border = format!("rgba({}, {}, {}, 0.08)", border.0, border.1, border.2);
    let btn_border_hover = format!("rgba({}, {}, {}, 0.22)", border.0, border.1, border.2);
    let focus_border = format!("rgba({}, {}, {}, 0.65)", primary.0, primary.1, primary.2);
    let error_border = format!("rgba({}, {}, {}, 0.85)", red.0, red.1, red.2);
    let end_border = format!("rgba({}, {}, {}, 0.35)", red.0, red.1, red.2);
    let session_menu_bg = format!("rgba({}, {}, {}, 0.98)", canvas.0, canvas.1, canvas.2);
    let radial_center = format!("rgba({}, {}, {}, 0.85)", surface_alt.0, surface_alt.1, surface_alt.2);
    let radial_edge = format!("rgba({}, {}, {}, 0.98)", canvas.0, canvas.1, canvas.2);

    format!(
r#"/* Generated by inerium-theme for {} (author: {}, variant: {}) */
* {{ font-family: "JetBrainsMono Nerd Font", monospace; color: {}; }}
window {{
    background-color: {};
    background-image: radial-gradient(ellipse at center, {} 0%, {} 100%);
}}

.clock {{ font-size: 76px; font-weight: 800; color: {}; }}
.date  {{ font-size: 15px; font-weight: 700; color: {}; }}

.card {{
    background-color: {};
    border: 1px solid {};
    border-radius: 22px;
    padding: 24px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55);
}}
.user {{ font-size: 15px; font-weight: 700; color: {}; }}
.user-sub {{ font-size: 12px; color: {}; }}
.avatar {{
    min-width: 48px; min-height: 48px; border-radius: 14px;
    background-color: {}; color: {}; font-size: 22px;
}}

entry.password,
.password-input,
passwordentry {{
    min-height: 48px; padding: 0 16px; border-radius: 12px;
    background-color: {}; background-image: none; color: {}; font-size: 14px;
    border: 1px solid {}; box-shadow: none; outline: none;
}}
entry.password text,
.password-input text {{
    color: {};
    background-color: transparent;
    background-image: none;
}}
entry.password:focus-within,
.password-input:focus-within {{ border-color: {}; }}
entry.password.error,
.password-input.error {{ border-color: {}; }}
entry.password placeholder,
entry.password text placeholder,
.password-input placeholder {{ color: {}; }}
entry.password image,
.password-input image {{ color: {}; }}
entry.password image:hover,
.password-input image:hover {{ color: {}; }}

.session-btn {{
    min-height: 42px; padding: 0 14px; border-radius: 12px;
    background-color: {}; background-image: none; box-shadow: none;
    border: 1px solid {};
}}
.session-btn label {{ color: {}; }}
.session-btn:hover {{ border-color: {}; }}
.session-btn .arrow {{ color: {}; font-size: 14px; }}

.session-menu {{
    background-color: {};
    border: 1px solid {};
    border-radius: 12px; padding: 4px;
    margin-top: 4px;
}}
.session-item {{
    min-height: 38px; padding: 0 12px; border-radius: 8px;
    background: transparent; border: none; box-shadow: none;
}}
.session-item label {{ color: {}; font-size: 13.5px; }}
.session-item .check {{ color: {}; font-size: 14px; font-weight: bold; }}
.session-item:hover {{
    background-color: {};
}}
.session-item:hover label {{ color: {}; }}
.session-item.selected {{
    background-color: {};
}}
.session-item.selected label {{
    color: {};
    font-weight: 700;
}}

.go {{
    min-width: 48px; min-height: 48px; padding: 0; border-radius: 12px;
    background-color: {}; background-image: none; border: none; box-shadow: none;
    color: {}; font-size: 20px;
}}
.go label {{ color: {}; }}
.go:hover {{ background-color: {}; }}
.go:disabled {{ opacity: 0.6; }}

.label {{ font-size: 10.5px; font-weight: 700; color: {}; letter-spacing: 1px; }}
.status {{ font-size: 11.5px; color: {}; }}
.status.error {{ color: {}; }}
.status.warn {{ color: {}; }}

.end {{
    min-height: 34px; padding: 0 14px; border-radius: 12px; box-shadow: none; background-image: none;
    background-color: {};
    border: 1px solid {};
    font-size: 12px; font-weight: 600;
}}
.end label {{ color: {}; }}
.end:hover {{ background-color: {}; }}
.end:hover label {{ color: {}; }}
"#,
        theme.theme.name, theme.theme.author, theme.theme.variant,
        p.text_primary,
        p.bg_canvas,
        radial_center, radial_edge,
        p.text_heading,
        p.accent_amber,
        bg_card_rgba,
        card_border,
        p.text_heading,
        p.get_text_muted(),
        p.get_green_bg(), p.accent_green,
        p.get_surface_alt(), p.text_primary,
        input_border,
        p.text_primary,
        focus_border,
        error_border,
        p.get_text_muted(),
        p.get_text_muted(),
        p.text_primary,
        p.get_surface_alt(),
        btn_border,
        p.text_primary,
        btn_border_hover,
        p.get_text_muted(),
        session_menu_bg,
        card_border,
        p.text_primary,
        p.accent_green,
        p.get_surface_hover(),
        p.text_heading,
        p.get_green_bg(),
        p.accent_green,
        p.accent_green,
        p.get_text_inverted(),
        p.get_text_inverted(),
        p.get_green_border(),
        p.get_text_muted(),
        p.get_text_muted(),
        p.accent_red,
        p.accent_amber,
        bg_end_rgba,
        end_border,
        p.accent_red,
        p.accent_red,
        p.get_text_inverted(),
    )
}

// ============================================================================
// ATOMIC APPLICATION OF THEME
// ============================================================================

pub fn apply_theme(theme: &ThemeConfig) {
    let home = get_home();
    let is_dark = theme.theme.variant == "dark";
    let kde_scheme_name = format!("Inerium-{}", theme.theme.id);

    // 0. Ensure target directories exist
    let inerium_cfg_dir = home.join(".config/inerium");
    let _ = fs::create_dir_all(&inerium_cfg_dir);
    let _ = fs::create_dir_all(inerium_cfg_dir.join("hooks.d"));
    let _ = fs::create_dir_all(home.join(".config/hypr/config"));
    let _ = fs::create_dir_all(home.join(".config/waybar"));
    let _ = fs::create_dir_all(home.join(".config/rofi"));
    let _ = fs::create_dir_all(home.join(".config/kitty/themes"));
    let _ = fs::create_dir_all(home.join(".config/mako"));
    let _ = fs::create_dir_all(home.join(".local/share/color-schemes"));

    // 1. Write metadata current.json & theme-vars.css
    let current_json = inerium_cfg_dir.join("current.json");
    if let Ok(json_str) = serde_json::to_string_pretty(&theme.theme) {
        let _ = fs::write(&current_json, json_str + "\n");
    }

    let theme_vars_css = inerium_cfg_dir.join("theme-vars.css");
    let _ = fs::write(&theme_vars_css, emit_gtk_vars(theme));

    // 2. GSettings (Portal, Browsers, VS Code, GTK)
    let (gsettings_scheme, gtk_theme, icon_theme) = if is_dark {
        ("prefer-dark", "adw-gtk3-dark", "Papirus-Dark")
    } else {
        ("prefer-light", "adw-gtk3", "Papirus-Light")
    };

    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "color-scheme", gsettings_scheme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "gtk-theme", gtk_theme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "icon-theme", icon_theme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 2b. Typography (GSettings and GTK settings.ini)
    if let Some(typo) = &theme.typography {
        let font_ui_str = format!("{} {}", typo.get_ui_font(), typo.get_size());
        let font_mono_str = format!("{} {}", typo.get_mono_font(), typo.get_size());
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "font-name", &font_ui_str])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "document-font-name", &font_ui_str])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "monospace-font-name", &font_mono_str])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    // 3. GTK 3 & GTK 4 settings.ini and Libadwaita CSS
    let prefer_dark_str = if is_dark { "1" } else { "0" };
    let libadwaita_css = emit_libadwaita_css(theme);
    let font_spec = theme.typography.as_ref().map(|t| format!("{} {}", t.get_ui_font(), t.get_size()));
    for gtk_ver in &["gtk-3.0", "gtk-4.0"] {
        let dir = home.join(format!(".config/{}", gtk_ver));
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("gtk.css"), &libadwaita_css);

        let path = dir.join("settings.ini");
        if let Ok(content) = fs::read_to_string(&path) {
            let mut lines = Vec::new();
            let mut font_written = false;
            for line in content.lines() {
                if line.starts_with("gtk-theme-name=") {
                    lines.push(format!("gtk-theme-name={}", gtk_theme));
                } else if line.starts_with("gtk-application-prefer-dark-theme=") {
                    lines.push(format!("gtk-application-prefer-dark-theme={}", prefer_dark_str));
                } else if line.starts_with("gtk-icon-theme-name=") {
                    lines.push(format!("gtk-icon-theme-name={}", icon_theme));
                } else if line.starts_with("gtk-font-name=") {
                    if let Some(ref f) = font_spec {
                        lines.push(format!("gtk-font-name={}", f));
                        font_written = true;
                    } else {
                        lines.push(line.to_string());
                    }
                } else {
                    lines.push(line.to_string());
                }
            }
            if !font_written && let Some(ref f) = font_spec {
                lines.push(format!("gtk-font-name={}", f));
            }
            let _ = fs::write(&path, lines.join("\n") + "\n");
        }
    }

    // 3b. XDG Desktop Portal GTK (refresh CSS cache for file/app choosers)
    let _ = Command::new("systemctl")
        .args(["--user", "try-restart", "xdg-desktop-portal-gtk"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 4. KDE / Dolphin / Qt
    let kde_colors_content = emit_kde_colors(theme);
    let kde_colors_path = home.join(format!(".local/share/color-schemes/{}.colors", kde_scheme_name));
    let _ = fs::write(&kde_colors_path, &kde_colors_content);

    let kdeglobals_path = home.join(".config/kdeglobals");
    let mut kdeglobals = format!(
        "[General]\nColorScheme={}\nName={}\nTerminalApplication=kitty\n\n[Icons]\nTheme={}\n\n[KDE]\ncontrast=4\nwidgetStyle=Fusion\n\n",
        kde_scheme_name,
        theme.theme.name,
        icon_theme,
    );
    kdeglobals.push_str(&kde_colors_content);
    let _ = fs::write(&kdeglobals_path, kdeglobals.trim().to_string() + "\n");

    let dolphinrc_path = home.join(".config/dolphinrc");
    if let Ok(content) = fs::read_to_string(&dolphinrc_path) {
        let mut lines = Vec::new();
        let mut has_ui = false;
        let mut in_ui = false;
        for line in content.lines() {
            if line.trim() == "[UiSettings]" {
                in_ui = true;
                has_ui = true;
                lines.push(line.to_string());
                continue;
            }
            if in_ui && line.starts_with('[') {
                in_ui = false;
            }
            if in_ui && line.starts_with("ColorScheme=") {
                lines.push(format!("ColorScheme={}", kde_scheme_name));
                continue;
            }
            lines.push(line.to_string());
        }
        if !has_ui {
            lines.push("[UiSettings]".to_string());
            lines.push(format!("ColorScheme={}", kde_scheme_name));
        }
        let _ = fs::write(&dolphinrc_path, lines.join("\n") + "\n");
    }

    let _ = Command::new("plasma-apply-colorscheme")
        .arg(&kde_scheme_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 5. Waybar
    let waybar_colors = home.join(".config/waybar/colors.css");
    let _ = fs::write(&waybar_colors, emit_waybar_css(theme));
    let _ = Command::new("pkill")
        .args(["-SIGUSR2", "waybar"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 6. Rofi
    let rofi_colors = home.join(".config/rofi/colors.rasi");
    let _ = fs::write(&rofi_colors, emit_rofi_rasi(theme));

    // 7. Kitty
    let kitty_theme = home.join(".config/kitty/themes/current-theme.conf");
    let _ = fs::write(&kitty_theme, emit_kitty_conf(theme));
    let _ = Command::new("killall")
        .args(["-SIGUSR1", "kitty"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 8. Mako
    let mako_cfg = home.join(".config/mako/config");
    let _ = fs::write(&mako_cfg, emit_mako_config(theme));
    let _ = Command::new("makoctl")
        .arg("reload")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 9. Hyprland
    let hypr_colors = home.join(".config/hypr/config/colors.lua");
    let _ = fs::write(&hypr_colors, emit_hyprland_lua(theme));
    let _ = Command::new("hyprctl")
        .arg("reload")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 10. Hyprlock & Greeter
    let hyprlock_colors = home.join(".config/hypr/hyprlock-colors.conf");
    let _ = fs::write(&hyprlock_colors, emit_hyprlock_conf(theme));

    let greeter_css = emit_greeter_css(theme);
    let greeter_targets = [
        PathBuf::from("/var/cache/inerium/greeter-theme.css"),
        PathBuf::from("/var/tmp/inerium/greeter-theme.css"),
    ];
    for t in &greeter_targets {
        if let Some(parent) = t.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(t, &greeter_css);
    }

    // 11. Display Color Temperature (Harmonization for paper light themes)
    let temp = theme
        .display
        .as_ref()
        .and_then(|d| d.temperature)
        .unwrap_or(6500);
    apply_display_temperature(temp);

    // 11. Custom User & App Hooks (~/.config/inerium/hooks.d/*)
    run_theme_hooks(theme);

    // 12. Inerium Shell Daemon Hot-Reload via UNIX domain socket
    let uid = inerium_tools::sys::posix::posix_getuid();
    let sock_path = format!("/run/user/{}/inerium-popups.sock", uid);
    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
        let msg = format!("theme {}\n", theme.theme.variant);
        let _ = stream.write_all(msg.as_bytes());
    }
}

fn simple_time() -> String {
    if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let total_secs = duration.as_secs();
        let hours = (total_secs / 3600) % 24;
        let mins = (total_secs / 60) % 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        "00:00:00".to_string()
    }
}

pub fn generate_new_theme_template(id: &str) -> String {
    let title_cased = id
        .split(['-', '_'])
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!(
r##"# ==============================================================================
# Inerium Desktop Theme: {title_cased}
#
# ФОРМАТЫ ЦВЕТОВ:
#   Вы можете указывать цвета в любом удобном формате:
#     - HEX:      "#1e2029", "1e2029" (без решётки), "#123" (3-значный)
#     - HEX+Alpha:"#1e2029cc", "#1238"
#     - RGB:      "rgb(30, 32, 41)"
#     - RGBA:     "rgba(30, 32, 41, 0.85)"
#     - CSS-4:    "rgb(30 32 41 / 85%)"
#
# СИСТЕМА АВТОВЫЧИСЛЕНИЙ:
#   Поля с пометкой (опционально) можно опустить. Движок автоматически рассчитает
#   нужные оттенки (фон кнопок @ 16%, рамки @ 35%, hover @ +8%, active @ +15%).
# ==============================================================================

[theme]
id = "{id}"
name = "{title_cased}"
author = "User"
variant = "dark" # "dark" или "light"

# ------------------------------------------------------------------------------
# Шрифты интерфейса (опционально)
# Автоматически настраивает GTK, Waybar, Rofi, Kitty, Mako и системные окна.
# Включает защитный fallback на "JetBrainsMono Nerd Font" для иконок.
# ------------------------------------------------------------------------------
[typography]
font_ui = "Inter"
font_mono = "JetBrainsMono Nerd Font"
font_size = 10.0

# ------------------------------------------------------------------------------
# Цветовая палитра
# ------------------------------------------------------------------------------
[palette]
# Основные холсты и карточки
bg_canvas = "#111216"           # Фоновый цвет рабочего стола / подложки
bg_surface = "#1e2029"          # Цвет карточек, виджетов Waybar, меню
# bg_surface_alt = "#17181f"    # (опционально) Вторичные поверхности / списки
# bg_surface_hover = "#262935"  # (опционально) Фон при наведении
# bg_surface_active = "#2e3241" # (опционально) Фон при клике / нажатии

# Текст и контраст
text_heading = "#e6e8ee"        # Заголовки, жирный текст
text_primary = "#cfd2dd"        # Основной текст приложений
# text_muted = "#72778e"        # (опционально) Второстепенные подписи / плейсхолдеры
# text_subtle = "#9ba0b4"       # (опционально) Промежуточный текст
# text_inverted = "#111216"     # (опционально) Текст на контрастных плашках

# Рамки и границы окон Hyprland
border_subtle = "rgba(147, 138, 169, 0.20)"  # Неактивные окна и разделители
border_active = "#938aa9"                    # Активное окно Hyprland
# border_active_secondary = "#938aa9"        # (опционально) 2-й цвет для градиента рамки окна

# Ползунки громкости и яркости в панелях
# slider_fill = "#938aa9"                    # (опционально, по умолчанию = primary)
# slider_fill_secondary = "#e6c384"          # (опционально, для внешнего монитора)

# Акцентные цвета
accent_primary = "#938aa9"      # Главный акцент системы (кнопки, фокус, бейджи)
accent_green = "#87a987"        # Статус сети, активные тумблеры, успех
accent_blue = "#7fb4ca"         # Ссылки, информация, Bluetooth
accent_amber = "#e6c384"        # Предупреждения, зарядка батареи
accent_red = "#e46876"          # Ошибки, кнопка выключения, опасность
accent_aqua = "#7aa89f"         # Дополнительный акцент
accent_purple = "#938aa9"       # Фиолетовый / аметистовый
"##
    )
}

fn cmd_new(id: &str) {
    let clean_id = id.trim().to_lowercase().replace(' ', "-");
    if clean_id.is_empty() {
        eprintln!("󰅖 Ошибка: ID темы не может быть пустым.");
        eprintln!("Использование: inerium-theme new <id_темы>");
        std::process::exit(1);
    }

    let home = get_home();
    let theme_dir = home.join(".config/inerium/themes");
    let _ = fs::create_dir_all(&theme_dir);
    let theme_path = theme_dir.join(format!("{}.toml", clean_id));

    if theme_path.exists() {
        eprintln!("󰅖 Файл темы уже существует: {}", theme_path.display());
        eprintln!("  Отредактируйте его или удалите перед созданием заново.");
        std::process::exit(1);
    }

    let content = generate_new_theme_template(&clean_id);
    if let Err(e) = fs::write(&theme_path, content) {
        eprintln!("󰅖 Не удалось создать файл {}: {}", theme_path.display(), e);
        std::process::exit(1);
    }

    println!("󰄬 Новая тема успешно создана: {}", theme_path.display());
    println!("  Редактируйте её в любом редакторе и запустите live-просмотр:");
    println!("   inerium-theme watch {}", clean_id);
}

fn cmd_watch(id: &str) {
    let clean_id = id.trim().to_lowercase().replace(' ', "-");
    let home = get_home();

    let target_path = if clean_id.ends_with(".toml") && Path::new(&clean_id).exists() {
        PathBuf::from(&clean_id)
    } else {
        let user_theme = home.join(format!(".config/inerium/themes/{}.toml", clean_id));
        if user_theme.exists() {
            user_theme
        } else {
            let sys_theme = PathBuf::from(format!("/usr/share/inerium/themes/{}.toml", clean_id));
            if sys_theme.exists() {
                sys_theme
            } else {
                eprintln!("󰅖 Тема '{}' не найдена в ~/.config/inerium/themes/", clean_id);
                eprintln!("  Создайте её командой: inerium-theme new {}", clean_id);
                std::process::exit(1);
            }
        }
    };

    println!("󰈈 Запущен режим отслеживания: {}", target_path.display());
    println!("  Нажмите Ctrl+C для выхода. Изменения применяются сразу при сохранении файла.");

    let read_and_apply = |path: &Path| -> bool {
        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<ThemeConfig>(&content) {
                Ok(theme) => {
                    apply_theme(&theme);
                    let now = simple_time();
                    println!("  [✓ {}] Тема '{}' успешно обновлена и применена!", now, theme.theme.name);
                    true
                }
                Err(e) => {
                    let now = simple_time();
                    eprintln!("  [✗ {}] Ошибка синтаксиса TOML: {}", now, e);
                    false
                }
            },
            Err(e) => {
                eprintln!("  [✗] Ошибка чтения файла {}: {}", path.display(), e);
                false
            }
        }
    };

    // Initial apply
    read_and_apply(&target_path);

    let mut last_mtime = fs::metadata(&target_path).and_then(|m| m.modified()).ok();

    loop {
        thread::sleep(Duration::from_millis(250));
        let cur_mtime = fs::metadata(&target_path).and_then(|m| m.modified()).ok();
        if cur_mtime.is_some() && cur_mtime != last_mtime {
            last_mtime = cur_mtime;
            thread::sleep(Duration::from_millis(50));
            read_and_apply(&target_path);
        }
    }
}

fn print_help() {
    println!(
        r#"inerium-theme — Менеджер и компилятор тем оформления Inerium Desktop

ИСПОЛЬЗОВАНИЕ:
    inerium-theme <КОМАНДА>

КОМАНДЫ:
    list [--all] [--json] Показать активные темы (или все включая дополнительные с --all)
    set <id>              Применить палитру по ID (если тема в extra, переносит её в постоянные)
    preview <id>          Быстрый показ палитры (только применение оформления без переноса файла)
    install <id>          Перенести тему из дополнительных в постоянные и применить
    new <id>              Создать шаблон новой темы в ~/.config/inerium/themes/<id>.toml
    watch [id]            Запустить отслеживание изменений темы (live reload на лету)
    toggle                Быстрое переключение (темный <-> светлый вариант)
    status                Вывести текущую активную тему и авторство в формате JSON
    help, --help          Показать эту справку
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return;
    }

    let active_themes = load_available_themes();

    match args[0].as_str() {
        "list" => {
            let cur_id = get_current_theme_id();
            let show_all = args.iter().any(|a| a == "--all" || a == "-a");
            let themes_map = if show_all {
                load_all_themes()
            } else {
                active_themes.clone()
            };

            if args.iter().any(|a| a == "--json") {
                let list: Vec<_> = themes_map
                    .iter()
                    .map(|(id, t)| {
                        let is_extra = !active_themes.contains_key(id);
                        serde_json::json!({
                            "id": id,
                            "name": t.theme.name,
                            "author": t.theme.author,
                            "variant": t.theme.variant,
                            "description": t.theme.description.clone().unwrap_or_default(),
                            "is_active": id == &cur_id,
                            "is_extra": is_extra,
                            "colors": {
                                "bg": t.palette.bg_canvas,
                                "surface": t.palette.bg_surface,
                                "accent": t.palette.get_primary(),
                                "fg": t.palette.text_primary,
                            }
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&list).unwrap_or_default());
                return;
            }

            println!("Доступные палитры Inerium Desktop:\n");
            for (id, t) in &themes_map {
                let active_mark = if id == &cur_id { "* " } else { "  " };
                let extra_mark = if !active_themes.contains_key(id) { " (extra)" } else { "" };
                let author_str = format!("(автор: {}){}", t.theme.author, extra_mark);
                println!(
                    "{}{:<18} [{:<5}] {:<22} {}",
                    active_mark, id, t.theme.variant, t.theme.name, author_str
                );
                if let Some(desc) = &t.theme.description {
                    println!("     └─ {}", desc);
                }
            }
            println!("\nПрименить: inerium-theme set <id>");
        }
        "status" => {
            let cur_id = get_current_theme_id();
            let theme = active_themes.get(&cur_id).or_else(|| active_themes.get("kanagawa-dragon"));
            if let Some(t) = theme {
                if let Ok(s) = serde_json::to_string(&t.theme) {
                    println!("{}", s);
                }
            } else {
                println!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",\"author\":\"rebelot\",\"variant\":\"dark\"}}",
                    cur_id, cur_id
                );
            }
        }
        "toggle" => {
            let cur_id = get_current_theme_id();
            let cur_theme = active_themes.get(&cur_id).or_else(|| active_themes.get("kanagawa-dragon"));
            let is_dark = cur_theme.map(|t| t.theme.variant == "dark").unwrap_or(true);

            let target_id = if is_dark {
                if active_themes.contains_key("inerium-light") {
                    "inerium-light"
                } else {
                    "kanagawa-lotus"
                }
            } else {
                "kanagawa-dragon"
            };

            if let Some(target) = active_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "Тема переключена: {} (автор: {})",
                    target.theme.name, target.theme.author
                );
            } else {
                eprintln!("Ошибка: целевая палитра '{}' не найдена.", target_id);
                std::process::exit(1);
            }
        }
        "install" | "add" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите идентификатор темы. Выполните 'inerium-theme list --all'.");
                std::process::exit(1);
            }
            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);
            match install_extra_theme(target_id) {
                Ok(theme) => {
                    apply_theme(&theme);
                    println!(
                        "Тема '{}' (автор: {}) успешно перенесена в постоянные и применена.",
                        theme.theme.name, theme.theme.author
                    );
                }
                Err(e) => {
                    eprintln!("Ошибка: {e}");
                    std::process::exit(1);
                }
            }
        }
        "preview" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите идентификатор темы для превью. Выполните 'inerium-theme list --all'.");
                std::process::exit(1);
            }
            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);
            let all_themes = load_all_themes();
            if let Some(target) = all_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "Быстрый показ палитры: {} (автор: {})",
                    target.theme.name, target.theme.author
                );
            } else {
                eprintln!("Ошибка: тема '{}' не найдена.", target_id);
                println!("Запустите 'inerium-theme list --all' для просмотра всех доступных палитр.");
                std::process::exit(1);
            }
        }
        "set" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите идентификатор темы. Выполните 'inerium-theme list'.");
                std::process::exit(1);
            }

            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);

            if let Some(target) = active_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "Установлена палитра: {} (автор: {})",
                    target.theme.name, target.theme.author
                );
                if let Some(upstream) = &target.theme.upstream {
                    println!("Оригинальный репозиторий: {}", upstream);
                }
            } else if let Ok(target) = install_extra_theme(target_id) {
                apply_theme(&target);
                println!(
                    "Палитра '{}' перенесена из дополнительных в постоянные и применена (автор: {}).",
                    target.theme.name, target.theme.author
                );
                if let Some(upstream) = &target.theme.upstream {
                    println!("Оригинальный репозиторий: {}", upstream);
                }
            } else {
                eprintln!("Ошибка: тема '{}' не найдена.", target_id);
                println!("Запустите 'inerium-theme list --all' для просмотра всех доступных палитр.");
                std::process::exit(1);
            }
        }
        "new" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите идентификатор новой темы. Пример: inerium-theme new my-theme");
                std::process::exit(1);
            }
            cmd_new(&args[1]);
        }
        "watch" => {
            let id = if args.len() >= 2 {
                args[1].clone()
            } else {
                get_current_theme_id()
            };
            cmd_watch(&id);
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!("Неизвестная команда: '{}'. Запустите 'inerium-theme --help'.", other);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_parse_hex3_hex6_hex8_and_bare() {
        assert_eq!(Rgba::parse("#fff"), Rgba(255, 255, 255, 255));
        assert_eq!(Rgba::parse("#000"), Rgba(0, 0, 0, 255));
        assert_eq!(Rgba::parse("#1238"), Rgba(17, 34, 51, 136));
        assert_eq!(Rgba::parse("#1e2029"), Rgba(30, 32, 41, 255));
        assert_eq!(Rgba::parse("1e2029"), Rgba(30, 32, 41, 255));
        assert_eq!(Rgba::parse("#1e202980"), Rgba(30, 32, 41, 128));
        assert_eq!(Rgba::parse("transparent"), Rgba(0, 0, 0, 0));
        assert_eq!(Rgba::parse("white"), Rgba(255, 255, 255, 255));
        assert_eq!(Rgba::parse("black"), Rgba(0, 0, 0, 255));
    }

    #[test]
    fn test_rgba_parse_rgb_rgba_and_slash() {
        assert_eq!(Rgba::parse("rgb(30, 32, 41)"), Rgba(30, 32, 41, 255));
        assert_eq!(Rgba::parse("rgba(30, 32, 41, 0.5)"), Rgba(30, 32, 41, 128));
        assert_eq!(Rgba::parse("rgb(30 32 41 / 50%)"), Rgba(30, 32, 41, 128));
    }

    #[test]
    fn test_palette_auto_derivation() {
        let p = Palette {
            bg_canvas: "#111216".to_string(),
            bg_surface: "#1e2029".to_string(),
            bg_surface_alt: None,
            bg_surface_hover: None,
            bg_surface_active: None,
            text_heading: "#e6e8ee".to_string(),
            text_primary: "#cfd2dd".to_string(),
            text_muted: None,
            text_subtle: None,
            text_inverted: None,
            border_subtle: "rgba(147, 138, 169, 0.20)".to_string(),
            border_active: "#938aa9".to_string(),
            border_active_secondary: None,
            accent_green: "#87a987".to_string(),
            accent_green_bg: None,
            accent_green_border: None,
            accent_blue: "#7fb4ca".to_string(),
            accent_blue_bg: None,
            accent_blue_border: None,
            accent_amber: "#e6c384".to_string(),
            accent_amber_bg: None,
            accent_amber_border: None,
            accent_red: "#e46876".to_string(),
            accent_red_bg: None,
            accent_red_border: None,
            accent_aqua: "#7aa89f".to_string(),
            accent_aqua_bg: None,
            accent_aqua_border: None,
            accent_purple: "#938aa9".to_string(),
            accent_purple_bg: None,
            accent_purple_border: None,
            accent_primary: None,
            accent_orange: None,
            slider_fill: None,
            slider_fill_secondary: None,
            slider_trough: None,
            slider_thumb: None,
            shadow: None,
            modal_backdrop: None,
        };

        assert_eq!(p.get_primary(), "#7fb4ca");
        assert_eq!(p.get_slider_fill(), "#7fb4ca");
        assert_eq!(p.get_slider_fill_secondary(), "#e6c384");
        assert_eq!(p.get_border_active_secondary(), "#938aa9");
        assert!(p.get_green_bg().contains("rgba(135, 169, 135, 0.16)"));
        assert!(p.get_green_border().contains("rgba(135, 169, 135, 0.35)"));
    }

    #[test]
    fn test_typography_fallbacks() {
        let typo = Typography {
            font_ui: Some("Inter".to_string()),
            font_mono: Some("JetBrainsMono Nerd Font".to_string()),
            font_size: Some(10.5),
        };
        assert_eq!(
            typo.to_css_font_family(),
            "\"Inter\", \"JetBrainsMono Nerd Font\", sans-serif, monospace"
        );
    }

    #[test]
    fn test_slider_fill_fallback() {
        let mut p = Palette {
            bg_canvas: "#111216".to_string(),
            bg_surface: "#1e2029".to_string(),
            bg_surface_alt: None,
            bg_surface_hover: None,
            bg_surface_active: None,
            text_heading: "#e6e8ee".to_string(),
            text_primary: "#cfd2dd".to_string(),
            text_muted: None,
            text_subtle: None,
            text_inverted: None,
            border_subtle: "rgba(147, 138, 169, 0.20)".to_string(),
            border_active: "#938aa9".to_string(),
            border_active_secondary: None,
            accent_green: "#87a987".to_string(),
            accent_green_bg: None,
            accent_green_border: None,
            accent_blue: "#7fb4ca".to_string(),
            accent_blue_bg: None,
            accent_blue_border: None,
            accent_amber: "#e6c384".to_string(),
            accent_amber_bg: None,
            accent_amber_border: None,
            accent_red: "#e46876".to_string(),
            accent_red_bg: None,
            accent_red_border: None,
            accent_aqua: "#7aa89f".to_string(),
            accent_aqua_bg: None,
            accent_aqua_border: None,
            accent_purple: "#938aa9".to_string(),
            accent_purple_bg: None,
            accent_purple_border: None,
            accent_primary: Some("#938aa9".to_string()),
            accent_orange: None,
            slider_fill: None,
            slider_fill_secondary: None,
            slider_trough: None,
            slider_thumb: None,
            shadow: None,
            modal_backdrop: None,
        };

        // Fallbacks to primary and amber
        assert_eq!(p.get_slider_fill(), "#938aa9");
        assert_eq!(p.get_slider_fill_secondary(), "#e6c384");

        // Custom override
        p.slider_fill = Some("#87a987".to_string());
        p.slider_fill_secondary = Some("#7fb4ca".to_string());
        assert_eq!(p.get_slider_fill(), "#87a987");
        assert_eq!(p.get_slider_fill_secondary(), "#7fb4ca");
    }
}
