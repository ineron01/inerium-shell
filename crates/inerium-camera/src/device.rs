use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

const ADB_TCP_PORT: u16 = 5555;
/// Last ADB Wi-Fi endpoint (ip:port) that worked, relative to $HOME
const LAST_WIFI_TARGET_FILE: &str = ".cache/inerium-camera/last-wifi-device";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinStatus {
    Available {
        device_path: String,
    },
    HardwareDisabled, // Fn + F6 toggled off
    NotFound,
}

pub struct DeviceManager;

impl DeviceManager {
    /// Detect built-in laptop webcam and Fn+F6 state dynamically
    pub fn detect_builtin_webcam() -> BuiltinStatus {
        let Ok(entries) = fs::read_dir("/sys/class/video4linux") else {
            return BuiltinStatus::NotFound;
        };

        let mut devs: Vec<_> = entries.flatten().collect();
        devs.sort_by_key(|e| e.file_name());

        for entry in devs {
            let sys_path = entry.path();
            // Skip virtual v4l2loopback devices dynamically by checking real sysfs path
            if let Ok(canon) = fs::canonicalize(&sys_path) {
                if canon.to_string_lossy().contains("/devices/virtual/") {
                    continue;
                }
            }

            let name_file = sys_path.join("name");
            let name = fs::read_to_string(&name_file).unwrap_or_default();
            if name.contains("Loopback") || name.contains("PhoneWebcam") {
                continue;
            }

            let dev_name = entry.file_name().to_string_lossy().to_string();
            let dev_path = format!("/dev/{}", dev_name);

            // Check Device Caps so metadata-only nodes (e.g. video2/video4) are excluded
            let output = Command::new("v4l2-ctl")
                .args(["-d", &dev_path, "--info"])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let info_str = String::from_utf8_lossy(&out.stdout);
                    if info_str.contains("v4l2 loopback") {
                        continue;
                    }
                    let device_caps = info_str.split("Device Caps").nth(1).unwrap_or("");
                    if device_caps.contains("Video Capture") {
                        return BuiltinStatus::Available {
                            device_path: dev_path,
                        };
                    }
                }
            }
        }
        BuiltinStatus::HardwareDisabled
    }

    /// Check if physical V4L2 device supports 1280x720 MJPG stream (preferred 30fps mode)
    pub fn supports_mjpg_720p(dev_path: &str) -> bool {
        let Ok(out) = Command::new("v4l2-ctl")
            .args(["-d", dev_path, "--list-formats-ext"])
            .output()
        else {
            return false;
        };
        let s = String::from_utf8_lossy(&out.stdout);
        let mut in_mjpg = false;
        for line in s.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_mjpg = trimmed.contains("'MJPG'");
            } else if in_mjpg && trimmed.contains("1280x720") {
                return true;
            }
        }
        false
    }

    /// Locate virtual V4L2 loopback device (PhoneWebcam)
    pub fn find_v4l2_loopback() -> Option<String> {
        if let Ok(entries) = fs::read_dir("/sys/class/video4linux") {
            let mut devs: Vec<_> = entries.flatten().collect();
            devs.sort_by_key(|e| e.file_name());
            for entry in devs {
                let sys_path = entry.path();
                let is_virtual = fs::canonicalize(&sys_path)
                    .map(|p| p.to_string_lossy().contains("/devices/virtual/"))
                    .unwrap_or(false);
                let name = fs::read_to_string(sys_path.join("name")).unwrap_or_default();
                let trimmed = name.trim();
                if is_virtual
                    || trimmed.contains("PhoneWebcam")
                    || trimmed.contains("Loopback")
                    || trimmed.contains("v4l2 loopback")
                {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let dev_path = format!("/dev/{}", file_name);
                    if Path::new(&dev_path).exists() {
                        return Some(dev_path);
                    }
                }
            }
        }

        None
    }

    /// Discover external device (ADB USB, live ADB Wi-Fi, LAN or hotspot 10.42.0.x)
    pub async fn discover_external_device(custom_ip: Option<&str>) -> Result<(String, String), String> {
        if let Some(ip) = custom_ip {
            let ip = ip.trim();
            let target = if ip.contains(':') { ip.to_string() } else { format!("{}:{}", ip, ADB_TCP_PORT) };
            if Self::adb_connect_verified(&target).await {
                Self::remember_wifi_target(&target);
                return Ok((target, format!("Wi-Fi (Ручной адрес: {})", ip)));
            }
            return Err(format!("Не удалось подключиться к {} по ADB.", target));
        }

        // 1. Check direct USB connection first (fastest and lowest latency)
        let adb_l = tokio::process::Command::new("adb")
            .args(["devices", "-l"])
            .output()
            .await
            .map_err(|e| format!("Ошибка запуска adb: {}", e))?;

        let adb_str = String::from_utf8_lossy(&adb_l.stdout);
        for line in adb_str.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("List of") {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                let serial = parts[0];
                if line.contains("usb:") || !serial.contains(':') {
                    let desc = if let Some(model_part) = parts.iter().find(|p| p.starts_with("model:")) {
                        let model = model_part.trim_start_matches("model:");
                        format!("USB кабель (Модель: {})", model)
                    } else {
                        "USB кабель (минимальная задержка)".to_string()
                    };
                    return Ok((serial.to_string(), desc));
                }
            }
        }

        // 2. Check already connected ADB over Wi-Fi, verifying live responsiveness (get-state caches dead sockets)
        for line in adb_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0].contains(':') {
                let ip_port = parts[0];
                if parts[1] == "device" && Self::adb_is_responsive(ip_port).await {
                    Self::remember_wifi_target(ip_port);
                    return Ok((ip_port.to_string(), Self::wifi_description(ip_port)));
                }
                Self::adb_disconnect(ip_port).await;
            }
        }

        // 3. Auto-discover phone over Wi-Fi (last known address, mDNS, ARP neighbours, LAN/hotspot sweep)
        if let Some(target) = Self::discover_wifi_device().await {
            return Ok((target.clone(), Self::wifi_description(&target)));
        }

        Err("Устройство не обнаружено.\nПодключите кабель USB, либо подключите телефон к той же Wi-Fi сети или к точке доступа ноутбука (на телефоне должна быть включена отладка по ADB TCP/IP 5555).".to_string())
    }

    fn wifi_description(target: &str) -> String {
        if target.starts_with("10.42.0.") {
            format!("Wi-Fi (Точка доступа {})", target)
        } else {
            format!("Wi-Fi (Локальная сеть {})", target)
        }
    }

    fn last_wifi_target_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(LAST_WIFI_TARGET_FILE))
    }

    fn load_last_wifi_target() -> Option<String> {
        let path = Self::last_wifi_target_path()?;
        let target = fs::read_to_string(path).ok()?.trim().to_string();
        (!target.is_empty()).then_some(target)
    }

    fn remember_wifi_target(target: &str) {
        if let Some(path) = Self::last_wifi_target_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(path, format!("{}\n", target));
        }
    }

    /// True only if the device actually answers an ADB round-trip (unlike `get-state`, which returns cached state)
    async fn adb_is_responsive(target: &str) -> bool {
        matches!(
            timeout(
                Duration::from_millis(1800),
                tokio::process::Command::new("adb")
                    .args(["-s", target, "shell", "true"])
                    .output(),
            )
            .await,
            Ok(Ok(out)) if out.status.success()
        )
    }

    async fn adb_disconnect(target: &str) {
        let _ = timeout(
            Duration::from_secs(1),
            tokio::process::Command::new("adb")
                .args(["disconnect", target])
                .output(),
        )
        .await;
    }

    async fn tcp_port_open(target: &str, dur: Duration) -> bool {
        matches!(timeout(dur, TcpStream::connect(target)).await, Ok(Ok(_)))
    }

    /// `adb connect` and confirm live responsiveness, cleaning up stale transports on failure
    async fn adb_connect_verified(target: &str) -> bool {
        if !Self::tcp_port_open(target, Duration::from_millis(900)).await {
            Self::adb_disconnect(target).await;
            return false;
        }

        let _ = timeout(
            Duration::from_secs(4),
            tokio::process::Command::new("adb").args(["connect", target]).output(),
        )
        .await;

        for attempt in 0..4 {
            if attempt > 0 {
                sleep(Duration::from_millis(200)).await;
            }
            if Self::adb_is_responsive(target).await {
                return true;
            }
        }
        Self::adb_disconnect(target).await;
        false
    }

    /// Find the phone over Wi-Fi on any physical network (home LAN or laptop hotspot) and connect ADB
    pub async fn discover_wifi_device() -> Option<String> {
        let mut tried: Vec<String> = Vec::new();

        // 1. Verify existing ADB Wi-Fi transports and evict stale ones (e.g. after switching LAN <-> hotspot)
        if let Ok(Ok(adb_l)) = timeout(
            Duration::from_secs(2),
            tokio::process::Command::new("adb").args(["devices", "-l"]).output(),
        )
        .await
        {
            for line in String::from_utf8_lossy(&adb_l.stdout).lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].contains(':') {
                    let target = parts[0];
                    if parts[1] == "device" && Self::adb_is_responsive(target).await {
                        Self::remember_wifi_target(target);
                        return Some(target.to_string());
                    }
                    Self::adb_disconnect(target).await;
                    tried.push(target.to_string());
                }
            }
        }

        // 2. Last successfully used address (fast TCP check avoids 4s stall if on a different network)
        if let Some(target) = Self::load_last_wifi_target() {
            if !tried.iter().any(|t| t == &target) {
                if Self::adb_connect_verified(&target).await {
                    return Some(target);
                }
                tried.push(target);
            }
        }

        // 3. Android 11+ wireless debugging advertised via mDNS (works for already paired phones)
        if let Ok(Ok(out)) = timeout(
            Duration::from_secs(3),
            tokio::process::Command::new("adb").args(["mdns", "services"]).output(),
        )
        .await
        {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines().filter(|l| l.contains("_adb-tls-connect._tcp")) {
                if let Some(target) = line.split_whitespace().last() {
                    if !tried.iter().any(|t| t == target) && Self::adb_connect_verified(target).await {
                        Self::remember_wifi_target(target);
                        return Some(target.to_string());
                    }
                    tried.push(target.to_string());
                }
            }
        }

        // 4. Probe live ARP neighbours on physical interfaces first (finds hotspot/LAN clients in <1s)
        let neigh_ips = Self::physical_neighbours();
        for ip in Self::probe_adb_port(&neigh_ips, Duration::from_millis(900)).await {
            let target = format!("{}:{}", ip, ADB_TCP_PORT);
            if tried.contains(&target) {
                continue;
            }
            if Self::adb_connect_verified(&target).await {
                Self::remember_wifi_target(&target);
                return Some(target);
            }
            tried.push(target);
        }

        // 5. Fallback: sweep ADB TCP port on every physical subnet (LAN Wi-Fi/Ethernet and hotspot 10.42.0.0/24)
        for ip in Self::scan_adb_hosts().await {
            let target = format!("{}:{}", ip, ADB_TCP_PORT);
            if tried.contains(&target) {
                continue;
            }
            if Self::adb_connect_verified(&target).await {
                Self::remember_wifi_target(&target);
                return Some(target);
            }
            tried.push(target);
        }

        None
    }

    fn is_physical_iface(iface: &str) -> bool {
        Path::new("/sys/class/net").join(iface).join("device").exists()
    }

    /// Active ARP neighbours on physical interfaces, prioritising hotspot (10.42.0.x) and REACHABLE hosts
    fn physical_neighbours() -> Vec<Ipv4Addr> {
        let Ok(out) = Command::new("ip").args(["-4", "neigh", "show"]).output() else {
            return Vec::new();
        };

        let mut entries: Vec<(bool, bool, Ipv4Addr)> = Vec::new();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let Some(ip_str) = parts.first() else {
                continue;
            };
            let Ok(ip) = ip_str.parse::<Ipv4Addr>() else {
                continue;
            };
            let Some(dev_idx) = parts.iter().position(|&p| p == "dev") else {
                continue;
            };
            let Some(iface) = parts.get(dev_idx + 1) else {
                continue;
            };
            if !Self::is_physical_iface(iface) || line.contains("FAILED") || line.contains("INCOMPLETE") {
                continue;
            }
            let is_hotspot = ip.octets()[0..3] == [10, 42, 0];
            let is_reachable = line.contains("REACHABLE");
            entries.push((is_hotspot, is_reachable, ip));
        }

        entries.sort_by_key(|(hotspot, reachable, ip)| (!*hotspot, !*reachable, u32::from(*ip)));
        entries.into_iter().map(|(_, _, ip)| ip).collect()
    }

    /// IPv4 subnets of physical interfaces (skips loopback, VPN, veth, Waydroid and other virtual links)
    fn physical_subnets() -> Vec<(Ipv4Addr, u8)> {
        let Ok(out) = Command::new("ip")
            .args(["-4", "-o", "addr", "show", "up", "scope", "global"])
            .output()
        else {
            return Vec::new();
        };

        let mut subnets = Vec::new();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let (Some(iface), Some(cidr)) = (parts.get(1), parts.get(3)) else {
                continue;
            };
            if !Self::is_physical_iface(iface) {
                continue;
            }
            let Some((addr, prefix)) = cidr.split_once('/') else {
                continue;
            };
            if let (Ok(addr), Ok(prefix)) = (addr.parse::<Ipv4Addr>(), prefix.parse::<u8>()) {
                // Keep sweeps bounded: /22 .. /30 (at most 1022 hosts)
                if (22..=30).contains(&prefix) {
                    subnets.push((addr, prefix));
                }
            }
        }
        subnets
    }

    async fn probe_adb_port(hosts: &[Ipv4Addr], per_host_timeout: Duration) -> Vec<Ipv4Addr> {
        let mut open: Vec<Ipv4Addr> = Vec::new();
        for chunk in hosts.chunks(256) {
            let mut set = JoinSet::new();
            for &ip in chunk {
                set.spawn(async move {
                    let probe = timeout(
                        per_host_timeout,
                        TcpStream::connect((ip, ADB_TCP_PORT)),
                    )
                    .await;
                    matches!(probe, Ok(Ok(_))).then_some(ip)
                });
            }
            while let Some(res) = set.join_next().await {
                if let Ok(Some(ip)) = res {
                    open.push(ip);
                }
            }
        }
        open
    }

    /// Parallel TCP probe of ADB port 5555 across physical subnets, hotspot & ARP neighbours first
    pub async fn scan_adb_hosts() -> Vec<Ipv4Addr> {
        let mut hosts: Vec<Ipv4Addr> = Vec::new();
        for (own, prefix) in Self::physical_subnets() {
            let mask = u32::MAX << (32 - prefix);
            let network = u32::from(own) & mask;
            let broadcast = network | !mask;
            for raw in (network + 1)..broadcast {
                let ip = Ipv4Addr::from(raw);
                if ip != own {
                    hosts.push(ip);
                }
            }
        }

        let mut open = Self::probe_adb_port(&hosts, Duration::from_millis(800)).await;
        let neigh = Self::physical_neighbours();
        open.sort_by_key(|ip| {
            let pos = neigh.iter().position(|n| n == ip).unwrap_or(usize::MAX);
            (pos, ip.octets()[0..3] != [10, 42, 0], u32::from(*ip))
        });
        open
    }

    /// Enable TCP/IP mode on USB device (`-d`) to prepare for wireless transition
    pub async fn enable_tcpip_5555() -> Result<(), String> {
        let out = tokio::process::Command::new("adb")
            .args(["-d", "tcpip", "5555"])
            .output()
            .await
            .map_err(|e| format!("Ошибка adb tcpip: {}", e))?;

        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }
}
