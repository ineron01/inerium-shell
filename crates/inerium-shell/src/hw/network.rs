use std::fs;
use std::process::{Command, Stdio};
use std::thread;

pub fn is_wifi_enabled() -> bool {
    if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
        return String::from_utf8_lossy(&output.stdout).trim() == "enabled";
    }
    true
}

pub fn split_nmcli_escaped(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                if next == ':' || next == '\\' {
                    current.push(next);
                    chars.next();
                } else {
                    current.push('\\');
                }
            } else {
                current.push('\\');
            }
        } else if ch == ':' {
            fields.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiConnStatus {
    pub active_ssid: Option<String>,
    pub connecting_ssid: Option<String>,
}

pub fn get_wifi_connection_status() -> WifiConnStatus {
    let mut active = None;
    let mut connecting = None;
    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,NAME,STATE", "connection", "show", "--active"])
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts = split_nmcli_escaped(line);
            if parts.len() >= 3 && (parts[0] == "802-11-wireless" || parts[0] == "wifi") {
                let name = parts[1].trim().to_string();
                let state = parts[2].trim();
                if !name.is_empty() {
                    if state == "activated" {
                        active = Some(name);
                    } else if state == "activating" {
                        connecting = Some(name);
                    }
                }
            }
        }
    }
    WifiConnStatus {
        active_ssid: active,
        connecting_ssid: connecting,
    }
}

pub fn get_active_wifi_ssid() -> Option<String> {
    get_wifi_connection_status().active_ssid
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NetworkConnectivity {
    Full,
    Portal,
    Limited,
    #[default]
    Unknown,
}

impl NetworkConnectivity {
    fn from_nmcli(raw: &str) -> Self {
        match raw.trim() {
            "full" => Self::Full,
            "portal" => Self::Portal,
            "limited" => Self::Limited,
            _ => Self::Unknown,
        }
    }

    pub fn needs_portal_login(self) -> bool {
        matches!(self, Self::Portal | Self::Limited)
    }
}

pub fn get_network_connectivity() -> NetworkConnectivity {
    if let Ok(out) = Command::new("nmcli")
        .env("LC_ALL", "C")
        .args(["-t", "-f", "CONNECTIVITY", "general"])
        .stdin(Stdio::null())
        .output()
        && out.status.success()
    {
        return NetworkConnectivity::from_nmcli(&String::from_utf8_lossy(&out.stdout));
    }
    NetworkConnectivity::Unknown
}

pub fn refresh_network_connectivity() -> NetworkConnectivity {
    if let Ok(out) = Command::new("timeout")
        .env("LC_ALL", "C")
        .args(["5", "nmcli", "networking", "connectivity", "check"])
        .stdin(Stdio::null())
        .output()
        && out.status.success()
    {
        return NetworkConnectivity::from_nmcli(&String::from_utf8_lossy(&out.stdout));
    }
    get_network_connectivity()
}

pub fn open_captive_portal() {
    let _ = Command::new("xdg-open")
        .arg("http://detectportal.firefox.com/canonical.html")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn notify_network_changed() {
    let _ = inerium_tools::ipc::PidRegistry::notify_all("network", inerium_tools::sys::posix::SIGUSR1);
}

pub fn set_wifi_enabled_sync(enable: bool) {
    let val = if enable { "on" } else { "off" };
    let _ = Command::new("nmcli")
        .args(["radio", "wifi", val])
        .stdin(Stdio::null())
        .output();
    notify_network_changed();
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
            let parts = split_nmcli_escaped(line);
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
            let parts = split_nmcli_escaped(line);
            if parts.len() >= 2 && (parts[0] == "802-11-wireless" || parts[0] == "wifi") {
                let name = parts[1].trim().to_string();
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
    let _ = get_active_wifi_ssid()?;
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

pub fn connect_enterprise_wifi(
    ssid: &str,
    identity: &str,
    password: &str,
    no_ca_cert: bool,
) -> Result<(), String> {
    let exists = Command::new("nmcli")
        .args(["-t", "-f", "NAME", "connection", "show"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == ssid)
        })
        .unwrap_or(false);

    let cfg_ok = if exists {
        let mut mod_args = vec![
            "connection".to_string(),
            "modify".to_string(),
            "id".to_string(),
            ssid.to_string(),
            "wifi-sec.key-mgmt".to_string(),
            "wpa-eap".to_string(),
            "802-1x.eap".to_string(),
            "peap".to_string(),
            "802-1x.phase2-auth".to_string(),
            "mschapv2".to_string(),
            "802-1x.identity".to_string(),
            identity.to_string(),
            "802-1x.password".to_string(),
            password.to_string(),
        ];
        if no_ca_cert {
            mod_args.push("802-1x.ca-cert".to_string());
            mod_args.push("".to_string());
        }
        Command::new("nmcli")
            .args(&mod_args)
            .stdin(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        let mut add_args = vec![
            "connection".to_string(),
            "add".to_string(),
            "type".to_string(),
            "wifi".to_string(),
            "con-name".to_string(),
            ssid.to_string(),
            "ifname".to_string(),
            "*".to_string(),
            "ssid".to_string(),
            ssid.to_string(),
            "wifi-sec.key-mgmt".to_string(),
            "wpa-eap".to_string(),
            "802-1x.eap".to_string(),
            "peap".to_string(),
            "802-1x.phase2-auth".to_string(),
            "mschapv2".to_string(),
            "802-1x.identity".to_string(),
            identity.to_string(),
            "802-1x.password".to_string(),
            password.to_string(),
        ];
        if no_ca_cert {
            add_args.push("802-1x.ca-cert".to_string());
            add_args.push("".to_string());
        }
        Command::new("nmcli")
            .args(&add_args)
            .stdin(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if !cfg_ok {
        return Err("Failed to configure 802.1X enterprise profile".to_string());
    }

    let up_res = Command::new("nmcli")
        .args(["--wait", "15", "connection", "up", "id", ssid])
        .stdin(Stdio::null())
        .output();

    match up_res {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg = if err_msg.is_empty() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                err_msg
            };
            Err(msg)
        }
        Err(e) => Err(e.to_string()),
    }
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
            } else if trimmed.starts_with("Battery Percentage:")
                && let Some((_, rest)) = trimmed.split_once('(')
                && let Some((num_str, _)) = rest.split_once(')')
                && let Ok(pct) = num_str.trim().parse::<u8>()
            {
                battery = Some(pct);
            }
        }
        assert!(trusted);
        assert_eq!(battery, Some(85));
    }

    #[test]
    fn test_split_nmcli_escaped_ssid() {
        let line = "*:My\\:WiFi\\\\\\:Net:80:WPA2";
        let fields = split_nmcli_escaped(line);
        assert_eq!(fields.len(), 4);
        assert_eq!(fields[0], "*");
        assert_eq!(fields[1], "My:WiFi\\:Net");
        assert_eq!(fields[2], "80");
        assert_eq!(fields[3], "WPA2");
    }
}
