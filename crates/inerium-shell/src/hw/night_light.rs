//! Night Light & Display Temperature controller with schedule synchronization via `hyprsunset`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

pub const DEFAULT_DAY_TEMP: u32 = 6500;
pub const MIN_DAY_TEMP: u32 = 4000;
pub const DEFAULT_NIGHT_TEMP: u32 = 4000;
pub const MIN_NIGHT_TEMP: u32 = 2500;
pub const MAX_TEMP: u32 = 6500;

fn default_day_temp() -> u32 {
    DEFAULT_DAY_TEMP
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NightLightConfig {
    pub enabled: bool,
    #[serde(default = "default_day_temp")]
    pub day_temperature: u32,
    pub temperature: u32,
    pub schedule_enabled: bool,
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    #[serde(default)]
    pub manual_override_phase: Option<bool>,
}

impl Default for NightLightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            day_temperature: DEFAULT_DAY_TEMP,
            temperature: DEFAULT_NIGHT_TEMP,
            schedule_enabled: false,
            start_hour: 22,
            start_minute: 0,
            end_hour: 7,
            end_minute: 0,
            manual_override_phase: None,
        }
    }
}

fn config_path() -> PathBuf {
    super::home_dir().join(".config/inerium/night_light.json")
}

pub fn load_config() -> NightLightConfig {
    let path = config_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return NightLightConfig::default();
    };
    let mut cfg: NightLightConfig = serde_json::from_str(&raw).unwrap_or_default();
    cfg.day_temperature = cfg.day_temperature.clamp(MIN_DAY_TEMP, MAX_TEMP);
    cfg.temperature = cfg.temperature.clamp(MIN_NIGHT_TEMP, MAX_TEMP);
    cfg.start_hour = cfg.start_hour.min(23);
    cfg.start_minute = cfg.start_minute.min(59);
    cfg.end_hour = cfg.end_hour.min(23);
    cfg.end_minute = cfg.end_minute.min(59);
    cfg
}

pub fn save_config(cfg: &NightLightConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

/// Parses flexible time inputs (`"22:00"`, `"22.00"`, `"22,00"`, `"22 00"`, `"2200"`, `"730"`, `"22"`)
/// into `(hour, minute)`.
pub fn parse_hhmm(input: &str) -> Option<(u8, u8)> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    let (h, m): (u8, u8) = if let Some(sep_idx) = s.find(|c: char| !c.is_ascii_digit()) {
        let (left, right) = s.split_at(sep_idx);
        let right = right.trim_start_matches(|c: char| !c.is_ascii_digit());
        if left.is_empty() || right.is_empty() || !right.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        (left.parse().ok()?, right.parse().ok()?)
    } else {
        match s.len() {
            1 | 2 => (s.parse().ok()?, 0),
            3 => (s[..1].parse().ok()?, s[1..].parse().ok()?),
            4 => (s[..2].parse().ok()?, s[2..].parse().ok()?),
            _ => return None,
        }
    };

    (h <= 23 && m <= 59).then_some((h, m))
}

/// Returns `true` if `(hour, minute)` falls inside `[start, end)`,
/// supporting overnight windows (e.g. `22:00` -> `07:00`).
pub fn is_in_schedule_window(cfg: &NightLightConfig, hour: u8, minute: u8) -> bool {
    let now = u16::from(hour) * 60 + u16::from(minute);
    let start = u16::from(cfg.start_hour) * 60 + u16::from(cfg.start_minute);
    let end = u16::from(cfg.end_hour) * 60 + u16::from(cfg.end_minute);

    if start == end {
        false
    } else if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

pub fn current_local_hm() -> Option<(u8, u8)> {
    let dt = glib::DateTime::now_local().ok()?;
    let h = u8::try_from(dt.hour()).ok()?;
    let m = u8::try_from(dt.minute()).ok()?;
    Some((h, m))
}

pub fn active_temperature(cfg: &NightLightConfig) -> u32 {
    if cfg.enabled {
        cfg.temperature.clamp(MIN_NIGHT_TEMP, MAX_TEMP)
    } else {
        cfg.day_temperature.clamp(MIN_DAY_TEMP, MAX_TEMP)
    }
}

/// Applies the effective color temperature via `hyprsunset` IPC,
/// spawning the daemon lazily if a non-identity temperature is needed.
pub fn apply_config(cfg: &NightLightConfig) {
    let target = active_temperature(cfg);
    if target < MAX_TEMP {
        let ok = Command::new("hyprctl")
            .args(["hyprsunset", "temperature", &target.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        if !ok {
            eprintln!("[night_light] hyprctl IPC failed for {target}K, spawning hyprsunset");
            if let Err(e) = Command::new("setsid")
                .args(["hyprsunset", "-t", &target.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                eprintln!("[night_light] failed to spawn hyprsunset: {e}");
            }
        }
    } else {
        let out = Command::new("hyprctl")
            .args(["hyprsunset", "identity"])
            .output();
        if let Ok(o) = out
            && !o.status.success()
        {
            eprintln!(
                "[night_light] hyprctl identity failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            );
        }
    }
}

/// Reconciles `cfg` against `(hour, minute)`. Returns `true` if `cfg` changed.
pub fn reconcile_schedule_at(cfg: &mut NightLightConfig, hour: u8, minute: u8) -> bool {
    if !cfg.schedule_enabled {
        return false;
    }
    let in_window = is_in_schedule_window(cfg, hour, minute);
    if cfg.manual_override_phase == Some(in_window) {
        return false;
    }

    let mut changed = false;
    if cfg.manual_override_phase.is_some() {
        cfg.manual_override_phase = None;
        changed = true;
    }
    if cfg.enabled != in_window {
        cfg.enabled = in_window;
        changed = true;
    }
    changed
}

pub fn init_on_startup() {
    let mut cfg = load_config();
    if let Some((h, m)) = current_local_hm()
        && reconcile_schedule_at(&mut cfg, h, m)
    {
        save_config(&cfg);
    }
    apply_config(&cfg);
}

pub fn sync_schedule_tick() {
    let mut cfg = load_config();
    let Some((h, m)) = current_local_hm() else {
        return;
    };
    let was_enabled = cfg.enabled;
    if reconcile_schedule_at(&mut cfg, h, m) {
        save_config(&cfg);
        if cfg.enabled != was_enabled {
            apply_config(&cfg);
        }
    }
}

fn update_and_apply(mutate: impl FnOnce(&mut NightLightConfig)) -> NightLightConfig {
    let mut cfg = load_config();
    mutate(&mut cfg);
    save_config(&cfg);
    apply_config(&cfg);
    cfg
}

fn sync_enabled_with_schedule(cfg: &mut NightLightConfig) {
    cfg.manual_override_phase = None;
    if cfg.schedule_enabled
        && let Some((h, m)) = current_local_hm()
    {
        cfg.enabled = is_in_schedule_window(cfg, h, m);
    }
}

pub fn set_enabled(enabled: bool) -> NightLightConfig {
    update_and_apply(|cfg| {
        cfg.enabled = enabled;
        if cfg.schedule_enabled
            && let Some((h, m)) = current_local_hm()
        {
            let in_window = is_in_schedule_window(cfg, h, m);
            cfg.manual_override_phase = (enabled != in_window).then_some(in_window);
        } else {
            cfg.manual_override_phase = None;
        }
    })
}

pub fn set_day_temperature(temp: u32) -> NightLightConfig {
    update_and_apply(|cfg| {
        cfg.day_temperature = temp.clamp(MIN_DAY_TEMP, MAX_TEMP);
    })
}

pub fn set_temperature(temp: u32) -> NightLightConfig {
    update_and_apply(|cfg| {
        cfg.temperature = temp.clamp(MIN_NIGHT_TEMP, MAX_TEMP);
    })
}

pub fn set_schedule_enabled(schedule_enabled: bool) -> NightLightConfig {
    update_and_apply(|cfg| {
        cfg.schedule_enabled = schedule_enabled;
        sync_enabled_with_schedule(cfg);
    })
}

pub fn set_schedule_times(
    start_hour: u8,
    start_minute: u8,
    end_hour: u8,
    end_minute: u8,
) -> NightLightConfig {
    update_and_apply(|cfg| {
        cfg.start_hour = start_hour.min(23);
        cfg.start_minute = start_minute.min(59);
        cfg.end_hour = end_hour.min(23);
        cfg.end_minute = end_minute.min(59);
        sync_enabled_with_schedule(cfg);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overnight_window_checks() {
        let cfg = NightLightConfig {
            start_hour: 22,
            start_minute: 0,
            end_hour: 7,
            end_minute: 0,
            ..Default::default()
        };
        assert!(!is_in_schedule_window(&cfg, 21, 59));
        assert!(is_in_schedule_window(&cfg, 22, 0));
        assert!(is_in_schedule_window(&cfg, 23, 30));
        assert!(is_in_schedule_window(&cfg, 0, 0));
        assert!(is_in_schedule_window(&cfg, 6, 59));
        assert!(!is_in_schedule_window(&cfg, 7, 0));
    }

    #[test]
    fn daytime_window_checks() {
        let cfg = NightLightConfig {
            start_hour: 9,
            start_minute: 30,
            end_hour: 18,
            end_minute: 15,
            ..Default::default()
        };
        assert!(!is_in_schedule_window(&cfg, 9, 29));
        assert!(is_in_schedule_window(&cfg, 9, 30));
        assert!(is_in_schedule_window(&cfg, 18, 14));
        assert!(!is_in_schedule_window(&cfg, 18, 15));
    }

    #[test]
    fn manual_override_persists_until_phase_changes() {
        let mut cfg = NightLightConfig {
            enabled: false,
            schedule_enabled: true,
            start_hour: 22,
            start_minute: 0,
            end_hour: 7,
            end_minute: 0,
            manual_override_phase: Some(true),
            ..Default::default()
        };

        assert!(!reconcile_schedule_at(&mut cfg, 23, 0));
        assert!(!cfg.enabled);

        assert!(reconcile_schedule_at(&mut cfg, 7, 0));
        assert!(!cfg.enabled);
        assert_eq!(cfg.manual_override_phase, None);

        assert!(reconcile_schedule_at(&mut cfg, 22, 0));
        assert!(cfg.enabled);
    }

    #[test]
    fn parse_hhmm_variants() {
        assert_eq!(parse_hhmm("22:00"), Some((22, 0)));
        assert_eq!(parse_hhmm("20.08"), Some((20, 8)));
        assert_eq!(parse_hhmm("20,08"), Some((20, 8)));
        assert_eq!(parse_hhmm("20 08"), Some((20, 8)));
        assert_eq!(parse_hhmm("7:05"), Some((7, 5)));
        assert_eq!(parse_hhmm("0730"), Some((7, 30)));
        assert_eq!(parse_hhmm("730"), Some((7, 30)));
        assert_eq!(parse_hhmm("22"), Some((22, 0)));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("12:60"), None);
        assert_eq!(parse_hhmm("abc"), None);
    }
}
