use std::collections::HashMap;
use std::process::{Command, Stdio};

use crate::hw;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiAp {
    pub ssid: String,
    pub signal: u32,
    pub security: String,
    pub is_active: bool,
    pub is_connecting: bool,
    pub is_saved: bool,
}

#[derive(Clone, Debug)]
pub struct SavedConn {
    pub uuid: String,
    pub _name: String,
    pub autoconnect: bool,
}

pub fn query_saved_wifi_connections() -> HashMap<String, SavedConn> {
    let mut saved = HashMap::new();
    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "UUID,TYPE,AUTOCONNECT,NAME", "connection", "show"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts = crate::hw::split_nmcli_escaped(line);
            if parts.len() >= 4 {
                let uuid = parts[0].trim().to_string();
                let ctype = parts[1].trim();
                let autoconnect = parts[2].trim().eq_ignore_ascii_case("yes");
                let name = parts[3].trim().to_string();
                if ctype == "802-11-wireless" || ctype == "wifi" {
                    let conn = SavedConn {
                        uuid,
                        _name: name.clone(),
                        autoconnect,
                    };
                    saved.insert(name, conn);
                }
            }
        }
    }
    saved
}

pub fn query_wifi_networks(rescan: bool) -> Vec<WifiAp> {
    if hw::is_hotspot_active() {
        return Vec::new();
    }
    let saved_map = query_saved_wifi_connections();
    let active_ssid = hw::get_active_wifi_ssid();
    let mut networks: Vec<WifiAp> = Vec::new();

    let rescan_arg = if rescan { "yes" } else { "no" };
    if let Ok(out) = Command::new("timeout")
        .args([
            "10",
            "nmcli",
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "dev",
            "wifi",
            "list",
            "--rescan",
            rescan_arg,
        ])
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);

        for line in text.lines() {
            let parts = crate::hw::split_nmcli_escaped(line);
            if parts.len() >= 4 {
                let _in_use = parts[0] == "*";
                let ssid = parts[1].trim().to_string();
                if ssid.is_empty() {
                    continue;
                }
                let signal: u32 = parts[2].parse().unwrap_or(50);
                let security = parts[3].clone();
                let is_saved = saved_map.contains_key(&ssid);
                let is_active = active_ssid.as_deref() == Some(&ssid);

                if let Some(existing) = networks.iter_mut().find(|n| n.ssid == ssid) {
                    existing.signal = existing.signal.max(signal);
                    existing.is_active = is_active;
                } else {
                    networks.push(WifiAp {
                        ssid,
                        signal,
                        security,
                        is_active,
                        is_connecting: false,
                        is_saved,
                    });
                }
            }
        }
    }

    if let Some(ref act) = active_ssid {
        if let Some(existing) = networks.iter_mut().find(|n| &n.ssid == act) {
            existing.is_active = true;
        } else {
            networks.push(WifiAp {
                ssid: act.clone(),
                signal: 100,
                security: String::new(),
                is_active: true,
                is_connecting: false,
                is_saved: saved_map.contains_key(act),
            });
        }
    }

    networks.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then_with(|| b.is_saved.cmp(&a.is_saved))
            .then_with(|| b.signal.cmp(&a.signal))
    });

    networks
}

pub fn parse_cmd_output(res: Result<std::process::Output, std::io::Error>) -> (bool, String) {
    match res {
        Ok(r) => {
            if r.status.success() {
                (true, String::new())
            } else {
                let err = String::from_utf8_lossy(&r.stderr).trim().to_string();
                let msg = if err.is_empty() {
                    String::from_utf8_lossy(&r.stdout).trim().to_string()
                } else {
                    err
                };
                (false, msg)
            }
        }
        Err(e) => (false, e.to_string()),
    }
}
