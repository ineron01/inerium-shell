use std::path::{Path, PathBuf};

use super::{escape_lua_string, extract_lua_field};

#[derive(Debug, Clone, PartialEq)]
pub struct InputOverrides {
    pub kb_layout: String,
    pub kb_options: String,
    pub natural_scroll: bool,
    pub tap_to_click: bool,
    pub disable_while_typing: bool,
    pub sensitivity: f64,
    pub accel_profile: String,
    pub scroll_factor: f64,
    pub launcher_bind: Option<String>,
}

impl Default for InputOverrides {
    fn default() -> Self {
        Self {
            kb_layout: "us,ru".to_string(),
            kb_options: "grp:alt_shift_toggle".to_string(),
            natural_scroll: false,
            tap_to_click: true,
            disable_while_typing: true,
            sensitivity: 0.0,
            accel_profile: "flat".to_string(),
            scroll_factor: 1.0,
            launcher_bind: None,
        }
    }
}

pub const STANDARD_GESTURES_LUA: &str = "\
hl.gesture({\n    fingers   = 4,\n    direction = \"horizontal\",\n    action    = \"workspace\",\n})\n\n\
hl.gesture({\n    fingers   = 3,\n    direction = \"down\",\n    action    = \"close\",\n})\n\n\
hl.gesture({\n    fingers   = 3,\n    direction = \"up\",\n    action    = \"fullscreen\",\n})\n\n\
hl.gesture({\n    fingers   = 3,\n    direction = \"left\",\n    action    = \"float\",\n})\n";

pub fn extract_gestures_lua(content: &str) -> Option<String> {
    let mut gestures = Vec::new();
    let mut in_gesture = false;
    let mut cur_block = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !in_gesture && trimmed.starts_with("hl.gesture(") {
            in_gesture = true;
            cur_block.clear();
            cur_block.push_str(line);
            cur_block.push('\n');
            if trimmed.ends_with("})") {
                gestures.push(cur_block.trim_end().to_string());
                cur_block.clear();
                in_gesture = false;
            }
            continue;
        }
        if in_gesture {
            cur_block.push_str(line);
            cur_block.push('\n');
            if trimmed.starts_with("})") || trimmed.ends_with("})") {
                gestures.push(cur_block.trim_end().to_string());
                cur_block.clear();
                in_gesture = false;
            }
        }
    }

    if gestures.is_empty() {
        None
    } else {
        Some(gestures.join("\n\n"))
    }
}

pub fn generate_inputs_lua_with_gestures(cfg: &InputOverrides, gestures: &str) -> String {
    let trimmed = gestures.trim();
    let gestures_block = if trimmed.is_empty() {
        STANDARD_GESTURES_LUA.trim_end()
    } else {
        trimmed
    };

    format!(
        "-- Input configuration\n\
-- Wiki: https://wiki.hypr.land/Configuring/Variables/#input\n\n\
hl.config({{\n    input = {{\n        accel_profile = \"{accel}\",\n        kb_layout     = \"{layout}\",\n        kb_options    = \"{options}\",\n        sensitivity   = {sensitivity:.2},\n        scroll_factor = {scroll:.2},\n        touchpad      = {{\n            natural_scroll       = {natural_scroll},\n            tap_to_click         = {tap_to_click},\n            disable_while_typing = {disable_while_typing},\n        }},\n    }},\n}})\n\n\
{gestures_block}\n",
        layout = escape_lua_string(cfg.kb_layout.trim()),
        options = escape_lua_string(cfg.kb_options.trim()),
        sensitivity = cfg.sensitivity.clamp(-1.0, 1.0),
        accel = escape_lua_string(cfg.accel_profile.trim()),
        scroll = cfg.scroll_factor.clamp(0.1, 5.0),
        natural_scroll = cfg.natural_scroll,
        tap_to_click = cfg.tap_to_click,
        disable_while_typing = cfg.disable_while_typing,
    )
}

pub fn generate_inputs_lua(cfg: &InputOverrides) -> String {
    generate_inputs_lua_with_gestures(cfg, STANDARD_GESTURES_LUA)
}

pub fn parse_inputs_lua_with_fallback(content: &str, fallback: InputOverrides) -> InputOverrides {
    let mut cfg = fallback;
    if let Some(v) = extract_lua_field(content, "kb_layout")
        && !v.is_empty()
    {
        cfg.kb_layout = v;
    }
    if let Some(v) = extract_lua_field(content, "kb_options") {
        cfg.kb_options = v;
    }
    if let Some(v) = extract_lua_field(content, "natural_scroll") {
        cfg.natural_scroll = v == "true";
    }
    if let Some(v) = extract_lua_field(content, "tap_to_click")
        .or_else(|| extract_lua_field(content, "tap-to-click"))
    {
        cfg.tap_to_click = v == "true";
    }
    if let Some(v) = extract_lua_field(content, "disable_while_typing") {
        cfg.disable_while_typing = v == "true";
    }
    if let Some(v) = extract_lua_field(content, "sensitivity")
        && let Ok(f) = v.parse::<f64>()
    {
        cfg.sensitivity = f;
    }
    if let Some(v) = extract_lua_field(content, "accel_profile")
        && !v.is_empty()
    {
        cfg.accel_profile = v;
    }
    if let Some(v) = extract_lua_field(content, "scroll_factor")
        && let Ok(f) = v.parse::<f64>()
    {
        cfg.scroll_factor = f;
    }
    cfg
}

#[allow(dead_code)]
pub fn parse_inputs_lua(content: &str) -> InputOverrides {
    parse_inputs_lua_with_fallback(content, InputOverrides::default())
}

pub fn parse_launcher_bind(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.contains("rofi -show drun") && line.contains("hl.bind(") {
            let start = line.find("hl.bind(")?;
            let rem = &line[start + 8..];
            let comma = rem.find(',')?;
            let key_expr = rem[..comma].trim();
            if key_expr.contains("mainMod .. \" + ") {
                if let Some(suffix) = key_expr.split("mainMod .. \" + ").nth(1) {
                    let clean = suffix.trim_matches('"').trim();
                    return Some(format!("SUPER + {clean}"));
                }
            } else if key_expr.starts_with('"') && key_expr.ends_with('"') && key_expr.len() >= 2 {
                return Some(key_expr[1..key_expr.len() - 1].to_string());
            } else {
                return Some(key_expr.to_string());
            }
        }
    }
    None
}

pub fn replace_launcher_bind(content: &str, new_key: &str) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in content.lines() {
        if line.contains("rofi -show drun") && line.contains("hl.bind(") && !replaced {
            let bind_str = if new_key == "SUPER + Space" {
                "hl.bind(mainMod .. \" + Space\", hl.dsp.exec_cmd(launchPrefix .. \"rofi -show drun\"))"
            } else {
                &format!("hl.bind(\"{new_key}\", hl.dsp.exec_cmd(launchPrefix .. \"rofi -show drun\"))")
            };
            lines.push(bind_str.to_string());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("hl.bind(\"{new_key}\", hl.dsp.exec_cmd(launchPrefix .. \"rofi -show drun\"))"));
    }
    lines.join("\n") + "\n"
}

pub fn detect_battery_charge_threshold() -> Option<(PathBuf, u8)> {
    let backend = inerium_hw::battery::BatteryBackend::probe();
    if backend.has_threshold() {
        let val = backend.get_threshold() as u8;
        let path = match &backend {
            inerium_hw::battery::BatteryBackend::PercentageThreshold { end_path, .. } => end_path.clone(),
            inerium_hw::battery::BatteryBackend::BinaryConservationMode { control_path, .. } => control_path.clone(),
            inerium_hw::battery::BatteryBackend::ReadOnly { .. } => return None,
        };
        Some((path, val))
    } else {
        None
    }
}

pub fn set_battery_charge_threshold(val: u8) -> Result<(), String> {
    let backend = inerium_hw::battery::BatteryBackend::probe();
    if backend.has_threshold() {
        backend.set_threshold(val as u32)
    } else {
        Err("No battery threshold control found in sysfs".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleConfig {
    pub enabled: bool,
    pub dim_secs: u32,
    pub lock_secs: u32,
    pub dpms_secs: u32,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dim_secs: 240,
            lock_secs: 420,
            dpms_secs: 600,
        }
    }
}

pub fn generate_hypridle_conf(cfg: &IdleConfig) -> String {
    let mut out = format!(
        "# Auto-generated by inerium-setup (local hypridle overrides, gitignored)\n\
# inerium-setup: enabled={} dim={} lock={} dpms={}\n\n\
general {{\n    lock_cmd = pidof hyprlock || hyprlock\n    before_sleep_cmd = loginctl lock-session\n    after_sleep_cmd = hyprctl dispatch dpms on\n}}\n",
        cfg.enabled, cfg.dim_secs, cfg.lock_secs, cfg.dpms_secs
    );

    if cfg.enabled {
        if cfg.dim_secs > 0 {
            out.push_str(&format!(
                "\nlistener {{\n    timeout = {}\n    on-timeout = brightnessctl -s set 20%\n    on-resume = brightnessctl -r\n}}\n",
                cfg.dim_secs
            ));
        }
        if cfg.lock_secs > 0 {
            out.push_str(&format!(
                "\nlistener {{\n    timeout = {}\n    on-timeout = loginctl lock-session\n}}\n",
                cfg.lock_secs
            ));
        }
        if cfg.dpms_secs > 0 {
            out.push_str(&format!(
                "\nlistener {{\n    timeout = {}\n    on-timeout = hyprctl dispatch dpms off\n    on-resume = hyprctl dispatch dpms on\n}}\n",
                cfg.dpms_secs
            ));
        }
    }
    out
}

pub fn parse_hypridle_conf(content: &str) -> IdleConfig {
    let mut cfg = IdleConfig::default();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(meta) = trimmed.strip_prefix("# inerium-setup:") {
            for token in meta.split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    match k {
                        "enabled" => cfg.enabled = v == "true",
                        "dim" => {
                            if let Ok(n) = v.parse() {
                                cfg.dim_secs = n;
                            }
                        }
                        "lock" => {
                            if let Ok(n) = v.parse() {
                                cfg.lock_secs = n;
                            }
                        }
                        "dpms" => {
                            if let Ok(n) = v.parse() {
                                cfg.dpms_secs = n;
                            }
                        }
                        _ => {}
                    }
                }
            }
            return cfg;
        }
    }

    // Fallback: parse raw hypridle listener blocks (e.g., generated by install.sh)
    let mut in_listener = false;
    let mut cur_timeout: Option<u32> = None;
    let mut cur_cmd = String::new();
    let mut saw_listener = false;
    let mut saw_dim = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("listener") && trimmed.contains('{') {
            in_listener = true;
            cur_timeout = None;
            cur_cmd.clear();
            continue;
        }
        if in_listener {
            if trimmed == "}" {
                if let Some(t) = cur_timeout {
                    saw_listener = true;
                    if cur_cmd.contains("brightnessctl") {
                        cfg.dim_secs = t;
                        saw_dim = true;
                    } else if cur_cmd.contains("lock-session") {
                        cfg.lock_secs = t;
                    } else if cur_cmd.contains("dpms off") {
                        cfg.dpms_secs = t;
                    }
                }
                in_listener = false;
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                if k == "timeout" {
                    cur_timeout = v.parse::<u32>().ok();
                } else if k == "on-timeout" {
                    cur_cmd = v.to_string();
                }
            }
        }
    }

    if saw_listener && !saw_dim {
        cfg.dim_secs = 0;
    }

    cfg
}

pub fn generate_hypridle_dropin(conf_path: &Path) -> String {
    format!(
        "[Service]\nExecStart=\nExecStart=/usr/bin/hypridle -c {}\n",
        conf_path.display()
    )
}
