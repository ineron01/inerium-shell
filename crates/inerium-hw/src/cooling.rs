use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::sysfs::{read_sysfs_string, read_sysfs_u32};

pub enum PlatformControl {
    #[cfg(feature = "msi-ec")]
    MsiEc { base_path: PathBuf },
    None,
}

impl PlatformControl {
    pub fn probe() -> Self {
        #[cfg(feature = "msi-ec")]
        {
            let p = Path::new("/sys/devices/platform/msi-ec");
            if p.exists() {
                return Self::MsiEc {
                    base_path: p.to_path_buf(),
                };
            }
        }
        Self::None
    }

    pub fn get_cooler_boost(&self) -> bool {
        match self {
            #[cfg(feature = "msi-ec")]
            Self::MsiEc { base_path } => {
                read_sysfs_string(&base_path.join("cooler_boost"))
                    .map(|s| s == "on")
                    .unwrap_or(false)
            }
            Self::None => false,
        }
    }

    pub fn set_cooler_boost(&self, on: bool) {
        match self {
            #[cfg(feature = "msi-ec")]
            Self::MsiEc { base_path } => {
                let cb = base_path.join("cooler_boost");
                let val = if on { "on\n" } else { "off\n" };
                let _ = fs::write(cb, val);
                if !on {
                    self.apply_balanced_fan_curve();
                }
            }
            Self::None => {}
        }
    }

    pub fn get_fan_mode(&self) -> String {
        match self {
            #[cfg(feature = "msi-ec")]
            Self::MsiEc { base_path } => {
                read_sysfs_string(&base_path.join("fan_mode"))
                    .unwrap_or_else(|| "advanced".to_string())
            }
            Self::None => "auto".to_string(),
        }
    }

    pub fn apply_balanced_fan_curve(&self) {
        match self {
            #[cfg(feature = "msi-ec")]
            Self::MsiEc { base_path } => {
                let _ = fs::write(base_path.join("fan_mode"), "advanced\n");
            }
            Self::None => {}
        }
    }
}

pub struct FanSensor {
    pub name: String,
    pub hwmon_name: String,
    pub index: u32,
    pub cached_path: PathBuf,
}

pub struct FanTelemetry {
    sensors: Vec<FanSensor>,
    pub has_hwmon_fans: bool,
    last_probe: Instant,
}

impl FanTelemetry {
    pub fn probe() -> Self {
        let sensors = Self::scan_hwmon();
        let has_hwmon_fans = !sensors.is_empty();
        Self {
            sensors,
            has_hwmon_fans,
            last_probe: Instant::now(),
        }
    }

    fn driver_priority(name: &str) -> u32 {
        match name {
            "msi_wmi_platform" | "msi_ec" => 10,
            "asus-nb-wmi" | "asus_custom" => 10,
            "thinkpad" | "thinkpad_hwmon" => 10,
            "dell_smm" | "dell_smm_hwmon" => 10,
            "acpi_fan" => 8,
            "coretemp" | "k10temp" => 6,
            "amdgpu" | "nvidia" => 2,
            _ => 5,
        }
    }

    fn scan_hwmon() -> Vec<FanSensor> {
        let mut list = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let p = entry.path();
                let hw_name = read_sysfs_string(&p.join("name")).unwrap_or_default();
                for i in 1..=4 {
                    let fan_file = format!("fan{}_input", i);
                    let full_path = p.join(&fan_file);
                    if read_sysfs_u32(&full_path).is_some() {
                        let label = read_sysfs_string(&p.join(format!("fan{}_label", i)))
                            .unwrap_or_else(|| format!("{}: Fan {}", hw_name, i));
                        list.push(FanSensor {
                            name: label,
                            hwmon_name: hw_name.clone(),
                            index: i,
                            cached_path: full_path,
                        });
                    }
                }
            }
        }

        list.sort_by(|a, b| {
            let pa = Self::driver_priority(&a.hwmon_name);
            let pb = Self::driver_priority(&b.hwmon_name);
            pb.cmp(&pa).then_with(|| a.index.cmp(&b.index))
        });
        list
    }

    pub fn read_rpm(&mut self) -> (Option<u32>, Option<u32>) {
        let mut f1 = self.sensors.first().and_then(|s| read_sysfs_u32(&s.cached_path));
        let mut f2 = self.sensors.get(1).and_then(|s| read_sysfs_u32(&s.cached_path));

        if self.has_hwmon_fans && f1.is_none() && f2.is_none() && self.last_probe.elapsed() > Duration::from_secs(5) {
            self.sensors = Self::scan_hwmon();
            self.last_probe = Instant::now();
            f1 = self.sensors.first().and_then(|s| read_sysfs_u32(&s.cached_path));
            f2 = self.sensors.get(1).and_then(|s| read_sysfs_u32(&s.cached_path));
        }

        (f1, f2)
    }
}

pub struct CoolingSubsystem {
    pub control: PlatformControl,
    pub telemetry: Mutex<FanTelemetry>,
}

impl CoolingSubsystem {
    pub fn probe() -> Self {
        Self {
            control: PlatformControl::probe(),
            telemetry: Mutex::new(FanTelemetry::probe()),
        }
    }

    pub fn has_fan_telemetry(&self) -> bool {
        if let Ok(telem) = self.telemetry.lock()
            && telem.has_hwmon_fans {
                return true;
            }
        #[cfg(feature = "msi-ec")]
        if matches!(self.control, PlatformControl::MsiEc { .. }) {
            return true;
        }
        false
    }

    pub fn get_telemetry(&self) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
        let (cpu_rpm, gpu_rpm) = if let Ok(mut telem) = self.telemetry.lock() {
            telem.read_rpm()
        } else {
            (None, None)
        };

        let (cpu_pct, gpu_pct) = match &self.control {
            #[cfg(feature = "msi-ec")]
            PlatformControl::MsiEc { base_path } => {
                let cp = read_sysfs_u32(&base_path.join("cpu/realtime_fan_speed"));
                let gp = read_sysfs_u32(&base_path.join("gpu/realtime_fan_speed"));
                (cp, gp)
            }
            _ => (None, None),
        };

        (cpu_rpm, gpu_rpm, cpu_pct, gpu_pct)
    }

    pub fn format_fan_telemetry(&self) -> String {
        let (cpu_rpm, gpu_rpm, cpu_pct, gpu_pct) = self.get_telemetry();
        let c_pct = cpu_pct.unwrap_or(0);
        let g_pct = gpu_pct.unwrap_or(0);

        let c_rpm_str = match cpu_rpm {
            Some(rpm) if rpm > 0 => format!("{} RPM", rpm),
            _ => "0 RPM".to_string(),
        };
        let g_rpm_str = match gpu_rpm {
            Some(rpm) if rpm > 0 => format!("{} RPM", rpm),
            _ => "0 RPM".to_string(),
        };

        let fan_prefix = inerium_i18n::tr("󰈨 Fans", "󰈨 Кулеры");
        if cpu_pct.is_some() || gpu_pct.is_some() {
            format!("{}: CPU {}% ({}) · GPU {}% ({})", fan_prefix, c_pct, c_rpm_str, g_pct, g_rpm_str)
        } else {
            format!("{}: Fan 1 ({}) · Fan 2 ({})", fan_prefix, c_rpm_str, g_rpm_str)
        }
    }

    pub fn format_fan_meta(&self) -> String {
        let mode = self.control.get_fan_mode();
        let (cpu_rpm, gpu_rpm, cpu_pct, gpu_pct) = self.get_telemetry();
        let c_pct = cpu_pct.unwrap_or(0);
        let g_pct = gpu_pct.unwrap_or(0);

        match (cpu_rpm, gpu_rpm) {
            (Some(cr), Some(gr)) if cr > 0 && gr > 0 => {
                format!("msi-ec: {}  ·  CPU {}% ({} RPM)  ·  GPU {}% ({} RPM)", mode, c_pct, cr, g_pct, gr)
            }
            (Some(cr), _) if cr > 0 => {
                format!("msi-ec: {}  ·  CPU {}% ({} RPM)  ·  GPU {}%", mode, c_pct, cr, g_pct)
            }
            _ => {
                format!("msi-ec: {}  ·  CPU {}%  ·  GPU {}%", mode, c_pct, g_pct)
            }
        }
    }
}

// ============================================================================
// CPU Temperature reading utilities (Hwmon sensor discovery)
// ============================================================================

pub fn cpu_driver_priority(name: &str) -> Option<usize> {
    let n = name.trim();
    if n == "k10temp" {
        Some(0)
    } else if n == "zenpower" {
        Some(1)
    } else if n == "coretemp" {
        Some(2)
    } else if n == "cpu_thermal" || n == "soc_dts" || n.starts_with("soc_dts") || n == "acpitz" {
        Some(3)
    } else {
        None
    }
}

pub fn read_hwmon_dir_temp(dir: &Path) -> Option<i32> {
    // Prefer labeled CPU package/die sensors (e.g., "Package id 0", "Tctl", "Tdie")
    for idx in 1..=16 {
        let label_path = dir.join(format!("temp{idx}_label"));
        if let Some(label) = read_sysfs_string(&label_path) {
            let l = label.trim();
            if l.eq_ignore_ascii_case("Package id 0")
                || l.eq_ignore_ascii_case("Tctl")
                || l.eq_ignore_ascii_case("Tdie")
            {
                let input_path = dir.join(format!("temp{idx}_input"));
                if let Some(val) = read_sysfs_u32(&input_path) {
                    return Some((val / 1000) as i32);
                }
            }
        }
    }

    // Fallback to temp1_input (or first available tempN_input) within this sensor
    for idx in 1..=16 {
        let input_path = dir.join(format!("temp{idx}_input"));
        if let Some(val) = read_sysfs_u32(&input_path) {
            return Some((val / 1000) as i32);
        }
    }
    None
}

pub fn read_cpu_temp_from_base(base: &Path) -> Option<i32> {
    let entries = fs::read_dir(base).ok()?;
    let mut best: Option<(usize, i32)> = None;
    let mut fallback: Option<i32> = None;

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = read_sysfs_string(&p.join("name")).unwrap_or_default();
        if let Some(prio) = cpu_driver_priority(&name) {
            if let Some(temp) = read_hwmon_dir_temp(&p)
                && best.is_none_or(|(best_prio, _)| prio < best_prio)
            {
                best = Some((prio, temp));
            }
        } else if fallback.is_none() {
            fallback = read_hwmon_dir_temp(&p);
        }
    }

    best.map(|(_, t)| t).or(fallback)
}

pub fn read_cpu_temp() -> Option<i32> {
    read_cpu_temp_from_base(Path::new("/sys/class/hwmon"))
}
