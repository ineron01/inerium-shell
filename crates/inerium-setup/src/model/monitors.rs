use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::{atomic_write, escape_lua_string, extract_lua_field};

pub fn parse_workspaces_count(raw: &str) -> u8 {
    raw.trim()
        .lines()
        .next()
        .and_then(|l| l.trim().parse::<u8>().ok())
        .filter(|&v| (1..=9).contains(&v))
        .unwrap_or(3)
}

pub fn read_workspaces_count(path: &Path) -> u8 {
    fs::read_to_string(path)
        .map(|s| parse_workspaces_count(&s))
        .unwrap_or(3)
}

pub fn write_workspaces_count(path: &Path, count: u8) -> Result<(), String> {
    if !(1..=9).contains(&count) {
        return Err(format!("Workspaces count must be 1..=9, got {count}"));
    }
    atomic_write(path, &format!("{count}\n"))
}

// ============================================================================
// Monitors & monitors_local.lua
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyprMonitor {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub refresh_rate: f64,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub vrr: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub available_modes: Vec<String>,
}

fn default_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorConfig {
    pub output: String,
    pub mode: String,
    pub position: String,
    pub scale: f64,
    pub vrr: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    Auto,
    HorizontalTop,
    HorizontalBottom,
    Vertical,
}

pub fn parse_hypr_monitors_json(raw: &str) -> Result<Vec<HyprMonitor>, String> {
    serde_json::from_str::<Vec<HyprMonitor>>(raw)
        .map_err(|e| format!("Failed parsing hyprctl monitors JSON: {e}"))
}

pub fn format_scale(scale: f64) -> String {
    let rounded = (scale * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.005 {
        format!("{:.1}", rounded.round())
    } else {
        let s = format!("{rounded:.2}");
        s.trim_end_matches('0').to_string()
    }
}

pub fn parse_mode_dimensions(mode: &str, fallback_w: u32, fallback_h: u32) -> (u32, u32) {
    let res_part = mode.split('@').next().unwrap_or("");
    if let Some((w_str, h_str)) = res_part.split_once('x')
        && let (Ok(w), Ok(h)) = (w_str.trim().parse::<u32>(), h_str.trim().parse::<u32>())
        && w > 0
        && h > 0
    {
        return (w, h);
    }
    (fallback_w.max(640), fallback_h.max(480))
}

pub fn apply_layout_preset(
    configs: &mut [MonitorConfig],
    live: &[HyprMonitor],
    preset: LayoutPreset,
) {
    if preset == LayoutPreset::Auto {
        for cfg in configs.iter_mut() {
            cfg.position = "auto".to_string();
        }
        return;
    }

    let logical_sizes: Vec<(i32, i32)> = configs
        .iter()
        .map(|cfg| {
            let live_m = live.iter().find(|m| m.name == cfg.output);
            let (fw, fh) = live_m.map(|m| (m.width, m.height)).unwrap_or((1920, 1080));
            let (pw, ph) = parse_mode_dimensions(&cfg.mode, fw, fh);
            let scale = if cfg.scale > 0.1 { cfg.scale } else { 1.0 };
            (
                ((pw as f64) / scale).round() as i32,
                ((ph as f64) / scale).round() as i32,
            )
        })
        .collect();

    let max_h = logical_sizes.iter().map(|&(_, h)| h).max().unwrap_or(1080);
    let mut cursor_x = 0i32;
    let mut cursor_y = 0i32;

    for (cfg, &(lw, lh)) in configs.iter_mut().zip(logical_sizes.iter()) {
        match preset {
            LayoutPreset::Auto => {}
            LayoutPreset::HorizontalTop => {
                cfg.position = format!("{cursor_x}x0");
                cursor_x += lw;
            }
            LayoutPreset::HorizontalBottom => {
                let y = (max_h - lh).max(0);
                cfg.position = format!("{cursor_x}x{y}");
                cursor_x += lw;
            }
            LayoutPreset::Vertical => {
                cfg.position = format!("0x{cursor_y}");
                cursor_y += lh;
            }
        }
    }
}

pub fn generate_monitors_lua(monitors: &[MonitorConfig]) -> String {
    let mut out = String::from(
        "-- Monitor configuration\n-- Wiki: https://wiki.hypr.land/Configuring/Monitors/\n\n",
    );

    for m in monitors {
        if m.output.trim().is_empty() {
            continue;
        }
        let scale_str = format_scale(m.scale);
        let vrr_val = if m.vrr { 1 } else { 0 };
        out.push_str(&format!(
            "hl.monitor({{\n    output   = \"{}\",\n    mode     = \"{}\",\n    position = \"{}\",\n    scale    = {},\n    vrr      = {},\n}})\n\n",
            escape_lua_string(m.output.trim()),
            escape_lua_string(m.mode.trim()),
            escape_lua_string(m.position.trim()),
            scale_str,
            vrr_val
        ));
    }

    out.push_str(
        "-- Universal fallback for newly connected displays\nhl.monitor({\n    output   = \"\",\n    mode     = \"preferred\",\n    position = \"auto\",\n    scale    = 1.0,\n})\n",
    );
    out
}

pub fn parse_monitors_lua(content: &str) -> Vec<MonitorConfig> {
    let mut result = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("hl.monitor(") {
        let after = &rest[start + "hl.monitor(".len()..];
        let Some(end) = after.find("})") else {
            break;
        };
        let block = &after[..end];
        if let Some(output) = extract_lua_field(block, "output")
            && !output.is_empty()
        {
            let mode = extract_lua_field(block, "mode").unwrap_or_else(|| "preferred".to_string());
            let position =
                extract_lua_field(block, "position").unwrap_or_else(|| "auto".to_string());
            let scale = extract_lua_field(block, "scale")
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|&v| (0.5..=4.0).contains(&v))
                .unwrap_or(1.0);
            let vrr = extract_lua_field(block, "vrr")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false);
            result.push(MonitorConfig {
                output,
                mode,
                position,
                scale,
                vrr,
            });
        }
        rest = &after[end + 2..];
    }
    result
}

pub fn merge_monitors(live: &[HyprMonitor], saved: &[MonitorConfig]) -> Vec<MonitorConfig> {
    live.iter()
        .filter(|m| !m.disabled)
        .map(|m| {
            if let Some(s) = saved.iter().find(|s| s.output == m.name) {
                s.clone()
            } else {
                let current_mode = if m.width > 0 && m.height > 0 && m.refresh_rate > 1.0 {
                    let candidate = format!("{}x{}@{:.2}Hz", m.width, m.height, m.refresh_rate);
                    if m.available_modes.contains(&candidate) {
                        candidate
                    } else {
                        "preferred".to_string()
                    }
                } else {
                    "preferred".to_string()
                };
                MonitorConfig {
                    output: m.name.clone(),
                    mode: current_mode,
                    position: format!("{}x{}", m.x, m.y),
                    scale: m.scale,
                    vrr: m.vrr,
                }
            }
        })
        .collect()
}
