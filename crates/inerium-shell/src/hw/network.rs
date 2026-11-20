use std::fs;
use std::process::{Command, Stdio};
use std::thread;

pub fn is_wifi_enabled() -> bool {
    if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
        return String::from_utf8_lossy(&output.stdout).trim() == "enabled";
    }
    true
}

pub fn get_active_wifi_ssid() -> Option<String> {
    // 1. Fast direct check via iw link (2 ms)
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let iface = entry.file_name().to_string_lossy().into_owned();
            if iface.starts_with("wl")
                && let Ok(out) = Command::new("iw").args(["dev", &iface, "link"]).output() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("SSID: ") {
                            let s = rest.trim();
                            if !s.is_empty() {
                                return Some(s.to_string());
                            }
                        }
                    }
                }
        }
    }

    // 2. Fallback check via nmcli
    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,NAME", "connection", "show", "--active"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && (parts[0] == "802-11-wireless" || parts[0] == "wifi") {
                let name = parts[1].replace("\\:", ":").trim().to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }

    None
}

pub fn set_wifi_enabled_sync(enable: bool) {
    let val = if enable { "on" } else { "off" };
    let _ = Command::new("nmcli")
        .args(["radio", "wifi", val])
        .stdin(Stdio::null())
        .output();
}

pub fn toggle_wifi(enable: bool) {
    thread::spawn(move || {
        set_wifi_enabled_sync(enable);
    });
}

pub fn is_wifi_device_ready() -> bool {
    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,STATE", "dev", "status"])
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[0] == "wifi" {
                let state = parts[1].trim();
                if state != "unavailable" && state != "unmanaged" {
                    return true;
                }
            }
        }
    }
    false
}

pub fn wait_for_wifi_ready(timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if is_wifi_device_ready() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    is_wifi_device_ready()
}

pub fn is_hotspot_active() -> bool {
    // 1. Check AP mode via iw (1 ms)
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let iface = entry.file_name().to_string_lossy().into_owned();
            if iface.starts_with("wl")
                && let Ok(out) = Command::new("iw").args(["dev", &iface, "info"]).output()
                    && String::from_utf8_lossy(&out.stdout).contains("type AP") {
                        return true;
                    }
        }
    }

    // 2. Check active hotspot connection via nmcli
    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,NAME", "connection", "show", "--active"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && (parts[0] == "802-11-wireless" || parts[0] == "wifi") {
                let name = parts[1].replace("\\:", ":").trim().to_string();
                if name == "Hotspot" || name == "inerium-hotspot" {
                    return true;
                }
            }
        }
    }

    false
}

pub fn has_bluetooth() -> bool {
    fs::read_dir("/sys/class/bluetooth")
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

pub fn is_bluetooth_enabled() -> bool {
    if !has_bluetooth() {
        return false;
    }

    // 1. Check rfkill
    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            let tpath = entry.path().join("type");
            if let Ok(t) = fs::read_to_string(&tpath)
                && t.trim() == "bluetooth" {
                    let spath = entry.path().join("state");
                    if let Ok(s) = fs::read_to_string(&spath)
                        && s.trim() == "0" {
                            return false;
                        }
                }
        }
    }

    // 2. Check bluetoothctl show with external timeout
    if let Ok(output) = Command::new("timeout")
        .args(["2", "bluetoothctl", "show"])
        .stdin(Stdio::null())
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).contains("Powered: yes");
    }
    false
}

pub fn wait_for_bluetooth_ready(timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if is_bluetooth_enabled() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    is_bluetooth_enabled()
}

pub fn set_bluetooth_enabled_sync(enable: bool) {
    if enable {
        let _ = Command::new("rfkill")
            .args(["unblock", "bluetooth"])
            .stdin(Stdio::null())
            .output();
        for _ in 0..10 {
            if let Ok(out) = Command::new("timeout")
                .args(["2", "bluetoothctl", "show"])
                .stdin(Stdio::null())
                .output()
                && String::from_utf8_lossy(&out.stdout).contains("Powered: yes")
            {
                break;
            }
            let _ = Command::new("timeout")
                .args(["2", "bluetoothctl", "power", "on"])
                .stdin(Stdio::null())
                .output();
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    } else {
        let _ = Command::new("timeout")
            .args(["2", "bluetoothctl", "power", "off"])
            .stdin(Stdio::null())
            .output();
    }
    // Linux TASK_COMM_LEN is 15 chars: inerium-bluetooth is stored as inerium-bluetoo in comm
    let _ = Command::new("pkill").args(["-SIGUSR1", "-x", "inerium-bluetoo"]).output();
}

pub fn toggle_bluetooth(enable: bool) {
    thread::spawn(move || {
        set_bluetooth_enabled_sync(enable);
    });
}

#[derive(Clone, Debug, Default)]
pub struct ActiveWifiDetails {
    pub ip: String,
    pub gateway: String,
    pub band: String,
    pub freq_mhz: u32,
    pub signal_dbm: i32,
}

pub fn get_active_wifi_details() -> Option<ActiveWifiDetails> {
    let mut iface_opt = None;
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("wl") {
                iface_opt = Some(name);
                break;
            }
        }
    }
    let iface = iface_opt?;

    let out = Command::new("iw")
        .args(["dev", &iface, "link"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !text.contains("Connected to") {
        return None;
    }

    let mut freq_mhz = 0;
    let mut signal_dbm = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("freq:") {
            let f_str = rest.trim();
            if let Some(f_val) = f_str.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
                freq_mhz = f_val;
            }
        } else if let Some(rest) = trimmed.strip_prefix("signal:") {
            let s_str = rest.trim();
            if let Some(s_val) = s_str.split_whitespace().next().and_then(|s| s.parse::<i32>().ok()) {
                signal_dbm = s_val;
            }
        }
    }

    let band = if freq_mhz >= 5900 {
        "6 GHz".to_string()
    } else if freq_mhz >= 4900 {
        "5 GHz".to_string()
    } else if freq_mhz > 0 {
        "2.4 GHz".to_string()
    } else {
        String::new()
    };

    let mut ip = String::new();
    let mut gateway = String::new();

    if let Ok(nm_out) = Command::new("nmcli")
        .args(["-t", "-f", "IP4.ADDRESS,IP4.GATEWAY", "dev", "show", &iface])
        .output()
    {
        let nm_text = String::from_utf8_lossy(&nm_out.stdout);
        for line in nm_text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("IP4.ADDRESS") {
                if let Some(val) = trimmed.split(':').nth(1) {
                    let clean_ip = val.split('/').next().unwrap_or(val).trim();
                    if !clean_ip.is_empty() && ip.is_empty() {
                        ip = clean_ip.to_string();
                    }
                }
            } else if let Some(val) = trimmed.strip_prefix("IP4.GATEWAY:") {
                let clean_gw = val.trim();
                if !clean_gw.is_empty() {
                    gateway = clean_gw.to_string();
                }
            }
        }
    }

    Some(ActiveWifiDetails {
        ip,
        gateway,
        band,
        freq_mhz,
        signal_dbm,
    })
}

pub fn set_connection_autoconnect(uuid: &str, enable: bool) -> bool {
    let val = if enable { "yes" } else { "no" };
    Command::new("nmcli")
        .args(["connection", "modify", uuid, "connection.autoconnect", val])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn delete_connection(uuid: &str) -> bool {
    Command::new("nmcli")
        .args(["connection", "delete", "uuid", uuid])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn get_bluetooth_device_meta(mac: &str) -> (bool, Option<u8>) {
    let mut trusted = false;
    let mut battery = None;

    if let Ok(out) = Command::new("timeout")
        .args(["2", "bluetoothctl", "info", mac])
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Trusted:") {
                trusted = trimmed.contains("yes");
            } else if trimmed.starts_with("Battery Percentage:") {
                if let Some((_, rest)) = trimmed.split_once('(')
                    && let Some((num_str, _)) = rest.split_once(')')
                    && let Ok(pct) = num_str.trim().parse::<u8>()
                {
                    battery = Some(pct);
                } else if let Some(rest) = trimmed.strip_prefix("Battery Percentage:") {
                    let cleaned = rest.trim().trim_end_matches('%');
                    if let Ok(pct) = cleaned.parse::<u8>() {
                        battery = Some(pct);
                    }
                }
            }
        }
    }

    (trusted, battery)
}

pub fn set_bluetooth_trusted(mac: &str, trust: bool) -> bool {
    let cmd = if trust { "trust" } else { "untrust" };
    Command::new("timeout")
        .args(["2", "bluetoothctl", cmd, mac])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn remove_bluetooth_device(mac: &str) -> bool {
    Command::new("timeout")
        .args(["2", "bluetoothctl", "remove", mac])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_wifi_details_query() {
        if is_wifi_enabled() && get_active_wifi_ssid().is_some() {
            let details = get_active_wifi_details();
            assert!(details.is_some(), "Expected active wifi details when connected");
            let d = details.unwrap();
            assert!(!d.ip.is_empty(), "Expected non-empty IP");
            assert!(!d.band.is_empty(), "Expected non-empty band");
        }
    }

    #[test]
    fn test_battery_percentage_parsing() {
        let sample_out = "Device 14:6C:27:15:FB:4F\n\tName: Redmi Buds 6 Active\n\tTrusted: yes\n\tBattery Percentage: 0x55 (85)\n";
        let mut trusted = false;
        let mut battery = None;
        for line in sample_out.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Trusted:") {
                trusted = trimmed.contains("yes");
            } else if trimmed.starts_with("Battery Percentage:") {
                if let Some((_, rest)) = trimmed.split_once('(') {
                    if let Some((num_str, _)) = rest.split_once(')') {
                        if let Ok(pct) = num_str.trim().parse::<u8>() {
                            battery = Some(pct);
                        }
                    }
                }
            }
        }
        assert!(trusted);
        assert_eq!(battery, Some(85));
    }
}
