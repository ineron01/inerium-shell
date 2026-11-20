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
