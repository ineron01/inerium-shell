#[allow(dead_code)]
pub mod palette {
    pub const BG: &str = "rgba(24, 22, 22, 0.97)";
    pub const SURFACE: &str = "#282727";
    pub const LINE: &str = "rgba(197, 201, 197, 0.12)";
    pub const HOVER: &str = "rgba(197, 201, 197, 0.08)";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

pub fn detect_theme() -> ThemeMode {
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

pub fn get_theme_css(mode: ThemeMode) -> String {
    let vars = match mode {
        ThemeMode::Dark => DRAGON_VARS,
        ThemeMode::Light => LOTUS_VARS,
    };
    format!("{}\n{}", vars, POPUP_CSS_RULES)
}

pub const DRAGON_VARS: &str = r#"
@define-color cc_bg               rgba(24, 22, 22, 0.97);
@define-color cc_surface          #221f1f;
@define-color cc_surface_hover    #2d2a2a;
@define-color cc_surface_active   #363434;
@define-color cc_line             rgba(197, 201, 197, 0.10);
@define-color cc_line_light       rgba(197, 201, 197, 0.20);
@define-color cc_fg               #c5c9c5;
@define-color cc_fg_bright        #e6e2cc;
@define-color cc_fg_muted         #737c73;
@define-color cc_fg_subtle        #a6a69c;
@define-color cc_fg_on_accent     #181616;

@define-color cc_green            #87a987;
@define-color cc_green_bg         rgba(135, 169, 135, 0.16);
@define-color cc_green_border     rgba(135, 169, 135, 0.35);

@define-color cc_blue             #8ba4b0;
@define-color cc_blue_bg          rgba(139, 164, 176, 0.16);
@define-color cc_blue_border      rgba(139, 164, 176, 0.38);

@define-color cc_amber            #c4b28a;
@define-color cc_amber_bg         rgba(196, 178, 138, 0.15);
@define-color cc_amber_border     rgba(196, 178, 138, 0.38);

@define-color cc_red              #c4746e;
@define-color cc_red_bg           rgba(196, 116, 110, 0.15);
@define-color cc_red_border       rgba(196, 116, 110, 0.38);

@define-color cc_aqua             #8ea4a2;
@define-color cc_aqua_bg          rgba(142, 164, 162, 0.16);
@define-color cc_aqua_border      rgba(142, 164, 162, 0.38);

@define-color cc_violet           #8992a7;
@define-color cc_violet_bg        rgba(137, 146, 167, 0.16);
@define-color cc_violet_border    rgba(137, 146, 167, 0.38);

@define-color cc_shadow           rgba(0, 0, 0, 0.85);
@define-color cc_modal_backdrop   rgba(18, 16, 16, 0.48);
@define-color cc_slider_trough    rgba(197, 201, 197, 0.10);
@define-color cc_slider_thumb     #e6e2cc;
"#;

pub const LOTUS_VARS: &str = r#"
@define-color cc_bg               rgba(242, 236, 188, 0.98);
@define-color cc_surface          #e5dfc5;
@define-color cc_surface_hover    #ded7bc;
@define-color cc_surface_active   #d5cdb0;
@define-color cc_line             rgba(84, 84, 100, 0.14);
@define-color cc_line_light       rgba(84, 84, 100, 0.24);
@define-color cc_fg               #545464;
@define-color cc_fg_bright        #43436c;
@define-color cc_fg_muted         #8a8980;
@define-color cc_fg_subtle        #716e61;
@define-color cc_fg_on_accent     #f2ecbc;

@define-color cc_green            #6f894e;
@define-color cc_green_bg         rgba(111, 137, 78, 0.18);
@define-color cc_green_border     rgba(111, 137, 78, 0.40);

@define-color cc_blue             #4d699b;
@define-color cc_blue_bg          rgba(77, 105, 155, 0.18);
@define-color cc_blue_border      rgba(77, 105, 155, 0.40);

@define-color cc_amber            #cc6d00;
@define-color cc_amber_bg         rgba(204, 109, 0, 0.18);
@define-color cc_amber_border     rgba(204, 109, 0, 0.40);

@define-color cc_red              #c84053;
@define-color cc_red_bg           rgba(200, 64, 83, 0.18);
@define-color cc_red_border       rgba(200, 64, 83, 0.40);

@define-color cc_aqua             #597b75;
@define-color cc_aqua_bg          rgba(89, 123, 117, 0.18);
@define-color cc_aqua_border      rgba(89, 123, 117, 0.40);

@define-color cc_violet           #8a7ea8;
@define-color cc_violet_bg        rgba(138, 126, 168, 0.18);
@define-color cc_violet_border    rgba(138, 126, 168, 0.40);

@define-color cc_shadow           rgba(84, 84, 100, 0.18);
@define-color cc_modal_backdrop   rgba(84, 84, 100, 0.35);
@define-color cc_slider_trough    rgba(84, 84, 100, 0.15);
@define-color cc_slider_thumb     #43436c;
"#;

pub const POPUP_CSS_RULES: &str = r#"
window.popup-window {
    background: transparent;
}

.popup-container, .cc {
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    border-radius: 16px;
    padding: 14px;
    box-shadow: 0 12px 36px @cc_shadow;
    font-family: "JetBrainsMono Nerd Font", monospace;
    color: @cc_fg;
}

/* Header */
.cc-head {
    margin-bottom: 6px;
}
.cc-host {
    font-size: 14px;
    font-weight: 700;
    color: @cc_fg_bright;
}
.cc-meta {
    font-size: 11px;
    color: @cc_fg_muted;
}
.cc-meta-accent {
    font-size: 11px;
    color: @cc_green;
    font-weight: 600;
}
.cc-meta-btn {
    padding: 1px 6px;
    margin: 0;
    border-radius: 6px;
    border: none;
    background-image: none;
    box-shadow: none;
    background-color: transparent;
    color: @cc_fg_muted;
    font-size: 11px;
    font-weight: 500;
    transition: background-color 150ms ease, color 150ms ease;
}
.cc-meta-btn:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.cc-meta-btn.has-updates {
    color: @cc_aqua;
    background-color: @cc_aqua_bg;
}
.cc-meta-btn.has-updates:hover {
    color: @cc_fg_bright;
    background-color: @cc_aqua_border;
}
.cc-meta-btn-lbl {
    font-size: 11px;
    font-weight: 600;
}
.cc-icon-btn {
    min-width: 32px;
    min-height: 32px;
    padding: 0;
    border-radius: 10px;
    border: none;
    background-image: none;
    box-shadow: none;
    background-color: @cc_surface_hover;
    color: @cc_fg_subtle;
    font-size: 14px;
}
.cc-icon-btn:hover {
    background-color: @cc_surface_active;
    color: @cc_fg_bright;
}
.cc-icon-btn.danger {
    color: @cc_red;
}
.cc-icon-btn.danger:hover {
    background-color: @cc_red_bg;
    color: @cc_fg_bright;
}
.cc-icon-btn.amber.on {
    color: @cc_amber;
    background-color: @cc_amber_bg;
}

/* Dedicated Back Button */
.back-btn {
    min-width: 72px;
    min-height: 28px;
    padding: 4px 12px;
    border-radius: 8px;
    border: 1px solid @cc_line;
    background-image: none;
    box-shadow: none;
    background-color: @cc_surface_hover;
    color: @cc_green;
    font-size: 12px;
    font-weight: 600;
}
.back-btn:hover {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
    color: @cc_fg_bright;
}

/* Fan Telemetry Badge */
.cc-fan-badge {
    padding: 4px 8px;
    border-radius: 8px;
    background-color: @cc_surface_hover;
    border: 1px solid @cc_line;
    font-size: 10.5px;
    color: @cc_green;
    font-weight: 600;
}

/* Quick Action Tiles */
.cc-tile {
    border-radius: 14px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
}
.cc-tile-main, .cc-tile-more {
    border: none;
    background: none;
    box-shadow: none;
    border-radius: 14px;
    padding: 8px 10px;
}
.cc-tile-more {
    padding: 8px 10px;
    color: @cc_fg_muted;
    font-size: 15px;
}
.cc-tile-main:hover, .cc-tile-more:hover {
    background-color: @cc_surface_hover;
}
.cc-tile-ico {
    min-width: 32px;
    min-height: 32px;
    border-radius: 10px;
    background-color: @cc_surface_hover;
    color: @cc_fg_subtle;
    font-size: 15px;
}
.cc-tile-title {
    font-size: 12px;
    font-weight: 700;
    color: @cc_fg;
}
.cc-tile-sub {
    font-size: 10.5px;
    color: @cc_fg_muted;
}

.cc-tile.on {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
}
.cc-tile.on .cc-tile-ico {
    background-color: @cc_green;
    color: @cc_fg_on_accent;
}
.cc-tile.on .cc-tile-sub {
    color: @cc_fg_subtle;
}

.cc-tile.on.blue {
    background-color: @cc_blue_bg;
    border-color: @cc_blue_border;
}
.cc-tile.on.blue .cc-tile-ico {
    background-color: @cc_blue;
    color: @cc_fg_on_accent;
}

.cc-tile.on.amber {
    background-color: @cc_amber_bg;
    border-color: @cc_amber_border;
}
.cc-tile.on.amber .cc-tile-ico {
    background-color: @cc_amber;
    color: @cc_fg_on_accent;
}

.cc-tile.on.red {
    background-color: @cc_red_bg;
    border-color: @cc_red_border;
}
.cc-tile.on.red .cc-tile-ico {
    background-color: @cc_red;
    color: @cc_fg_on_accent;
}

.cc-tile.on.purple {
    background-color: @cc_violet_bg;
    border-color: @cc_violet_border;
}
.cc-tile.on.purple .cc-tile-ico {
    background-color: @cc_violet;
    color: @cc_fg_on_accent;
}

/* Sliders Card */
.cc-card {
    border-radius: 14px;
    padding: 8px 10px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
}
.cc-label {
    font-size: 10.5px;
    font-weight: 700;
    color: @cc_fg_muted;
    letter-spacing: 1px;
}
.cc-slider-ico {
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    border: none;
    border-radius: 8px;
    background: none;
    color: @cc_fg_subtle;
    font-size: 14px;
}
.cc-slider-ico:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.cc-slider-ico.muted {
    color: @cc_red;
}
.cc-pct {
    font-size: 11px;
    color: @cc_fg_subtle;
    min-width: 38px;
}
.cc-scale trough {
    min-height: 6px;
    border-radius: 99px;
    background-color: @cc_slider_trough;
}
.cc-scale highlight {
    border-radius: 99px;
    background-color: @cc_green;
}
.cc-scale.amber highlight {
    background-color: @cc_amber;
}
.cc-scale slider {
    min-width: 14px;
    min-height: 14px;
    margin: -5px;
    border-radius: 99px;
    background-color: @cc_slider_thumb;
    border: none;
    box-shadow: none;
}

/* Segmented Buttons */
.cc-seg {
    border-radius: 12px;
    padding: 3px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
}
.cc-seg button {
    border: none;
    background: none;
    box-shadow: none;
    border-radius: 9px;
    padding: 6px 4px;
    font-size: 11.5px;
    font-weight: 600;
    color: @cc_fg_subtle;
}
.cc-seg button:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.cc-seg button.active {
    background-color: @cc_green_bg;
    color: @cc_fg_bright;
}
.cc-seg button.active.eco {
    background-color: @cc_aqua_bg;
}
.cc-seg button.active.turbo {
    background-color: @cc_red_bg;
}

/* Action Chips */
.cc-chip {
    border-radius: 10px;
    padding: 6px 10px;
    border: 1px solid @cc_line;
    background-color: @cc_surface;
    box-shadow: none;
    font-size: 11px;
    font-weight: 600;
    color: @cc_fg_subtle;
}
.cc-chip:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.cc-chip.on {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
    color: @cc_fg_bright;
}
.cc-chip.on.red {
    background-color: @cc_red_bg;
    border-color: @cc_red_border;
    color: @cc_fg_bright;
}
.cc-chip:disabled {
    opacity: 0.70;
    color: @cc_amber;
}

.cc-foot {
    font-size: 10.5px;
    color: @cc_fg_muted;
}

.cc-status {
    font-size: 11px;
    color: @cc_amber;
}
.cc-status.ok {
    color: @cc_green;
}
.cc-status.err {
    color: @cc_red;
}

/* Card list views (Audio, Bluetooth, Network) */
.list-scroll {
}
.card-item {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 9px;
    padding: 6px 10px;
    margin-bottom: 4px;
}
.card-item:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_line_light;
}
.card-item.active {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
}
.card-item:disabled {
    opacity: 0.70;
}
.card-icon {
    font-size: 15px;
    margin-right: 8px;
    color: @cc_green;
}
.card-title {
    font-size: 11.5px;
    font-weight: 600;
    color: @cc_fg;
}
.card-sub {
    font-size: 9.5px;
    color: @cc_fg_muted;
}
.card-badge {
    font-size: 9.5px;
    font-weight: 600;
    border-radius: 6px;
    padding: 1.5px 6px;
    background-color: @cc_green_bg;
    color: @cc_green;
}
.card-badge.active {
    background-color: @cc_green_bg;
    color: @cc_green;
}
.card-badge.saved {
    background-color: @cc_blue_bg;
    color: @cc_blue;
}
.auth-error {
    font-size: 11px;
    color: @cc_red;
    font-weight: 600;
    margin-top: 4px;
    margin-bottom: 2px;
}

.auth-input {
    min-height: 36px;
    padding: 6px 10px;
    border-radius: 9px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    color: @cc_fg_bright;
}
.auth-input:focus {
    background-color: @cc_surface_hover;
    border-color: @cc_green_border;
    color: @cc_fg_bright;
}

/* Modal Power Menu & Fullscreen Backdrop */
window.power-window {
    background-color: @cc_modal_backdrop;
}

.power-backdrop {
    background: transparent;
}

.power-modal {
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    border-radius: 24px;
    padding: 32px 36px;
    box-shadow: 0 24px 64px @cc_shadow;
}

.power-title {
    font-size: 18px;
    font-weight: 700;
    color: @cc_fg_bright;
    margin-bottom: 2px;
}

.power-subtitle {
    font-size: 12px;
    color: @cc_fg_muted;
    margin-bottom: 8px;
}

.power-card {
    min-width: 130px;
    min-height: 135px;
    padding: 16px 12px;
    border-radius: 18px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    color: @cc_fg;
    transition: all 0.2s ease;
}

.power-card:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_green_border;
    color: @cc_fg_bright;
    box-shadow: 0 4px 16px @cc_shadow;
}

.power-card.danger:hover {
    border-color: @cc_red_border;
}

.power-card-ico {
    font-size: 42px;
    margin-bottom: 8px;
}

.power-card-label {
    font-size: 13px;
    font-weight: 600;
    color: @cc_fg;
}

.power-card:hover .power-card-label {
    color: @cc_fg_bright;
}

.power-card-key {
    font-size: 11px;
    font-weight: 700;
    color: @cc_blue;
    background-color: @cc_blue_bg;
    border: 1px solid @cc_blue_border;
    border-radius: 6px;
    min-width: 32px;
    min-height: 20px;
    padding: 2px 0px;
    margin-top: 6px;
}

.power-card:hover .power-card-key {
    color: @cc_fg_bright;
    background-color: @cc_blue_bg;
    border-color: @cc_blue_border;
}

.power-card.danger .power-card-key {
    color: @cc_red;
    background-color: @cc_red_bg;
    border: 1px solid @cc_red_border;
}

.power-card.danger:hover .power-card-key {
    color: @cc_fg_bright;
    background-color: @cc_red_bg;
    border-color: @cc_red_border;
}

.power-cancel-btn {
    min-width: 140px;
    min-height: 36px;
    font-size: 13px;
    border-radius: 10px;
    margin-top: 8px;
    transition: all 0.2s ease;
}

.power-cancel-btn:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
    border-color: @cc_line_light;
}

/* Wallpaper Gallery */
.wall-card {
    min-width: 140px;
    border-radius: 12px;
    background-color: @cc_surface;
    border: 2px solid @cc_line;
    padding: 6px;
    transition: all 0.2s ease;
}
.wall-card:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_green_border;
}
.wall-card.active {
    border-color: @cc_green;
    background-color: @cc_green_bg;
}
.wall-thumb {
    border-radius: 8px;
}
.wall-caption {
    font-size: 10px;
    font-weight: 600;
    color: @cc_fg_subtle;
    margin-top: 4px;
}
.wall-card.active .wall-caption {
    color: @cc_fg_bright;
}

/* Media Player Card & Controls */
.media-badge {
    font-size: 11px;
    font-weight: 600;
    color: @cc_green;
    background-color: @cc_green_bg;
    border: 1px solid @cc_green_border;
    border-radius: 8px;
    padding: 3px 8px;
}
.media-status-pill {
    font-size: 10.5px;
    font-weight: 600;
    color: @cc_green;
    background-color: @cc_green_bg;
    border-radius: 8px;
    padding: 3px 8px;
}
.media-status-pill.paused {
    color: @cc_fg_subtle;
    background-color: @cc_surface_hover;
}
.media-card {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 14px;
    padding: 10px 12px;
}
.media-art-box {
    min-width: 64px;
    min-height: 64px;
    border-radius: 10px;
    background-color: @cc_surface_hover;
    border: 1px solid @cc_line;
}
.media-art {
    border-radius: 10px;
}
.media-art-fallback {
    font-size: 26px;
    color: @cc_green;
}
.media-title {
    font-size: 13px;
    font-weight: 700;
    color: @cc_fg_bright;
}
.media-artist {
    font-size: 11px;
    color: @cc_fg_muted;
}
.media-progress {
    margin-top: 4px;
    margin-bottom: 2px;
}
.media-progress trough {
    min-height: 5px;
    border-radius: 3px;
    background-color: @cc_slider_trough;
    border: none;
}
.media-progress progress {
    min-height: 5px;
    border-radius: 3px;
    background-color: @cc_green;
}
.media-time {
    font-size: 10px;
    color: @cc_fg_muted;
    font-family: monospace;
}
.media-btn {
    min-width: 34px;
    min-height: 34px;
    border-radius: 10px;
    border: none;
    background-color: @cc_surface_hover;
    color: @cc_fg;
    font-size: 15px;
}
.media-btn:hover {
    background-color: @cc_surface_active;
    color: @cc_fg_bright;
}
.play-btn {
    min-width: 40px;
    min-height: 40px;
    border-radius: 20px;
    border: none;
    background-color: @cc_green;
    color: @cc_fg_on_accent;
    font-size: 16px;
    font-weight: 700;
}
.play-btn:hover {
    background-color: @cc_green_border;
    color: @cc_fg_on_accent;
}

/* Thin Scrollbar */
scrollbar {
    background: transparent;
    border: none;
}
scrollbar slider {
    min-width: 4px;
    min-height: 4px;
    border-radius: 4px;
    background-color: @cc_surface_active;
}
scrollbar slider:hover {
    background-color: @cc_green_border;
}
"#;

#[allow(dead_code)]
pub const POPUP_CSS: &str = POPUP_CSS_RULES;
