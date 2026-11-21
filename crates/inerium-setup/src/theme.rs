use std::cell::RefCell;
use std::rc::Rc;

use gdk4::RGBA;
use gio::prelude::*;
use vte4::prelude::*;

#[allow(dead_code)]
pub mod palette {
    pub const BG: &str = "#181616";
    pub const SURFACE: &str = "#221f1f";
    pub const SURFACE_ALT: &str = "#282727";
    pub const FG: &str = "#c5c9c5";
    pub const FG_BRIGHT: &str = "#e6e2cc";
    pub const MUTED: &str = "#737c73";
    pub const SUBTLE: &str = "#a6a69c";
    pub const GREEN: &str = "#87a987";
    pub const BLUE: &str = "#8ba4b0";
    pub const AQUA: &str = "#8ea4a2";
    pub const YELLOW: &str = "#c4b28a";
    pub const ORANGE: &str = "#b6927b";
    pub const RED: &str = "#c4746e";
    pub const VIOLET: &str = "#8992a7";
}

thread_local! {
    static CSS_PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
    static LISTENERS: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    static FILE_MONITOR: RefCell<Option<gio::FileMonitor>> = const { RefCell::new(None) };
}

pub use inerium_theme::is_dark_theme;

pub fn vte_palette() -> (RGBA, RGBA, [RGBA; 16]) {
    let parse = |hex: &str| RGBA::parse(hex).unwrap_or(RGBA::BLACK);
    let mut fg = parse(palette::FG);
    let mut bg = parse(palette::BG);
    let mut pal = [
        parse("#181616"), // 0 black
        parse(palette::RED),
        parse(palette::GREEN),
        parse(palette::YELLOW),
        parse(palette::BLUE),
        parse(palette::VIOLET),
        parse(palette::AQUA),
        parse(palette::FG),
        parse(palette::MUTED),
        parse("#d48680"),
        parse("#9ab89a"),
        parse("#d4c49c"),
        parse("#9cb4c0"),
        parse("#9ba4b8"),
        parse("#9eb4b2"),
        parse(palette::FG_BRIGHT),
    ];

    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let kitty_theme = home.join(".config/kitty/themes/current-theme.conf");
    if let Ok(content) = std::fs::read_to_string(&kitty_theme) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                match key {
                    "foreground" => fg = parse(val),
                    "background" => bg = parse(val),
                    k if k.starts_with("color") => {
                        if let Ok(idx) = k[5..].parse::<usize>()
                            && idx < 16
                        {
                            pal[idx] = parse(val);
                        }
                    }
                    _ => {}
                }
            }
        }
    } else if !is_dark_theme() {
        fg = parse("#545464");
        bg = parse("#f2ecbc");
        pal[0] = parse("#e5dfc5");
        pal[7] = parse("#545464");
        pal[15] = parse("#43436c");
    }

    (fg, bg, pal)
}

pub fn update_terminal_palette(terminal: &vte4::Terminal) {
    let (fg, bg, pal) = vte_palette();
    let pal_refs: Vec<&gdk4::RGBA> = pal.iter().collect();
    terminal.set_colors(Some(&fg), Some(&bg), &pal_refs);
}

pub use inerium_theme::{
    DRAGON_VARS as FALLBACK_DRAGON_VARS,
    LOTUS_VARS as FALLBACK_LIGHT_VARS,
    THEME_ALIASES,
};

pub fn get_setup_css() -> String {
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let vars_path = home.join(".config/inerium/theme-vars.css");
    let is_dark = is_dark_theme();
    let vars = if let Ok(s) = std::fs::read_to_string(&vars_path) {
        if !s.trim().is_empty() {
            s
        } else if is_dark {
            FALLBACK_DRAGON_VARS.to_string()
        } else {
            FALLBACK_LIGHT_VARS.to_string()
        }
    } else if is_dark {
        FALLBACK_DRAGON_VARS.to_string()
    } else {
        FALLBACK_LIGHT_VARS.to_string()
    };
    format!("{vars}\n{THEME_ALIASES}\n{SETUP_CSS_RULES}")
}

pub fn init_theme(provider: &gtk4::CssProvider) {
    CSS_PROVIDER.with(|p| {
        *p.borrow_mut() = Some(provider.clone());
    });
    apply_theme(provider);

    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    let cfg_dir = gio::File::for_path(home.join(".config/inerium"));
    if let Ok(mon) = cfg_dir.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
        mon.connect_changed(move |_, file, _, event| {
            if matches!(
                event,
                gio::FileMonitorEvent::Changed
                    | gio::FileMonitorEvent::Created
                    | gio::FileMonitorEvent::ChangesDoneHint
            ) && let Some(name) = file.basename() {
                let s = name.to_string_lossy();
                if s == "theme-vars.css" || s == "current.json" {
                    reload_theme();
                }
            }
        });
        FILE_MONITOR.with(|m| {
            *m.borrow_mut() = Some(mon);
        });
    }
}

pub fn apply_theme(provider: &gtk4::CssProvider) {
    let is_dark = is_dark_theme();
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(is_dark);
    }
    provider.load_from_data(&get_setup_css());
    notify_theme_changed();
}

pub fn reload_theme() {
    CSS_PROVIDER.with(|p| {
        if let Some(ref provider) = *p.borrow() {
            apply_theme(provider);
        }
    });
}

pub fn add_theme_listener<F: Fn() + 'static>(f: F) -> Rc<dyn Fn()> {
    let rc: Rc<dyn Fn()> = Rc::new(f);
    LISTENERS.with(|l| l.borrow_mut().push(rc.clone()));
    rc
}

fn notify_theme_changed() {
    LISTENERS.with(|l| {
        for listener in l.borrow().iter() {
            listener();
        }
    });
}

pub const SETUP_CSS_RULES: &str = r#"
window.setup-window {
    background-color: @cc_bg;
    color: @cc_fg;
    font-family: "JetBrainsMono Nerd Font", monospace;
}

.setup-shell {
    background-color: @cc_bg;
    color: @cc_fg;
    padding: 18px 20px 16px 20px;
}

.setup-header {
    padding-bottom: 12px;
    border-bottom: 1px solid @cc_line;
}

.setup-brand-ico {
    min-width: 38px;
    min-height: 38px;
    border-radius: 11px;
    background-color: @cc_green_bg;
    border: 1px solid @cc_green_border;
    color: @cc_green;
    font-size: 18px;
}

.setup-title {
    font-size: 16px;
    font-weight: 800;
    color: @cc_fg_bright;
}

.setup-subtitle {
    font-size: 11.5px;
    color: @cc_fg_muted;
}

/* Navigation Tabs */
.setup-tabs {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 12px;
    padding: 4px;
}

.setup-tab-btn {
    min-height: 34px;
    padding: 6px 14px;
    border-radius: 9px;
    border: none;
    background-image: none;
    background-color: transparent;
    box-shadow: none;
    color: @cc_fg_subtle;
    font-size: 12px;
    font-weight: 600;
    transition: background-color 140ms ease, color 140ms ease;
}

.setup-tab-btn:hover {
    background-image: none;
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}

.setup-tab-btn.active {
    background-image: none;
    background-color: @cc_green_bg;
    color: @cc_fg_bright;
}

.setup-badge {
    min-width: 18px;
    min-height: 18px;
    padding: 0 6px;
    border-radius: 99px;
    background-color: @cc_amber_bg;
    border: 1px solid @cc_amber_border;
    color: @cc_amber;
    font-size: 10.5px;
    font-weight: 700;
}

/* Cards & Sections */
.setup-card {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 14px;
    padding: 14px 16px;
}

.setup-subcard {
    background-color: @cc_surface_active;
    border: 1px solid @cc_line;
    border-radius: 12px;
    padding: 12px 14px;
}

.section-label {
    font-size: 11px;
    font-weight: 700;
    color: @cc_fg_muted;
    letter-spacing: 0.8px;
}

.card-title {
    font-size: 13.5px;
    font-weight: 700;
    color: @cc_fg_bright;
}

.card-meta {
    font-size: 11.5px;
    color: @cc_fg_subtle;
}

.card-muted {
    font-size: 11px;
    color: @cc_fg_muted;
}

/* Buttons */
.btn-primary {
    min-height: 34px;
    padding: 6px 14px;
    border-radius: 10px;
    border: 1px solid @cc_green_border;
    background-image: none;
    background-color: @cc_green;
    box-shadow: none;
    color: @cc_fg_on_accent;
    font-size: 12px;
    font-weight: 700;
}
.btn-primary label {
    color: @cc_fg_on_accent;
}
.btn-primary:hover {
    background-image: none;
    opacity: 0.9;
}
.btn-primary:disabled {
    opacity: 0.55;
}

.btn-secondary {
    min-height: 34px;
    padding: 6px 14px;
    border-radius: 10px;
    border: 1px solid @cc_line;
    background-image: none;
    background-color: @cc_surface_hover;
    box-shadow: none;
    color: @cc_fg;
    font-size: 12px;
    font-weight: 600;
}
.btn-secondary:hover {
    background-image: none;
    background-color: @cc_surface_active;
    color: @cc_fg_bright;
}

.btn-danger {
    min-height: 34px;
    padding: 6px 14px;
    border-radius: 10px;
    border: 1px solid @cc_red_border;
    background-image: none;
    background-color: @cc_red_bg;
    box-shadow: none;
    color: @cc_red;
    font-size: 12px;
    font-weight: 600;
}
.btn-danger label {
    color: @cc_red;
}
.btn-danger:hover {
    background-image: none;
    background-color: @cc_red;
    color: @cc_fg_on_accent;
}
.btn-danger:hover label {
    color: @cc_fg_on_accent;
}

.seg-group {
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    border-radius: 10px;
    padding: 3px;
}

.seg-btn {
    min-height: 28px;
    min-width: 30px;
    padding: 4px 10px;
    border-radius: 8px;
    border: none;
    background-image: none;
    background-color: transparent;
    box-shadow: none;
    color: @cc_fg_subtle;
    font-size: 12px;
    font-weight: 600;
}
.seg-btn:hover {
    background-image: none;
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.seg-btn.active {
    background-image: none;
    background-color: @cc_green_bg;
    color: @cc_fg_bright;
}

.update-btn {
    min-height: 28px;
    padding: 4px 12px;
    border-radius: 8px;
    border: 1px solid @cc_green_border;
    background-color: @cc_green_bg;
    color: @cc_green;
    font-size: 12px;
    font-weight: 600;
}
.update-btn:hover {
    background-color: @cc_green;
    color: @cc_fg_on_accent;
}

/* Inputs & Dropdowns */
entry.setup-entry {
    min-height: 34px;
    padding: 0 10px;
    border-radius: 9px;
    background-color: @cc_bg;
    background-image: none;
    color: @cc_fg_bright;
    border: 1px solid @cc_line;
    box-shadow: none;
    font-size: 12px;
}
entry.setup-entry:focus-within {
    border-color: @cc_green;
}

dropdown.setup-dropdown > button {
    min-height: 34px;
    padding: 4px 10px;
    border-radius: 9px;
    background-color: @cc_bg;
    background-image: none;
    color: @cc_fg_bright;
    border: 1px solid @cc_line;
    box-shadow: none;
    font-size: 12px;
}
dropdown.setup-dropdown > button:hover {
    background-image: none;
    border-color: @cc_line_light;
}

popover.background > contents {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 10px;
    color: @cc_fg;
}

switch.setup-switch {
    background-image: none;
    background-color: @cc_surface_active;
    border: 1px solid @cc_line;
    border-radius: 99px;
    min-width: 44px;
    min-height: 24px;
}
switch.setup-switch:checked {
    background-image: none;
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
}
switch.setup-switch slider {
    background-image: none;
    background-color: @cc_fg_bright;
    border-radius: 99px;
    min-width: 18px;
    min-height: 18px;
}

/* Wallpaper Cards */
.wall-card {
    background-color: @cc_surface;
    background-image: none;
    border: 1px solid @cc_line;
    border-radius: 12px;
    padding: 6px;
    box-shadow: none;
}
.wall-card:hover {
    background-image: none;
    background-color: @cc_surface_hover;
    border-color: @cc_green_border;
}
.wall-card.active {
    background-image: none;
    border: 2px solid @cc_green;
    background-color: @cc_green_bg;
}
.wall-thumb {
    border-radius: 8px;
}
.wall-name {
    font-size: 10.5px;
    color: @cc_fg;
}

/* Monitor Layout Preview */
.layout-preview {
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    border-radius: 10px;
    padding: 8px 12px;
}
.mon-box {
    background-color: @cc_blue_bg;
    border: 1px solid @cc_blue_border;
    border-radius: 8px;
    padding: 6px 10px;
}
.mon-box.focused {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
}

/* Status Pills */
.pill {
    padding: 3px 9px;
    border-radius: 99px;
    font-size: 10.5px;
    font-weight: 700;
}
.pill-ok {
    background-color: @cc_green_bg;
    border: 1px solid @cc_green_border;
    color: @cc_green;
}
.pill-pending {
    background-color: @cc_amber_bg;
    border: 1px solid @cc_amber_border;
    color: @cc_amber;
}
.pill-deferred {
    background-color: @cc_line;
    border: 1px solid @cc_line;
    color: @cc_fg_subtle;
}

/* VTE Terminal Drawer */
.vte-frame {
    background-color: @cc_bg;
    border: 1px solid @cc_green_border;
    border-radius: 12px;
    padding: 10px;
}

/* Footer & Toast */
.setup-footer {
    padding-top: 10px;
    border-top: 1px solid @cc_line;
}
.status-msg {
    font-size: 11.5px;
    color: @cc_amber;
}
.status-msg.ok {
    color: @cc_green;
}
.status-msg.err {
    color: @cc_red;
}
"#;

#[allow(dead_code)]
pub const SETUP_CSS: &str = SETUP_CSS_RULES;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_setup_css_contains_semantic_tokens() {
        let css = get_setup_css();
        assert!(!css.is_empty());
        assert!(css.contains("@define-color cc_bg"));
        assert!(css.contains("window.setup-window"));
        assert!(css.contains(".setup-card"));
        assert!(css.contains(".btn-primary"));
    }

    #[test]
    fn test_vte_palette_returns_valid_palette() {
        let (fg, bg, pal) = vte_palette();
        assert_eq!(pal.len(), 16);
        // Ensure colors are initialized and not all transparent
        assert!(fg.alpha() > 0.0);
        assert!(bg.alpha() > 0.0);
        assert!(pal[0].alpha() > 0.0);
    }
}
