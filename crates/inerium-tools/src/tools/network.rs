//! Inerium Dragon — Native Rust Network Provider for Waybar
//! - Instant sysfs checking for Ethernet (eno1) and Wi-Fi (wlp4s0/wl*)
//! - Zero subprocess execution overhead (direct POSIX getifaddrs() system call)
//! - Watch mode: Streams network updates, reacts to SIGUSR1
//! - Tooltip: Rich Pango markup with interface name, IP address, state

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use inerium_i18n::tr;
use inerium_tools::format::escape_json;
use inerium_tools::ipc::{PidRegistry, SelfPipe};
use inerium_tools::sys::posix::get_interface_ipv4;

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn is_wifi_radio_enabled() -> bool {
    // 1. Check rfkill subsystem
    let mut found_wlan_rfkill = false;
    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(t) = read_trimmed(&path.join("type"))
                && t == "wlan"
            {
                found_wlan_rfkill = true;
                let soft = read_trimmed(&path.join("soft")).unwrap_or_default();
                let hard = read_trimmed(&path.join("hard")).unwrap_or_default();
                if soft == "1" || hard == "1" {
                    return false;
                }
            }
        }
    }
    if found_wlan_rfkill {
        return true;
    }

    // 2. Fallback: check wl* interface IFF_UP flag (0x1)
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("wl")
                && let Some(flags_str) = read_trimmed(&entry.path().join("flags"))
                && let Ok(flags) = u32::from_str_radix(flags_str.trim_start_matches("0x"), 16)
            {
                return (flags & 0x1) != 0;
            }
        }
    }

    false
}

enum NetState {
    Ethernet(String),
    WifiConnected(String),
    WifiSearching(String),
    WifiDisabled(String),
    Disconnected,
}

fn detect_network() -> NetState {
    // 1. Check Ethernet interfaces dynamically (eno1, enp*, enx*, eth*)
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("en") || name.starts_with("eth") {
                let carrier_path = entry.path().join("carrier");
                if let Some(c) = read_trimmed(&carrier_path)
                    && c == "1"
                {
                    return NetState::Ethernet(name);
                }
            }
        }
    }

    // 2. Check Wi-Fi interfaces
    let mut wifi_iface: Option<String> = None;
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("wl") {
                let oper_path = entry.path().join("operstate");
                if let Some(state) = read_trimmed(&oper_path)
                    && state == "up"
                {
                    return NetState::WifiConnected(name);
                }
                if wifi_iface.is_none() {
                    wifi_iface = Some(name);
                }
            }
        }
    }

    if let Some(iface) = wifi_iface {
        if is_wifi_radio_enabled() {
            return NetState::WifiSearching(iface);
        } else {
            return NetState::WifiDisabled(iface);
        }
    }

    NetState::Disconnected
}

fn format_status_for(state: &NetState) -> &'static str {
    match state {
        NetState::Ethernet(_) => "󰈀 Eth",
        NetState::WifiConnected(_) => "󰤨 WiFi",
        NetState::WifiSearching(_) => "󰤩 WiFi",
        NetState::WifiDisabled(_) | NetState::Disconnected => "󰤭",
    }
}

fn format_status() -> &'static str {
    format_status_for(&detect_network())
}

fn get_ip_address(iface: &str) -> Option<String> {
    get_interface_ipv4(iface)
}

fn format_tooltip_for(state: &NetState) -> String {
    match state {
        NetState::Ethernet(iface) => {
            let ip = get_ip_address(iface).unwrap_or_else(|| tr("Acquiring...", "Получение...").to_string());
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#87a987' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span> <span color='#e6e2cc'>{}</span>\n\
                <span color='#727169'>{}</span>   <span color='#8ba4b0'>{}</span>\n\
                <span color='#727169'>{}</span>     <span color='#87a987'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰈀 Ethernet", "󰈀 Проводная сеть"),
                tr("Interface:", "Интерфейс:"),
                iface,
                tr("IP Address:", "IP-адрес:"),
                ip,
                tr("Status:", "Статус:"),
                tr("Connected", "Подключено"),
                tr("LMB: Networks & Wi-Fi · RMB: Toggle · MMB: Connection Settings", "ЛКМ: Сети и Wi-Fi · ПКМ: Вкл/выкл · СКМ: Настройка соединений")
            )
        }
        NetState::WifiConnected(iface) => {
            let ip = get_ip_address(iface).unwrap_or_else(|| tr("Acquiring...", "Получение...").to_string());
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#7fb4ca' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span> <span color='#e6e2cc'>{}</span>\n\
                <span color='#727169'>{}</span>   <span color='#8ba4b0'>{}</span>\n\
                <span color='#727169'>{}</span>     <span color='#87a987'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰤨 Wireless Network", "󰤨 Беспроводная сеть"),
                tr("Interface:", "Интерфейс:"),
                iface,
                tr("IP Address:", "IP-адрес:"),
                ip,
                tr("Status:", "Статус:"),
                tr("Online", "В сети"),
                tr("LMB: Networks & Wi-Fi · RMB: Toggle · MMB: Connection Settings", "ЛКМ: Сети и Wi-Fi · ПКМ: Вкл/выкл · СКМ: Настройка соединений")
            )
        }
        NetState::WifiSearching(iface) => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#e6c384' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span> <span color='#e6e2cc'>{}</span>\n\
                <span color='#727169'>{}</span>     <span color='#e6c384'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰤩 Wireless Network", "󰤩 Беспроводная сеть"),
                tr("Interface:", "Интерфейс:"),
                iface,
                tr("Status:", "Статус:"),
                tr("Searching / Connecting...", "Поиск сетей / Подключение..."),
                tr("LMB: Networks & Wi-Fi · RMB: Toggle · MMB: Connection Settings", "ЛКМ: Сети и Wi-Fi · ПКМ: Вкл/выкл · СКМ: Настройка соединений")
            )
        }
        NetState::WifiDisabled(iface) => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#c4746e' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span> <span color='#e6e2cc'>{}</span>\n\
                <span color='#727169'>{}</span>     <span color='#c4746e'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰤭 Wi-Fi Disabled", "󰤭 Wi-Fi выключен"),
                tr("Interface:", "Интерфейс:"),
                iface,
                tr("Status:", "Статус:"),
                tr("Radio off", "Модуль выключен"),
                tr("LMB: Networks & Wi-Fi · RMB: Turn On · MMB: Connection Settings", "ЛКМ: Сети и Wi-Fi · ПКМ: Включить · СКМ: Настройка соединений")
            )
        }
        NetState::Disconnected => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#c4746e' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰤭 Network Disconnected", "󰤭 Сеть отключена"),
                tr("Interfaces inactive", "Интерфейсы не активны"),
                tr("LMB: Networks & Wi-Fi · MMB: Connection Settings", "ЛКМ: Сети и Wi-Fi · СКМ: Настройка соединений")
            )
        }
    }
}

fn format_tooltip() -> String {
    format_tooltip_for(&detect_network())
}

fn format_json_for(state: &NetState) -> String {
    let text = format_status_for(state);
    let tooltip = format_tooltip_for(state);
    let class = match state {
        NetState::Ethernet(_) => "ethernet",
        NetState::WifiConnected(_) => "wifi",
        NetState::WifiSearching(_) => "connecting",
        NetState::WifiDisabled(_) | NetState::Disconnected => "disconnected",
    };
    format!(
        "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":\"{}\"}}",
        escape_json(text),
        escape_json(&tooltip),
        class
    )
}

fn watch_mode(waybar: bool) {
    let reg = PidRegistry::new("network");
    let _pid_guard = reg.register();

    let self_pipe = match SelfPipe::new() {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("Failed to setup self-pipe: {e}");
            return;
        }
    };

    let mut last_output = String::new();

    loop {
        let state = detect_network();
        let timeout_ms = match &state {
            NetState::WifiSearching(_) => 500,
            _ => 3000,
        };

        let current = if waybar {
            format_json_for(&state)
        } else {
            format_status_for(&state).to_string()
        };

        if current != last_output {
            println!("{}", current);
            io::stdout().flush().ok();
            last_output = current;
        }

        if self_pipe.wait_event(timeout_ms) {
            inerium_i18n::Lang::reinit();
        }
    }
}

pub fn run(args: &[String]) {
    if args.iter().any(|a| a == "--tooltip") {
        println!("{}", format_tooltip());
        return;
    }
    if args.iter().any(|a| a == "--waybar") {
        watch_mode(true);
        return;
    }
    if args.iter().any(|a| a == "--watch") {
        let waybar = args.iter().any(|a| a == "--json" || a == "--waybar");
        watch_mode(waybar);
        return;
    }

    println!("{}", format_status());
}
