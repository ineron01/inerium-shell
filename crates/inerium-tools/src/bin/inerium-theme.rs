use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

pub const EMBEDDED_DRAGON: &str = include_str!("../../../../themes/kanagawa-dragon.toml");
pub const EMBEDDED_INERIUM_LIGHT: &str = include_str!("../../../../themes/inerium-light.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub theme: ThemeMeta,
    pub palette: Palette,
    #[serde(default)]
    pub display: Option<DisplayConfig>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg_canvas: String,
    pub bg_surface: String,
    pub bg_surface_alt: String,
    pub bg_surface_hover: String,
    pub bg_surface_active: String,
    pub border_subtle: String,
    pub border_active: String,
    pub text_primary: String,
    pub text_heading: String,
    pub text_muted: String,
    pub text_subtle: String,
    pub text_inverted: String,
    pub accent_green: String,
    pub accent_green_bg: String,
    pub accent_green_border: String,
    pub accent_blue: String,
    pub accent_blue_bg: String,
    pub accent_blue_border: String,
    pub accent_amber: String,
    pub accent_amber_bg: String,
    pub accent_amber_border: String,
    pub accent_red: String,
    pub accent_red_bg: String,
    pub accent_red_border: String,
    pub accent_aqua: String,
    pub accent_aqua_bg: String,
    pub accent_aqua_border: String,
    pub accent_purple: String,
    pub accent_purple_bg: String,
    pub accent_purple_border: String,
    pub shadow: String,
    pub modal_backdrop: String,
    pub slider_trough: String,
    pub slider_thumb: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if let Some(stripped) = s.strip_prefix('#') {
            if stripped.len() == 6 {
                let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
                return Rgba(r, g, b, 255);
            } else if stripped.len() == 8 {
                let r = u8::from_str_radix(&stripped[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&stripped[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&stripped[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&stripped[6..8], 16).unwrap_or(255);
                return Rgba(r, g, b, a);
            }
        } else if s.starts_with("rgba(") || s.starts_with("rgb(") {
            let inner = s
                .trim_start_matches("rgba(")
                .trim_start_matches("rgb(")
                .trim_end_matches(')');
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() >= 3 {
                let r = parts[0].parse::<u8>().unwrap_or(0);
                let g = parts[1].parse::<u8>().unwrap_or(0);
                let b = parts[2].parse::<u8>().unwrap_or(0);
                let a = if parts.len() >= 4 {
                    if let Ok(alpha_float) = parts[3].parse::<f32>() {
                        (alpha_float.clamp(0.0, 1.0) * 255.0).round() as u8
                    } else {
                        parts[3].parse::<u8>().unwrap_or(255)
                    }
                } else {
                    255
                };
                return Rgba(r, g, b, a);
            }
        }
        Rgba(0, 0, 0, 255)
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
            if path.is_file() {
                if path.extension().is_some_and(|e| e == "toml")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(c) = toml::from_str::<ThemeConfig>(&content)
                {
                    map.insert(c.theme.id.clone(), c);
                }
            } else if path.is_dir()
                && let Ok(sub_entries) = fs::read_dir(&path)
            {
                for sub in sub_entries.flatten() {
                    let sub_p = sub.path();
                    if sub_p.is_file()
                        && sub_p.extension().is_some_and(|e| e == "toml")
                        && let Ok(content) = fs::read_to_string(&sub_p)
                        && let Ok(c) = toml::from_str::<ThemeConfig>(&content)
                    {
                        map.insert(c.theme.id.clone(), c);
                    }
                }
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
        home.join(".local/share/inerium-shell/themes"),
        home.join(".config/inerium/themes"),
    ];

    for root in &theme_roots {
        scan_dir_for_themes(root, &mut map);
    }

    map
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

INERIUM_BASE        = "{}"
INERIUM_SURFACE     = "{}"
INERIUM_BORDER      = "{}"
INERIUM_TEXT        = "{}"
INERIUM_MUTED       = "{}"

INERIUM_GREEN       = "{}"
INERIUM_BLUE        = "{}"
INERIUM_AMBER       = "{}"
INERIUM_RED         = "{}"
INERIUM_PURPLE      = "{}"

INERIUM_DRAGON_GRAY = "{}"
INERIUM_DRAGON_BLUE = "{}"
"#,
        theme.theme.name,
        theme.theme.author,
        theme.theme.upstream.as_deref().unwrap_or("N/A"),
        Rgba::parse(&p.bg_canvas).to_hypr(),
        Rgba::parse(&p.bg_surface).to_hypr(),
        Rgba::parse(&p.border_subtle).to_hypr(),
        Rgba::parse(&p.text_primary).to_hypr(),
        Rgba::parse(&p.text_muted).to_hypr(),
        Rgba::parse(&p.accent_green).to_hypr(),
        Rgba::parse(&p.accent_blue).to_hypr(),
        Rgba::parse(&p.accent_amber).to_hypr(),
        Rgba::parse(&p.accent_red).to_hypr(),
        Rgba::parse(&p.accent_purple).to_hypr(),
        Rgba::parse(&p.border_subtle).to_hypr(),
        Rgba::parse(&p.accent_blue).to_hypr(),
    )
}

fn emit_waybar_css(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
@define-color bg        {};
@define-color surface   {};
@define-color hover     {};
@define-color line      {};
@define-color fg        {};
@define-color fg-bright {};
@define-color muted     {};
@define-color subtle    {};
@define-color green     {};
@define-color blue      {};
@define-color aqua      {};
@define-color yellow    {};
@define-color orange    {};
@define-color red       {};
@define-color violet    {};
"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.bg_surface_hover,
        p.border_subtle,
        p.text_primary,
        p.text_heading,
        p.text_muted,
        p.text_subtle,
        p.accent_green,
        p.accent_blue,
        p.accent_aqua,
        p.accent_amber,
        p.accent_amber,
        p.accent_red,
        p.accent_purple,
    )
}

fn emit_rofi_rasi(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
* {{
    bg:           {};
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
}}
"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.bg_surface_hover,
        p.border_active,
        p.border_subtle,
        p.text_primary,
        p.text_heading,
        p.text_muted,
        p.accent_green,
        p.accent_green_bg,
        p.accent_green_border,
        p.accent_red,
    )
}

fn emit_kitty_conf(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let bg = Rgba::parse(&p.bg_canvas).to_hex_rgb();
    let fg = Rgba::parse(&p.text_primary).to_hex_rgb();
    let sel_bg = Rgba::parse(&p.bg_surface_hover).to_hex_rgb();
    let green = Rgba::parse(&p.accent_green).to_hex_rgb();
    let blue = Rgba::parse(&p.accent_blue).to_hex_rgb();
    let red = Rgba::parse(&p.accent_red).to_hex_rgb();
    let amber = Rgba::parse(&p.accent_amber).to_hex_rgb();
    let aqua = Rgba::parse(&p.accent_aqua).to_hex_rgb();
    let purple = Rgba::parse(&p.accent_purple).to_hex_rgb();
    let muted = Rgba::parse(&p.text_muted).to_hex_rgb();
    let heading = Rgba::parse(&p.text_heading).to_hex_rgb();

    format!(
r#"# Generated by inerium-theme for {} (author: {})

foreground            {}
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
        blue,
        bg,
        green,
        muted,
        sel_bg,
        green,
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
        red,
        green,
        amber,
        blue,
        purple,
        aqua,
        heading,
    )
}

fn emit_mako_config(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let bg = Rgba::parse(&p.bg_canvas).to_hex_rgba();
    let text = Rgba::parse(&p.text_primary).to_hex_rgb();
    let text_subtle = Rgba::parse(&p.text_subtle).to_hex_rgb();
    let text_muted = Rgba::parse(&p.text_muted).to_hex_rgb();
    let border = Rgba::parse(&p.border_subtle).to_hex_rgba();
    let red = Rgba::parse(&p.accent_red).to_hex_rgb();
    let green_progress = Rgba::parse(&p.accent_green_bg).to_hex_rgba();

    format!(
r#"# Generated by inerium-theme for {} (author: {})
font=JetBrainsMono Nerd Font 10
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
        bg,
        text,
        border,
        green_progress,
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
    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
@define-color cc_bg               {};
@define-color cc_surface          {};
@define-color cc_surface_hover    {};
@define-color cc_surface_active   {};
@define-color cc_line             {};
@define-color cc_line_light       {};
@define-color cc_fg               {};
@define-color cc_fg_bright        {};
@define-color cc_fg_muted         {};
@define-color cc_fg_subtle        {};
@define-color cc_fg_on_accent     {};

@define-color cc_green            {};
@define-color cc_green_bg         {};
@define-color cc_green_border     {};

@define-color cc_blue             {};
@define-color cc_blue_bg          {};
@define-color cc_blue_border      {};

@define-color cc_amber            {};
@define-color cc_amber_bg         {};
@define-color cc_amber_border     {};

@define-color cc_red              {};
@define-color cc_red_bg           {};
@define-color cc_red_border       {};

@define-color cc_aqua             {};
@define-color cc_aqua_bg          {};
@define-color cc_aqua_border      {};

@define-color cc_violet           {};
@define-color cc_violet_bg        {};
@define-color cc_violet_border    {};

@define-color cc_shadow           {};
@define-color cc_modal_backdrop   {};
@define-color cc_slider_trough    {};
@define-color cc_slider_thumb     {};
"#,
        theme.theme.name,
        theme.theme.author,
        p.bg_canvas,
        p.bg_surface,
        p.bg_surface_hover,
        p.bg_surface_active,
        p.border_subtle,
        p.border_active,
        p.text_primary,
        p.text_heading,
        p.text_muted,
        p.text_subtle,
        p.text_inverted,
        p.accent_green,
        p.accent_green_bg,
        p.accent_green_border,
        p.accent_blue,
        p.accent_blue_bg,
        p.accent_blue_border,
        p.accent_amber,
        p.accent_amber_bg,
        p.accent_amber_border,
        p.accent_red,
        p.accent_red_bg,
        p.accent_red_border,
        p.accent_aqua,
        p.accent_aqua_bg,
        p.accent_aqua_border,
        p.accent_purple,
        p.accent_purple_bg,
        p.accent_purple_border,
        p.shadow,
        p.modal_backdrop,
        p.slider_trough,
        p.slider_thumb,
    )
}

fn emit_libadwaita_css(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
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
        p.accent_blue,
        p.accent_blue,
        p.bg_canvas,
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
    let bg_hover = Rgba::parse(&p.bg_surface_hover).to_kde_rgb();
    let text = Rgba::parse(&p.text_primary).to_kde_rgb();
    let text_muted = Rgba::parse(&p.text_muted).to_kde_rgb();
    let green = Rgba::parse(&p.accent_green).to_kde_rgb();
    let blue = Rgba::parse(&p.accent_blue).to_kde_rgb();
    let red = Rgba::parse(&p.accent_red).to_kde_rgb();
    let amber = Rgba::parse(&p.accent_amber).to_kde_rgb();
    let purple = Rgba::parse(&p.accent_purple).to_kde_rgb();

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
        bg_hover, bg_surface, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_canvas, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_hover, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_surface, bg_canvas, green, green, green, text_muted, blue, red, amber, text, green, purple,
        bg_hover, bg_surface, green, green, green, text_muted, blue, red, amber, text, green, purple,
        theme.theme.name, theme.theme.name,
    )
}

fn emit_hyprlock_conf(theme: &ThemeConfig) -> String {
    let p = &theme.palette;
    let card = Rgba::parse(&p.bg_surface);
    let border = Rgba::parse(&p.border_subtle);
    let heading = Rgba::parse(&p.text_heading);
    let amber = Rgba::parse(&p.accent_amber);
    let muted = Rgba::parse(&p.text_muted);
    let surface_alt = Rgba::parse(&p.bg_surface_alt);
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
$LOCK_PLACEHOLDER_COLOR = {}
$LOCK_FAIL_TEXT_COLOR = {}
$LOCK_CHECK_TEXT_COLOR = {}
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
    let green = Rgba::parse(&p.accent_green);
    let red = Rgba::parse(&p.accent_red);
    let canvas = Rgba::parse(&p.bg_canvas);
    let surface_alt = Rgba::parse(&p.bg_surface_alt);

    let bg_card_rgba = format!("rgba({}, {}, {}, 0.92)", card.0, card.1, card.2);
    let bg_end_rgba = format!("rgba({}, {}, {}, 0.85)", card.0, card.1, card.2);
    let card_border = format!("rgba({}, {}, {}, 0.10)", border.0, border.1, border.2);
    let input_border = format!("rgba({}, {}, {}, 0.12)", border.0, border.1, border.2);
    let btn_border = format!("rgba({}, {}, {}, 0.08)", border.0, border.1, border.2);
    let btn_border_hover = format!("rgba({}, {}, {}, 0.22)", border.0, border.1, border.2);
    let focus_border = format!("rgba({}, {}, {}, 0.65)", green.0, green.1, green.2);
    let error_border = format!("rgba({}, {}, {}, 0.85)", red.0, red.1, red.2);
    let end_border = format!("rgba({}, {}, {}, 0.35)", red.0, red.1, red.2);
    let session_menu_bg = format!("rgba({}, {}, {}, 0.98)", canvas.0, canvas.1, canvas.2);
    let radial_center = format!("rgba({}, {}, {}, 0.85)", surface_alt.0, surface_alt.1, surface_alt.2);
    let radial_edge = format!("rgba({}, {}, {}, 0.98)", canvas.0, canvas.1, canvas.2);

    format!(
r#"/* Generated by inerium-theme for {} (author: {}) */
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

passwordentry {{
    min-height: 48px; padding: 0 16px; border-radius: 12px;
    background-color: {}; color: {}; font-size: 14px;
    border: 1px solid {}; box-shadow: none; outline: none;
}}
passwordentry:focus-within {{ border-color: {}; }}
passwordentry.error {{ border-color: {}; }}
passwordentry > text > placeholder {{ color: {}; }}

.session-btn {{
    min-height: 42px; padding: 0 14px; border-radius: 12px;
    background-color: {}; background-image: none; box-shadow: none;
    border: 1px solid {};
}}
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
        theme.theme.name, theme.theme.author,
        p.text_primary,
        p.bg_canvas,
        radial_center, radial_edge,
        p.text_heading,
        p.accent_amber,
        bg_card_rgba,
        card_border,
        p.text_heading,
        p.text_muted,
        p.accent_green_bg, p.accent_green,
        p.bg_surface_alt, p.text_primary,
        input_border,
        focus_border,
        error_border,
        p.text_muted,
        p.bg_surface_alt,
        btn_border,
        btn_border_hover,
        p.text_muted,
        session_menu_bg,
        card_border,
        p.text_primary,
        p.accent_green,
        p.bg_surface_hover,
        p.text_heading,
        p.accent_green_bg,
        p.accent_green,
        p.accent_green,
        p.text_inverted,
        p.text_inverted,
        p.accent_green_border,
        p.text_muted,
        p.text_muted,
        p.accent_red,
        p.accent_amber,
        bg_end_rgba,
        end_border,
        p.accent_red,
        p.accent_red,
        p.text_inverted,
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

    // 3. GTK 3 & GTK 4 settings.ini and Libadwaita CSS
    let prefer_dark_str = if is_dark { "1" } else { "0" };
    let libadwaita_css = emit_libadwaita_css(theme);
    for gtk_ver in &["gtk-3.0", "gtk-4.0"] {
        let dir = home.join(format!(".config/{}", gtk_ver));
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(dir.join("gtk.css"), &libadwaita_css);

        let path = dir.join("settings.ini");
        if let Ok(content) = fs::read_to_string(&path) {
            let mut lines = Vec::new();
            for line in content.lines() {
                if line.starts_with("gtk-theme-name=") {
                    lines.push(format!("gtk-theme-name={}", gtk_theme));
                } else if line.starts_with("gtk-application-prefer-dark-theme=") {
                    lines.push(format!("gtk-application-prefer-dark-theme={}", prefer_dark_str));
                } else if line.starts_with("gtk-icon-theme-name=") {
                    lines.push(format!("gtk-icon-theme-name={}", icon_theme));
                } else {
                    lines.push(line.to_string());
                }
            }
            let _ = fs::write(&path, lines.join("\n") + "\n");
        }
    }

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

fn print_help() {
    println!(
        r#"inerium-theme — Менеджер и компилятор тем оформления Inerium Desktop

ИСПОЛЬЗОВАНИЕ:
    inerium-theme <КОМАНДА>

КОМАНДЫ:
    list                  Показать все доступные палитры с указанием авторов
    set <id>              Применить палитру по ID (например: inerium-light, kanagawa-dragon, kanagawa-lotus)
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

    let themes = load_available_themes();

    match args[0].as_str() {
        "list" => {
            let cur_id = get_current_theme_id();
            if args.iter().any(|a| a == "--json") {
                let list: Vec<_> = themes
                    .iter()
                    .map(|(id, t)| {
                        serde_json::json!({
                            "id": id,
                            "name": t.theme.name,
                            "author": t.theme.author,
                            "variant": t.theme.variant,
                            "description": t.theme.description.clone().unwrap_or_default(),
                            "is_active": id == &cur_id,
                            "colors": {
                                "bg": t.palette.bg_canvas,
                                "surface": t.palette.bg_surface,
                                "accent": t.palette.accent_blue,
                                "fg": t.palette.text_primary,
                            }
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&list).unwrap_or_default());
                return;
            }

            println!("Доступные палитры Inerium Desktop:\n");
            for (id, t) in &themes {
                let active_mark = if id == &cur_id { "* " } else { "  " };
                let author_str = format!("(автор: {})", t.theme.author);
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
            let theme = themes.get(&cur_id).or_else(|| themes.get("kanagawa-dragon"));
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
            let cur_theme = themes.get(&cur_id).or_else(|| themes.get("kanagawa-dragon"));
            let is_dark = cur_theme.map(|t| t.theme.variant == "dark").unwrap_or(true);

            let target_id = if is_dark {
                if themes.contains_key("inerium-light") {
                    "inerium-light"
                } else {
                    "kanagawa-lotus"
                }
            } else {
                "kanagawa-dragon"
            };

            if let Some(target) = themes.get(target_id) {
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
        "set" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите идентификатор темы. Выполните 'inerium-theme list'.");
                std::process::exit(1);
            }

            let input_id = args[1].to_lowercase();
            let target_id = match input_id.as_str() {
                "dark" | "dragon" | "черная" | "темная" | "тёмная" => "kanagawa-dragon",
                "light" | "inerium-light" | "белая" | "светлая" => "inerium-light",
                "lotus" => "kanagawa-lotus",
                "wave" => "kanagawa-wave",
                other => other,
            };

            if let Some(target) = themes.get(target_id) {
                apply_theme(target);
                println!(
                    "Установлена палитра: {} (автор: {})",
                    target.theme.name, target.theme.author
                );
                if let Some(upstream) = &target.theme.upstream {
                    println!("Оригинальный репозиторий: {}", upstream);
                }
            } else {
                eprintln!("Ошибка: тема '{}' не найдена.", target_id);
                println!("Запустите 'inerium-theme list' для просмотра всех доступных палитр.");
                std::process::exit(1);
            }
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
