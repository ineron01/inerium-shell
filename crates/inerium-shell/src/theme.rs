pub use inerium_theme::{
    detect_theme, ThemeMode, DRAGON_VARS,
    LOTUS_VARS,
};

pub fn get_theme_css(mode: ThemeMode) -> String {
    let home = inerium_theme::get_home();
    let vars_path = home.join(".config/inerium/theme-vars.css");
    let vars = if let Ok(s) = std::fs::read_to_string(&vars_path) {
        if !s.trim().is_empty() {
            s
        } else {
            match mode {
                ThemeMode::Dark => DRAGON_VARS.to_string(),
                ThemeMode::Light => LOTUS_VARS.to_string(),
            }
        }
    } else {
        match mode {
            ThemeMode::Dark => DRAGON_VARS.to_string(),
            ThemeMode::Light => LOTUS_VARS.to_string(),
        }
    };
    format!("{}\n{}", vars, POPUP_CSS_RULES)
}

pub const POPUP_CSS_RULES: &str = r#"
window.popup-window,
window.popup-window.background,
window.popup-window decoration {
    background: transparent;
    background-color: transparent;
    border: none;
    box-shadow: none;
}

.popup-container, .cc {
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    border-radius: 16px;
    padding: 14px;
    box-shadow: none;
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
    color: @cc_primary;
    font-size: 12px;
    font-weight: 600;
}
.back-btn:hover {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
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
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
}
.cc-tile.on .cc-tile-ico {
    background-color: @cc_primary;
    color: @cc_fg_on_accent;
}
.cc-tile.on .cc-tile-sub {
    color: @cc_fg_subtle;
}

.cc-tile.on.green {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
}
.cc-tile.on.green .cc-tile-ico {
    background-color: @cc_green;
    color: @cc_fg_on_accent;
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
    background-color: @cc_slider_fill;
}
.cc-scale.amber highlight {
    background-color: @cc_slider_fill_secondary;
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
    padding: 6px 8px;
    font-size: 11.5px;
    font-weight: 600;
    color: @cc_fg_subtle;
}
.cc-seg.cc-seg-wide button {
    padding: 5px 9px;
    min-width: 56px;
}
.cc-seg button:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.cc-seg button.active {
    background-color: @cc_primary_bg;
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
.cc-chip.active {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
    color: @cc_fg_bright;
}
.cc-chip:disabled {
    opacity: 0.70;
    color: @cc_amber;
}

/* Filter Pills */
.filter-pill {
    border-radius: 99px;
    padding: 4px 10px;
    border: 1px solid @cc_line;
    background-color: @cc_surface;
    font-size: 11px;
    font-weight: 600;
    color: @cc_fg_subtle;
    box-shadow: none;
    transition: all 120ms ease;
}
.filter-pill:hover {
    background-color: @cc_surface_hover;
    color: @cc_fg_bright;
}
.filter-pill.active {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
    color: @cc_fg_bright;
}

.cc-sep {
    background-color: @cc_line;
    min-height: 1px;
    margin: 4px 0;
}
.section-header-box {
    padding: 2px 0;
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
    padding: 7px 10px;
    margin-bottom: 5px;
    transition: background-color 100ms ease, border-color 100ms ease;
}
.card-item:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_line_light;
}
.card-item.active {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
}
.card-item.expanded {
    border-color: @cc_line_light;
    background-color: @cc_surface_hover;
}
.card-item.active.expanded {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
}
.card-item:disabled {
    opacity: 0.70;
}
.card-icon {
    font-size: 15px;
    margin-right: 8px;
    color: @cc_fg_subtle;
    transition: color 150ms ease;
}
.card-icon.active {
    color: @cc_green;
}
.card-icon.connected {
    color: @cc_blue;
}
.card-icon.signal-high {
    color: @cc_green;
}
.card-icon.signal-mid {
    color: @cc_aqua;
}
.card-icon.signal-low {
    color: @cc_amber;
}
.card-icon.audio {
    color: @cc_aqua;
}
.card-icon.input {
    color: @cc_violet;
}
.card-icon.phone,
.card-icon.computer {
    color: @cc_blue;
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
.card-sub.warning {
    color: @cc_amber;
    font-weight: 600;
}
.card-badge {
    font-size: 9.5px;
    font-weight: 600;
    border-radius: 6px;
    padding: 1.5px 6px;
    background-color: @cc_surface_hover;
    color: @cc_fg_subtle;
    border: 1px solid @cc_line;
}
.card-badge.active {
    background-color: @cc_green_bg;
    border-color: @cc_green_border;
    color: @cc_green;
}
.card-badge.connected {
    background-color: @cc_blue_bg;
    border-color: @cc_blue_border;
    color: @cc_blue;
}
.card-badge.saved,
.card-badge.paired {
    background-color: @cc_surface_hover;
    border-color: @cc_line;
    color: @cc_fg_subtle;
}
.card-badge.new {
    background-color: @cc_amber_bg;
    border-color: @cc_amber_border;
    color: @cc_amber;
}

/* Inline Card Drawer & Quick Actions */
.card-chevron {
    background: transparent;
    border: none;
    box-shadow: none;
    padding: 2px 4px;
    margin: 0;
    color: @cc_fg_muted;
    font-size: 11px;
    border-radius: 5px;
    min-height: 20px;
    min-width: 20px;
}
.card-chevron:hover {
    background-color: alpha(@cc_fg, 0.08);
    color: @cc_fg_bright;
}
.card-drawer-sep {
    margin: 6px 0 6px 0;
    background-color: @cc_line;
}
.card-drawer-inner {
    padding: 3px 0 2px 0;
}
.drawer-chip {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 8px;
    padding: 5px 4px;
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    color: @cc_fg_subtle;
    box-shadow: none;
}
.drawer-chip:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_line_light;
    color: @cc_fg_bright;
}
.drawer-chip.on {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
    color: @cc_primary;
}
.drawer-chip.on:hover {
    background-color: @cc_primary_bg;
    border-color: @cc_primary;
    color: @cc_fg_bright;
}
.drawer-chip.danger {
    color: @cc_red;
    border-color: alpha(@cc_red, 0.35);
}
.drawer-chip.danger:hover {
    background-color: @cc_red_bg;
    border-color: @cc_red;
    color: @cc_fg_bright;
}
.drawer-chip.info {
    color: @cc_fg_muted;
    border-color: transparent;
    background-color: alpha(@cc_line, 0.08);
}
.drawer-chip label {
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
}
.auth-error {
    font-size: 10.5px;
    color: @cc_red;
    font-weight: 600;
    margin-top: 3px;
    margin-bottom: 2px;
}

.auth-ca-check {
    font-size: 11px;
    font-weight: 500;
    color: @cc_fg_subtle;
    margin: 2px 0 2px 2px;
}
.auth-ca-check:hover {
    color: @cc_fg_bright;
}
.auth-ca-check check {
    border-radius: 4px;
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    min-width: 14px;
    min-height: 14px;
    margin-right: 6px;
}
.auth-ca-check check:checked {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
    color: @cc_primary;
}

.drawer-entry {
    min-height: 28px;
    padding: 4px 8px;
    border-radius: 7px;
    background-color: @cc_bg;
    border: 1px solid @cc_line;
    color: @cc_fg_bright;
    font-size: 11px;
    font-family: inherit;
}
.drawer-entry:focus-within,
.drawer-entry:focus {
    background-color: @cc_surface;
    border-color: @cc_primary_border;
    color: @cc_fg_bright;
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
    border-color: @cc_primary_border;
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
    border-color: @cc_primary_border;
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
.wall-btn {
    padding: 0;
    border: none;
    background: transparent;
    box-shadow: none;
    min-width: 128px;
    min-height: 72px;
}
.wall-card {
    min-width: 140px;
    border-radius: 12px;
    background-color: @cc_surface;
    border: 1.5px solid @cc_line;
    padding: 6px;
    transition: all 0.2s ease;
}
.wall-card:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_line_light;
}
.wall-card.active {
    border-color: @cc_primary;
    background-color: @cc_primary_bg;
}
.wall-thumb {
    border-radius: 8px;
    min-width: 128px;
    min-height: 72px;
}
.wall-caption,
.wall-name {
    font-size: 10.5px;
    font-weight: 600;
    color: @cc_fg_subtle;
}
.wall-card.active .wall-caption,
.wall-card.active .wall-name {
    color: @cc_fg_bright;
}
.wall-del-btn {
    min-width: 22px;
    min-height: 22px;
    padding: 2px 4px;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: @cc_fg_muted;
    font-size: 12px;
    transition: all 0.15s ease;
}
.wall-del-btn:hover {
    background-color: @cc_red_bg;
    border-color: @cc_red_border;
    color: @cc_red;
}
.wall-del-confirm {
    padding: 3px 6px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    background-color: @cc_red_bg;
    border: 1px solid @cc_red_border;
    color: @cc_red;
    transition: all 0.15s ease;
}
.wall-del-confirm:hover {
    background-color: @cc_red;
    color: @cc_fg_on_accent;
}
.wall-del-cancel {
    padding: 3px 6px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    background-color: @cc_surface_hover;
    border: 1px solid @cc_line;
    color: @cc_fg_subtle;
    transition: all 0.15s ease;
}
.wall-del-cancel:hover {
    background-color: @cc_line_light;
    color: @cc_fg_bright;
}

/* Theme Cards */
.theme-card {
    background-color: @cc_surface;
    border: 1px solid @cc_line;
    border-radius: 11px;
    padding: 8px 12px;
    margin-bottom: 3px;
    box-shadow: none;
    transition: background-color 120ms ease, border-color 120ms ease;
}
.theme-card:hover {
    background-color: @cc_surface_hover;
    border-color: @cc_line_light;
}
.theme-card.active {
    background-color: @cc_primary_bg;
    border-color: @cc_primary_border;
    border-left: 3px solid @cc_primary;
}
.theme-variant-pill {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background-color: @cc_surface_hover;
    color: @cc_fg_muted;
    border: 1px solid @cc_line;
}
.theme-card.active .theme-variant-pill {
    background-color: @cc_primary_bg;
    color: @cc_fg_bright;
    border-color: @cc_primary_border;
}
.theme-palette-tray {
    background-color: alpha(@cc_bg, 0.7);
    border: 1px solid @cc_line;
    border-radius: 99px;
    padding: 3px 6px;
    margin-left: 4px;
}
.theme-card:hover .theme-palette-tray {
    border-color: @cc_line_light;
}
.theme-card.active .theme-palette-tray {
    background-color: alpha(@cc_bg, 0.95);
    border-color: @cc_primary_border;
}

/* Custom Background Studio */
.custom-bg-card {
    background-color: @cc_surface;
    border: 1px solid @cc_line_light;
    border-radius: 12px;
    padding: 10px;
}
.bg-preview-area {
    min-height: 70px;
    border-radius: 8px;
    border: 1px solid @cc_line_light;
}
.color-swatch-box {
    border-radius: 8px;
    border: 1.5px solid @cc_line_light;
}
.hex-input {
    min-height: 28px;
    font-family: monospace;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 8px;
    background-color: @cc_surface_hover;
    border: 1px solid @cc_line;
    color: @cc_fg_bright;
}
.color-preset-btn {
    padding: 2px;
    border-radius: 99px;
    background: transparent;
    border: 2px solid transparent;
    box-shadow: none;
    transition: transform 100ms ease, border-color 100ms ease;
}
.color-preset-btn:hover {
    transform: scale(1.18);
}
.color-preset-btn.active {
    border-color: @cc_fg_bright;
    background-color: @cc_surface_hover;
}
.btn-primary-action {
    background-color: @cc_primary;
    color: @cc_fg_on_accent;
    font-weight: 700;
    font-size: 11.5px;
    border-radius: 9px;
    padding: 6px 14px;
    border: none;
    box-shadow: none;
    transition: opacity 120ms ease;
}
.btn-primary-action:hover {
    opacity: 0.90;
}

/* Media Player Card & Controls */
.media-badge {
    font-size: 11px;
    font-weight: 600;
    color: @cc_primary;
    background-color: @cc_primary_bg;
    border: 1px solid @cc_primary_border;
    border-radius: 8px;
    padding: 3px 8px;
}
.media-status-pill {
    font-size: 10.5px;
    font-weight: 600;
    color: @cc_primary;
    background-color: @cc_primary_bg;
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
    color: @cc_primary;
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
    background-color: @cc_slider_fill;
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
    background-color: @cc_primary;
    color: @cc_fg_on_accent;
    font-size: 16px;
    font-weight: 700;
}
.play-btn:hover {
    background-color: @cc_primary_border;
    color: @cc_fg_on_accent;
}

/* Thin Scrollbar */
scrollbar {
    background: transparent;
    border: none;
}
scrollbar.horizontal {
    margin-top: 6px;
    margin-bottom: 2px;
}
scrollbar slider {
    min-width: 4px;
    min-height: 4px;
    border-radius: 4px;
    background-color: @cc_surface_active;
}
scrollbar.horizontal slider {
    min-height: 5px;
}
scrollbar slider:hover {
    background-color: @cc_primary_border;
}

/* Custom Switches (Wi-Fi, Bluetooth) honoring theme primary accent */
switch {
    background-color: @cc_surface_hover;
    border: 1px solid @cc_line_light;
    border-radius: 12px;
    min-width: 44px;
    min-height: 24px;
    padding: 0;
    transition: all 200ms ease;
}
switch:hover {
    background-color: @cc_surface_active;
}
switch:checked {
    background-color: @cc_primary;
    border-color: @cc_primary;
}
switch slider {
    background-color: @cc_fg_muted;
    border-radius: 9px;
    min-width: 18px;
    min-height: 18px;
    margin: 2px;
    box-shadow: none;
    border: none;
}
switch:checked slider {
    background-color: @cc_fg_bright;
}
"#;

#[allow(dead_code)]
pub const POPUP_CSS: &str = POPUP_CSS_RULES;
