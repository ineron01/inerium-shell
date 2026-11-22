//! GPU telemetry, vendor discovery, and power profile mode switching.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub name: String,
    pub vendor: GpuVendor,
    pub sysfs_path: PathBuf,
}

pub fn scan_gpus() -> Vec<GpuDevice> {
    let mut gpus = Vec::new();
    let base = Path::new("/sys/class/drm");
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Match card0, card1, etc. excluding connectors like card0-DP-1
            if name.starts_with("card") && !name.contains('-') {
                let device_path = entry.path().join("device");
                let vendor_id = fs::read_to_string(device_path.join("vendor"))
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();
                let vendor = match vendor_id.as_str() {
                    "0x8086" => GpuVendor::Intel,
                    "0x1002" => GpuVendor::Amd,
                    "0x10de" => GpuVendor::Nvidia,
                    _ => GpuVendor::Unknown,
                };
                gpus.push(GpuDevice {
                    name,
                    vendor,
                    sysfs_path: entry.path(),
                });
            }
        }
    }
    gpus
}

pub fn has_envycontrol() -> bool {
    Command::new("envycontrol")
        .arg("--query")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn get_gpu_mode() -> String {
    if let Ok(output) = Command::new("envycontrol").arg("--query").output() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        if s.contains("hybrid") {
            return "hybrid".to_string();
        } else if s.contains("integrated") {
            return "integrated".to_string();
        } else if s.contains("nvidia") {
            return "nvidia".to_string();
        }
    }
    "hybrid".to_string()
}

pub fn switch_gpu_mode(mode: &str) -> bool {
    let out = Command::new("sudo")
        .args(["-n", "envycontrol", "-s", mode])
        .output();
    if let Ok(res) = out
        && res.status.success()
    {
        return true;
    }
    let pk_out = Command::new("pkexec")
        .args(["envycontrol", "-s", mode])
        .output();
    matches!(pk_out, Ok(res) if res.status.success())
}
