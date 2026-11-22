//! Inerium Dragon — Native Rust Battery Provider for Waybar
//! - Normal format: {icon} {capacity}%
//! - Extended format: {icon} {capacity}% · {time} (toggled by LKM click)
//! - Dynamic power_supply discovery (BAT* / Mains) with graceful desktop fallback
//! - Tooltip: Rich Pango markup with capacity, remaining time, power in W, health %
//! - Watch mode: Streams battery updates, instant reaction to SIGUSR1 on toggle

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use inerium_hw::battery::BatteryBackend;
use inerium_hw::sysfs::{read_sysfs_i64, read_sysfs_i64_opt, read_sysfs_str};
use inerium_i18n::{Lang, tr, tr_fmt};
use inerium_tools::format::escape_json;
use inerium_tools::ipc::{PidRegistry, SelfPipe};
use inerium_tools::sys::posix::SIGUSR1;

const BATTERY_ICONS: [&str; 10] = ["󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"];
const ICON_CHARGING: &str = "󰂄";
const ICON_PLUGGED: &str = "󰚥";

fn battery_registry() -> PidRegistry {
    PidRegistry::new("battery-alt")
}

fn is_alt_mode() -> bool {
    battery_registry().is_state_active()
}

fn toggle_alt_mode() {
    let reg = battery_registry();
    reg.toggle_state();
    let _ = reg.notify(SIGUSR1);
}

fn read_battery_threshold(bat_dir: &Path, backend: &BatteryBackend) -> i64 {
    if backend.has_threshold() {
        backend.get_threshold() as i64
    } else {
        read_sysfs_i64_opt(&bat_dir.join("charge_control_end_threshold"))
            .or_else(|| read_sysfs_i64_opt(&bat_dir.join("charge_stop_threshold")))
            .filter(|&v| (1..=100).contains(&v))
            .unwrap_or(100)
    }
}

struct BatteryInfo {
    present: bool,
    capacity: i64,
    status: String,
    ac_online: bool,
    power_w: f64,
    health_pct: f64,
    threshold: i64,
    time_str: String,
    icon: &'static str,
    total_now: i64,
    total_full: i64,
}

impl BatteryInfo {
    fn update_rates(&mut self, rate: f64, power_w: f64) {
        self.power_w = power_w;
        self.time_str = compute_time_str(
            &self.status,
            self.ac_online,
            self.total_now,
            self.total_full,
            rate,
            self.threshold,
        );
    }
}

fn compute_time_str(
    status: &str,
    ac_online: bool,
    total_now: i64,
    total_full: i64,
    rate: f64,
    threshold: i64,
) -> String {
    if rate <= 0.0 {
        return String::new();
    }
    if status == "Discharging" || (!ac_online && total_now > 0) {
        let hrs = total_now as f64 / rate;
        let h = hrs as i64;
        let m = ((hrs - h as f64) * 60.0) as i64;
        format!("{}{} {:02}{}", h, tr("h", "ч"), m, tr("m", "м"))
    } else if status == "Charging" {
        let target = if threshold < 100 {
            ((total_full as f64) * (threshold as f64 / 100.0)).round() as i64
        } else {
            total_full
        };
        if target > total_now {
            let rem = target - total_now;
            let hrs = rem as f64 / rate;
            let h = hrs as i64;
            let m = ((hrs - h as f64) * 60.0) as i64;
            format!("{}{} {:02}{} {} {}%", h, tr("h", "ч"), m, tr("m", "м"), tr("until", "до"), threshold)
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmaFilter {
    rate: Option<f64>,
    power: Option<f64>,
    last_status: String,
    last_ac: bool,
}

impl EmaFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, raw_rate: f64, raw_power: f64, status: &str, ac: bool) -> (f64, f64) {
        if self.last_status != status || self.last_ac != ac {
            self.rate = None;
            self.power = None;
            self.last_status = status.to_string();
            self.last_ac = ac;
        }

        const ALPHA: f64 = 0.12;

        let smoothed_rate = match self.rate {
            Some(prev) if raw_rate > 0.0 => ALPHA * raw_rate + (1.0 - ALPHA) * prev,
            _ => raw_rate,
        };

        let smoothed_power = match self.power {
            Some(prev) if raw_power > 0.0 => ALPHA * raw_power + (1.0 - ALPHA) * prev,
            _ => raw_power,
        };

        self.rate = Some(smoothed_rate);
        self.power = Some(smoothed_power);

        (smoothed_rate, smoothed_power)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BatteryCache {
    rate: f64,
    power: f64,
    status: String,
    timestamp: u64,
}

fn cache_path() -> std::path::PathBuf {
    crate::sys::runtime_dir().join("inerium-battery-rate.json")
}

fn save_smoothed_cache(rate: f64, power: f64, status: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cache = BatteryCache {
        rate,
        power,
        status: status.to_string(),
        timestamp: now,
    };
    if let Ok(data) = serde_json::to_string(&cache) {
        let _ = std::fs::write(cache_path(), data);
    }
}

fn load_smoothed_cache(current_status: &str) -> Option<(f64, f64)> {
    let content = std::fs::read_to_string(cache_path()).ok()?;
    let cache: BatteryCache = serde_json::from_str(&content).ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(cache.timestamp) <= 15 && cache.status == current_status {
        Some((cache.rate, cache.power))
    } else {
        None
    }
}

fn query_battery_raw() -> (BatteryInfo, f64, f64) {
    let backend = BatteryBackend::probe();
    query_battery_raw_with_backend(&backend)
}

fn scan_power_supplies() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut bats = Vec::new();
    let mut fallback_bats = Vec::new();
    let mut acs = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        dirs.sort();
        for path in dirs {
            let ptype = read_sysfs_str(&path.join("type"));
            if ptype == "Mains" || ptype == "USB" || ptype == "USB_C" || ptype == "USB_PD" {
                acs.push(path);
            } else if ptype == "Battery" {
                let scope = read_sysfs_str(&path.join("scope"));
                if scope == "Device" {
                    continue;
                }
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with("BAT") || name.starts_with("CMB") {
                    bats.push(path);
                } else {
                    fallback_bats.push(path);
                }
            }
        }
    }
    let final_bats = if !bats.is_empty() { bats } else { fallback_bats };
    (final_bats, acs)
}

fn empty_battery_info() -> (BatteryInfo, f64, f64) {
    (
        BatteryInfo {
            present: false,
            capacity: 0,
            status: String::new(),
            ac_online: true,
            power_w: 0.0,
            health_pct: 100.0,
            threshold: 100,
            time_str: String::new(),
            icon: ICON_PLUGGED,
            total_now: 0,
            total_full: 0,
        },
        0.0,
        0.0,
    )
}

fn query_battery_from_paths(
    bats: &[PathBuf],
    acs: &[PathBuf],
    backend: &BatteryBackend,
) -> Option<(BatteryInfo, f64, f64)> {
    if bats.is_empty() {
        return Some(empty_battery_info());
    }

    let mut ac_online = false;
    for ac in acs {
        if read_sysfs_i64(&ac.join("online")) == 1 {
            ac_online = true;
            break;
        }
    }

    let mut total_now = 0i64;
    let mut total_full = 0i64;
    let mut total_design = 0i64;
    let mut total_power_w = 0.0f64;
    let mut total_rate = 0i64;
    let mut min_threshold = 100i64;
    let mut has_charging = false;
    let mut has_discharging = false;
    let mut is_full = true;
    let mut cap_sum = 0i64;

    for bat_dir in bats {
        if !bat_dir.exists() {
            return None;
        }
        let cap = read_sysfs_i64(&bat_dir.join("capacity")).clamp(0, 100);
        cap_sum += cap;
        let st = read_sysfs_str(&bat_dir.join("status"));
        if st == "Charging" {
            has_charging = true;
            is_full = false;
        } else if st == "Discharging" {
            has_discharging = true;
            is_full = false;
        } else if st != "Full" && !st.is_empty() {
            is_full = false;
        }

        let (now, full, design, pw, rate) = if bat_dir.join("charge_now").exists() {
            let now = read_sysfs_i64(&bat_dir.join("charge_now"));
            let full = read_sysfs_i64(&bat_dir.join("charge_full"));
            let design = read_sysfs_i64(&bat_dir.join("charge_full_design")).max(1);
            let curr = read_sysfs_i64(&bat_dir.join("current_now")).abs();
            let volt = read_sysfs_i64(&bat_dir.join("voltage_now")).abs();
            let pw = (volt as f64 * curr as f64) / 1e12;
            (now, full, design, pw, curr)
        } else {
            let now = read_sysfs_i64(&bat_dir.join("energy_now"));
            let full = read_sysfs_i64(&bat_dir.join("energy_full"));
            let design = read_sysfs_i64(&bat_dir.join("energy_full_design")).max(1);
            let pnow = read_sysfs_i64(&bat_dir.join("power_now")).abs();
            let pw = pnow as f64 / 1e6;
            (now, full, design, pw, pnow)
        };

        total_now += now;
        total_full += full;
        total_design += design;
        total_power_w += pw;
        total_rate += rate;

        let th = read_battery_threshold(bat_dir, backend);
        if th < min_threshold {
            min_threshold = th;
        }
    }

    if has_charging {
        ac_online = true;
    }

    let overall_status = if has_charging {
        "Charging".to_string()
    } else if has_discharging {
        "Discharging".to_string()
    } else if is_full {
        "Full".to_string()
    } else if ac_online {
        "Not charging".to_string()
    } else {
        "Discharging".to_string()
    };

    let capacity = if total_full > 0 {
        ((total_now as f64 / total_full as f64) * 100.0).round().clamp(0.0, 100.0) as i64
    } else {
        (cap_sum / bats.len() as i64).clamp(0, 100)
    };

    let health_pct = if total_full > 0 && total_design > 0 {
        ((total_full as f64 / total_design as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        100.0
    };

    let threshold = min_threshold;
    let time_str = compute_time_str(
        &overall_status,
        ac_online,
        total_now,
        total_full,
        total_rate as f64,
        threshold,
    );

    let icon = if ac_online {
        if overall_status == "Charging" {
            ICON_CHARGING
        } else {
            ICON_PLUGGED
        }
    } else {
        let idx = ((capacity / 10).clamp(0, 9)) as usize;
        BATTERY_ICONS[idx]
    };

    Some((
        BatteryInfo {
            present: true,
            capacity,
            status: overall_status,
            ac_online,
            power_w: total_power_w,
            health_pct,
            threshold,
            time_str,
            icon,
            total_now,
            total_full,
        },
        total_rate as f64,
        total_power_w,
    ))
}

fn query_battery_raw_with_backend(backend: &BatteryBackend) -> (BatteryInfo, f64, f64) {
    let (bats, acs) = scan_power_supplies();
    query_battery_from_paths(&bats, &acs, backend).unwrap_or_else(empty_battery_info)
}

fn get_battery_info() -> BatteryInfo {
    let (mut info, raw_rate, raw_power) = query_battery_raw();
    if info.present {
        if let Some((smoothed_rate, smoothed_power)) = load_smoothed_cache(&info.status) {
            info.update_rates(smoothed_rate, smoothed_power);
        } else {
            info.update_rates(raw_rate, raw_power);
        }
    }
    info
}

fn format_battery(info: &BatteryInfo) -> String {
    if !info.present {
        return format!("{} {}", info.icon, tr("AC Power", "Сеть"));
    }
    if is_alt_mode() && !info.time_str.is_empty() {
        format!("{} {}% · {}", info.icon, info.capacity, info.time_str)
    } else {
        format!("{} {}%", info.icon, info.capacity)
    }
}

fn format_tooltip(info: &BatteryInfo) -> String {
    if !info.present {
        return tr("No battery detected (AC power)", "Батарея не обнаружена (питание от сети)").to_string();
    }
    let status_str = match info.status.as_str() {
        "Charging" => tr("Charging", "Заряжается"),
        "Discharging" => tr("Discharging", "Разряжается"),
        "Full" => tr("Full", "Полный заряд"),
        "Not charging" => tr("Not charging (limit)", "Не заряжается (лимит)"),
        _ => &info.status,
    };

    let ac_str = if info.ac_online {
        tr("Connected", "Подключен")
    } else {
        tr("Disconnected", "Отключен")
    };

    let mut lines = Vec::new();
    lines.push("<span font_desc='JetBrainsMono Nerd Font 10.5'>".to_string());
    lines.push(tr_fmt!(
        "<span color='#87a987' weight='bold'>󰁹 Battery: {cap}%</span>",
        "<span color='#87a987' weight='bold'>󰁹 Батарея: {cap}%</span>",
        cap = info.capacity,
    ));
    lines.push(format!(
        "<span color='#727169'>{:<14}</span><span color='#e6e2cc'>{}</span>",
        tr("Status:", "Статус:"),
        status_str
    ));
    lines.push(format!(
        "<span color='#727169'>{:<14}</span><span color='#8ba4b0'>{}</span>",
        tr("Power supply:", "Блок питания:"),
        ac_str
    ));

    if info.threshold < 100 {
        lines.push(format!(
            "<span color='#727169'>{:<14}</span><span color='#c4b28a'>{}%</span>",
            tr("Charge limit:", "Лимит заряда:"),
            info.threshold
        ));
    }

    if !info.time_str.is_empty() {
        let label = if info.status == "Charging" {
            if info.threshold < 100 {
                tr("Until limit:", "До лимита:")
            } else {
                tr("Until full:", "До полной:")
            }
        } else {
            tr("Remaining:", "Осталось:")
        };
        lines.push(format!(
            "<span color='#727169'>{:<14}</span><span color='#c4b28a'>{}</span>",
            label, info.time_str
        ));
    }

    if info.power_w > 0.05 {
        lines.push(format!(
            "<span color='#727169'>{:<14}</span><span color='#e6e2cc'>{:.1} {}</span>",
            tr("Power:", "Мощность:"),
            info.power_w,
            tr("W", "Вт")
        ));
    }

    lines.push(format!(
        "<span color='#727169'>{:<14}</span><span color='#87a987'>{:.1}%</span>",
        tr("Battery health:", "Здоровье АКБ:"),
        info.health_pct
    ));
    lines.push(String::new());
    lines.push(format!(
        "<span color='#727169' size='small'>{}</span>",
        tr("LMB: Toggle time / percentage", "ЛКМ: Переключить время / процент")
    ));
    lines.push("</span>".to_string());

    lines.join("\n")
}

fn format_json(info: &BatteryInfo) -> String {
    if !info.present {
        return "{\"text\":\"\",\"tooltip\":\"\",\"class\":[\"none\"]}".to_string();
    }
    let text = format_battery(info);
    let tooltip = format_tooltip(info);
    let mut class_list = vec!["\"battery\""];
    if info.ac_online {
        if info.status == "Charging" {
            class_list.push("\"charging\"");
        } else {
            class_list.push("\"plugged\"");
        }
    } else if info.capacity <= 15 {
        class_list.push("\"critical\"");
    } else if info.capacity <= 30 {
        class_list.push("\"warning\"");
    }
    format!(
        "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":[{}]}}",
        escape_json(&text),
        escape_json(&tooltip),
        class_list.join(",")
    )
}

fn run_watch(waybar: bool) {
    let reg = battery_registry();
    let _pid_guard = reg.register();
    let self_pipe = match SelfPipe::new() {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("Failed to create self pipe: {e}");
            return;
        }
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut last_output = String::new();
    let mut filter = EmaFilter::new();
    let backend = BatteryBackend::probe();

    let mut cached_paths: Option<(Vec<PathBuf>, Vec<PathBuf>)> = None;
    let mut last_scan = Instant::now() - Duration::from_secs(60);
    let mut last_saved_rate = -1.0;
    let mut last_saved_status = String::new();
    let mut last_saved_time = Instant::now() - Duration::from_secs(60);

    loop {
        if cached_paths.is_none() || last_scan.elapsed() >= Duration::from_secs(30) {
            cached_paths = Some(scan_power_supplies());
            last_scan = Instant::now();
        }

        let (mut info, raw_rate, raw_power) = {
            let (bats, acs) = cached_paths.as_ref().unwrap();
            match query_battery_from_paths(bats, acs, &backend) {
                Some(res) => res,
                None => {
                    let (new_bats, new_acs) = scan_power_supplies();
                    let res = query_battery_from_paths(&new_bats, &new_acs, &backend)
                        .unwrap_or_else(empty_battery_info);
                    cached_paths = Some((new_bats, new_acs));
                    last_scan = Instant::now();
                    res
                }
            }
        };

        if info.present {
            let (smoothed_rate, smoothed_power) =
                filter.update(raw_rate, raw_power, &info.status, info.ac_online);
            info.update_rates(smoothed_rate, smoothed_power);

            let status_changed = info.status != last_saved_status;
            let rate_diff_pct = if last_saved_rate > 0.0 {
                ((smoothed_rate - last_saved_rate).abs() / last_saved_rate) * 100.0
            } else if smoothed_rate > 0.0 {
                100.0
            } else {
                0.0
            };
            let rate_changed = rate_diff_pct > 2.0;
            let time_elapsed = last_saved_time.elapsed() >= Duration::from_secs(10);

            if status_changed || rate_changed || time_elapsed {
                save_smoothed_cache(smoothed_rate, smoothed_power, &info.status);
                last_saved_rate = smoothed_rate;
                last_saved_status = info.status.clone();
                last_saved_time = Instant::now();
            }
        }

        let current = if waybar { format_json(&info) } else { format_battery(&info) };
        if current != last_output {
            let _ = writeln!(handle, "{}", current);
            let _ = handle.flush();
            last_output = current;
        }

        if self_pipe.wait_event(2000) {
            Lang::reinit();
        }
    }
}

pub fn run(args: &[String]) {
    if args.len() > 1 {
        match args[1].as_str() {
            "--toggle" => {
                toggle_alt_mode();
                let info = get_battery_info();
                println!("{}", format_battery(&info));
                return;
            }
            "--tooltip" => {
                let info = get_battery_info();
                println!("{}", format_tooltip(&info));
                return;
            }
            "--waybar" => {
                run_watch(true);
                return;
            }
            "--watch" => {
                let waybar = args.iter().any(|a| a == "--json" || a == "--waybar");
                run_watch(waybar);
                return;
            }
            _ => {}
        }
    }
    let info = get_battery_info();
    println!("{}", format_battery(&info));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use inerium_hw::battery::discover_power_supplies_in;

    #[test]
    fn test_desktop_without_battery_hides_waybar_module() {
        let (bats, ac) = discover_power_supplies_in(Path::new("/nonexistent/power_supply"));
        assert!(bats.is_empty());
        assert!(!ac);
        let info = BatteryInfo {
            present: false,
            capacity: 0,
            status: String::new(),
            ac_online: true,
            power_w: 0.0,
            health_pct: 100.0,
            threshold: 100,
            time_str: String::new(),
            icon: ICON_PLUGGED,
            total_now: 0,
            total_full: 0,
        };
        assert_eq!(
            format_json(&info),
            "{\"text\":\"\",\"tooltip\":\"\",\"class\":[\"none\"]}"
        );
    }

    #[test]
    fn test_charging_time_calculation_with_threshold_80() {
        let full = 4_000_000i64;
        let now = 2_000_000i64;
        let rate = 2_000_000i64;
        let threshold = 80i64;
        let target = ((full as f64) * (threshold as f64 / 100.0)).round() as i64;
        assert_eq!(target, 3_200_000);
        let rem = target - now;
        assert_eq!(rem, 1_200_000);
        let hrs = rem as f64 / rate as f64;
        let h = hrs as i64;
        let m = ((hrs - h as f64) * 60.0) as i64;
        let time_str = format!("{}ч {:02}м до {}%", h, m, threshold);
        assert_eq!(time_str, "0ч 36м до 80%");
    }

    #[test]
    fn test_charging_time_calculation_full_100() {
        let full = 4_000_000i64;
        let now = 2_000_000i64;
        let rate = 2_000_000i64;
        let threshold = 100i64;
        let target = full;
        let rem = target - now;
        let hrs = rem as f64 / rate as f64;
        let h = hrs as i64;
        let m = ((hrs - h as f64) * 60.0) as i64;
        let time_str = format!("{}ч {:02}м до {}%", h, m, threshold);
        assert_eq!(time_str, "1ч 00м до 100%");
    }

    #[test]
    fn test_tooltip_includes_threshold_and_label() {
        Lang::set(Lang::Ru);
        let info = BatteryInfo {
            present: true,
            capacity: 55,
            status: "Charging".to_string(),
            ac_online: true,
            power_w: 25.0,
            health_pct: 95.0,
            threshold: 80,
            time_str: "0ч 30м до 80%".to_string(),
            icon: ICON_CHARGING,
            total_now: 0,
            total_full: 0,
        };
        let tip = format_tooltip(&info);
        assert!(tip.contains("Лимит заряда:"));
        assert!(tip.contains("80%"));
        assert!(tip.contains("До лимита:"));
        assert!(tip.contains("0ч 30м до 80%"));
    }

    #[test]
    fn test_type_c_pd_and_multi_battery_detection() {
        let root = std::env::temp_dir().join(format!("inerium_bat_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let usb_pd = root.join("ucsi-source-psy-USBC000:001");
        fs::create_dir_all(&usb_pd).unwrap();
        fs::write(usb_pd.join("type"), "USB_PD\n").unwrap();
        fs::write(usb_pd.join("online"), "1\n").unwrap();

        let bat0 = root.join("BAT0");
        let bat1 = root.join("BAT1");
        fs::create_dir_all(&bat0).unwrap();
        fs::create_dir_all(&bat1).unwrap();
        fs::write(bat0.join("type"), "Battery\n").unwrap();
        fs::write(bat1.join("type"), "Battery\n").unwrap();

        let (bats, ac) = discover_power_supplies_in(&root);
        assert!(ac);
        assert_eq!(bats.len(), 2);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_ema_filter_smoothing_and_reset() {
        let mut filter = EmaFilter::new();

        // 1. Initial sample sets baseline directly
        let (r1, p1) = filter.update(1000.0, 10.0, "Discharging", false);
        assert_eq!(r1, 1000.0);
        assert_eq!(p1, 10.0);

        // 2. Next sample is smoothed
        let (r2, p2) = filter.update(2000.0, 20.0, "Discharging", false);
        // ALPHA = 0.12 => 0.12 * 2000 + 0.88 * 1000 = 240 + 880 = 1120.0
        assert!((r2 - 1120.0).abs() < 1e-4);
        assert!((p2 - 11.2).abs() < 1e-4);

        // 3. Status change resets filter immediately
        let (r3, p3) = filter.update(3000.0, 30.0, "Charging", true);
        assert_eq!(r3, 3000.0);
        assert_eq!(p3, 30.0);
    }
}
