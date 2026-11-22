//! Linux sysfs backlight discovery and control (/sys/class/backlight).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crate::sysfs::read_sysfs_u32;

pub struct BacklightBackend {
    pub brightness_path: Option<PathBuf>,
    pub max_brightness: u32,
    pub dev_name: Option<String>,
}

impl BacklightBackend {
    pub fn probe() -> Self {
        let base = Path::new("/sys/class/backlight");
        let (dir, dev_name) = Self::find_best_backlight(base);

        let (brightness_path, max_brightness) = if let Some(ref d) = dir {
            let max = read_sysfs_u32(&d.join("max_brightness")).unwrap_or(255);
            (Some(d.join("brightness")), max)
        } else {
            (None, 255)
        };

        Self {
            brightness_path,
            max_brightness,
            dev_name,
        }
    }

    fn find_best_backlight(base: &Path) -> (Option<PathBuf>, Option<String>) {
        let entries = match fs::read_dir(base) {
            Ok(e) => e,
            Err(_) => return (None, None),
        };

        let mut best: Option<(usize, PathBuf, String)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let prio = if name.starts_with("intel_backlight")
                || name.starts_with("amdgpu_bl")
                || name.starts_with("nvidia_wmi")
            {
                0
            } else if name.starts_with("apple_backlight") || name.contains("backlight") {
                1
            } else {
                2
            };

            if best.as_ref().is_none_or(|(p, _, _)| prio < *p) {
                best = Some((prio, path, name));
            }
        }

        match best {
            Some((_, path, name)) => (Some(path), Some(name)),
            None => (None, None),
        }
    }

    pub fn has_backlight(&self) -> bool {
        self.brightness_path.is_some()
    }

    pub fn get_brightness(&self) -> u32 {
        if let Some(ref p) = self.brightness_path
            && let Some(cur) = read_sysfs_u32(p)
            && self.max_brightness > 0
        {
            return ((cur as f64 / self.max_brightness as f64) * 100.0).round() as u32;
        }
        50
    }

    pub fn set_brightness(&self, pct: u32) {
        let clamped = pct.clamp(5, 100);
        let dev_arg = self.dev_name.clone();
        thread::spawn(move || {
            let mut cmd = Command::new("brightnessctl");
            if let Some(ref dev) = dev_arg {
                cmd.args(["-d", dev]);
            }
            let _ = cmd.args(["-q", "s", &format!("{}%", clamped)]).output();
        });
    }
}

pub fn has_external_monitor() -> bool {
    let card_dir = "/sys/class/drm";
    if let Ok(entries) = fs::read_dir(card_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("card")
                && name.contains('-')
                && !name.contains("eDP")
                && !name.contains("LVDS")
                && !name.contains("DSI")
            {
                let status_path = entry.path().join("status");
                if let Ok(status) = fs::read_to_string(&status_path)
                    && status.trim() == "connected"
                {
                    return true;
                }
            }
        }
    }
    false
}
