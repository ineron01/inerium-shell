use std::cell::Cell;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use gdk4::BUTTON_SECONDARY;
use gtk4::prelude::*;
use gtk4::{Align, Button, CheckButton, Label, Orientation, PolicyType};
use inerium_i18n::{tr, tr_fmt};
use relm4::prelude::*;

use crate::hw;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiAp {
    pub ssid: String,
    pub signal: u32,
    pub security: String,
    pub is_active: bool,
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
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let uuid = parts[0].trim().to_string();
                let ctype = parts[1].trim();
                let autoconnect = parts[2].trim().eq_ignore_ascii_case("yes");
                let name = parts[3..].join(":").replace("\\:", ":").trim().to_string();
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
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let _in_use = parts[0] == "*";
                let ssid = parts[1].replace("\\:", ":").trim().to_string();
                if ssid.is_empty() {
                    continue;
                }
                let signal: u32 = parts[2].parse().unwrap_or(50);
                let security = parts[3].to_string();
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

fn parse_cmd_output(res: Result<std::process::Output, std::io::Error>) -> (bool, String) {
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

#[derive(Clone, Debug, Default)]
pub struct AuthFormState {
    pub ssid: String,
    pub identity: String,
    pub password: String,
    pub no_ca_cert: bool,
    pub is_enterprise: bool,
    pub error_msg: Option<String>,
    pub is_connecting: bool,
}

pub struct NetworkModel {
    pub wifi_enabled: bool,
    pub is_scanning: bool,
    pub hotspot_active: bool,
    pub active_ssid: Option<String>,
    pub active_details: Option<hw::network::ActiveWifiDetails>,
    pub saved_connections: HashMap<String, SavedConn>,
    pub expanded_ssid: Option<String>,
    pub networks: Vec<WifiAp>,
    pub auth_state: Option<AuthFormState>,
    pub needs_list_rebuild: Cell<bool>,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

impl NetworkModel {
    fn sync_active_network(&mut self) {
        self.saved_connections = query_saved_wifi_connections();
        self.active_details = hw::get_active_wifi_details();

        if let Some(ref act) = self.active_ssid {
            for net in &mut self.networks {
                net.is_active = &net.ssid == act;
                net.is_saved = self.saved_connections.contains_key(&net.ssid);
            }
            if !self.networks.iter().any(|n| &n.ssid == act) {
                self.networks.push(WifiAp {
                    ssid: act.clone(),
                    signal: 100,
                    security: String::new(),
                    is_active: true,
                    is_saved: self.saved_connections.contains_key(act),
                });
            }
        } else {
            for net in &mut self.networks {
                net.is_active = false;
                net.is_saved = self.saved_connections.contains_key(&net.ssid);
            }
        }

        self.networks.sort_by(|a, b| {
            b.is_active
                .cmp(&a.is_active)
                .then_with(|| b.is_saved.cmp(&a.is_saved))
                .then_with(|| b.signal.cmp(&a.signal))
        });
        self.needs_list_rebuild.set(true);
    }
}

#[derive(Debug)]
pub enum NetworkMsg {
    ToggleWifi(bool),
    StartRescan,
    ScanProgress(Vec<WifiAp>),
    RescanFinished(Vec<WifiAp>),
    PollStatus,
    ToggleExpandAp(String),
    ToggleAutoconnect(String, bool),
    SavedConnectionsUpdated(HashMap<String, SavedConn>),
    ForgetSavedNetwork(String),
    SelectAp(WifiAp),
    IdentityInput(String),
    PasswordInput(String),
    ToggleNoCaCert,
    SubmitAuth(String),
    ConnectFinished {
        ssid: String,
        success: bool,
        error: Option<String>,
        networks: Vec<WifiAp>,
    },
    DisconnectFinished(Vec<WifiAp>),
    StopHotspot,
    Back,
}

pub struct NetworkInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for NetworkModel {
    type Input = NetworkMsg;
    type Output = ();
    type Init = NetworkInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (390, 420),

            // Header (Back, Title, Power switch)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,
                set_valign: Align::Center,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: tr("‹ Back", "‹ Назад"),
                    set_valign: Align::Center,
                    connect_clicked => NetworkMsg::Back,
                },

                gtk4::Label {
                    add_css_class: "cc-host",
                    set_label: tr("Wi-Fi & Networks", "Wi-Fi и сети"),
                    set_hexpand: true,
                    set_xalign: 0.0,
                    set_valign: Align::Center,
                },

                #[name = "pwr_switch"]
                gtk4::Switch {
                    set_active: model.wifi_enabled,
                    set_valign: Align::Center,
                    connect_state_set[sender] => move |_, state| {
                        sender.input(NetworkMsg::ToggleWifi(state));
                        glib::Propagation::Proceed
                    },
                },
            },

            // Subheader (AVAILABLE NETWORKS / HOTSPOT, Rescan button)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                #[name = "subhead_lbl"]
                gtk4::Label {
                    add_css_class: "cc-label",
                    set_xalign: 0.0,
                    set_hexpand: true,
                    #[watch]
                    set_label: if model.hotspot_active {
                        tr("HOTSPOT", "ТОЧКА ДОСТУПА")
                    } else {
                        tr("AVAILABLE NETWORKS", "ДОСТУПНЫЕ СЕТИ")
                    },
                },

                #[name = "rescan_btn"]
                gtk4::Button {
                    add_css_class: "cc-chip",
                    #[watch]
                    set_visible: !model.hotspot_active,
                    #[watch]
                    set_sensitive: model.wifi_enabled && !model.is_scanning,
                    #[watch]
                    set_label: if model.is_scanning {
                        tr("󰑐  Searching...", "󰑐  Поиск сетей...")
                    } else {
                        tr("󰑐  Refresh", "󰑐  Обновить")
                    },
                    connect_clicked => NetworkMsg::StartRescan,
                },
            },

            // Scrolled Network List
            #[name = "scroll_win"]
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (PolicyType::Never, PolicyType::Automatic),
                set_propagate_natural_height: false,
                set_min_content_height: 300,
                set_max_content_height: 300,
                set_vexpand: true,

                #[name = "list_box"]
                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 4,
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let wifi_enabled = hw::is_wifi_enabled();
        let hotspot_active = hw::is_hotspot_active();
        let active_ssid = hw::get_active_wifi_ssid();
        let active_details = hw::get_active_wifi_details();
        let saved_connections = query_saved_wifi_connections();
        let mut networks = if wifi_enabled && !hotspot_active {
            query_wifi_networks(false)
        } else {
            Vec::new()
        };

        if let Some(ref act) = active_ssid {
            for net in &mut networks {
                net.is_active = &net.ssid == act;
            }
            networks.sort_by(|a, b| {
                b.is_active
                    .cmp(&a.is_active)
                    .then_with(|| b.is_saved.cmp(&a.is_saved))
                    .then_with(|| b.signal.cmp(&a.signal))
            });
        }

        let model = NetworkModel {
            wifi_enabled,
            is_scanning: false,
            hotspot_active,
            active_ssid,
            active_details,
            saved_connections,
            expanded_ssid: None,
            networks,
            auth_state: None,
            needs_list_rebuild: Cell::new(false),
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        let _ = populate_wifi_list(
            &widgets.list_box,
            &model.networks,
            &model.saved_connections,
            model.active_details.as_ref(),
            model.expanded_ssid.as_deref(),
            model.auth_state.as_ref(),
            model.hotspot_active,
            model.wifi_enabled,
            model.is_scanning,
            &sender,
        );

        let root_ref = root.clone();
        let s_poll = sender.clone();
        glib::timeout_add_local(Duration::from_millis(1000), move || {
            if !root_ref.is_mapped() {
                return glib::ControlFlow::Break;
            }
            s_poll.input(NetworkMsg::PollStatus);
            glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn post_view() {
        if self.needs_list_rebuild.get() {
            self.needs_list_rebuild.set(false);
            let vadj = scroll_win.vadjustment();
            let prev_val = vadj.value();

            let exp_widget = populate_wifi_list(
                list_box,
                &self.networks,
                &self.saved_connections,
                self.active_details.as_ref(),
                self.expanded_ssid.as_deref(),
                self.auth_state.as_ref(),
                self.hotspot_active,
                self.wifi_enabled,
                self.is_scanning,
                &sender,
            );

            let vadj_clone = vadj.clone();
            let scroll_win_clone = scroll_win.clone();
            glib::idle_add_local(move || {
                let max_val = (vadj_clone.upper() - vadj_clone.page_size()).max(0.0);
                if let Some(rect) = exp_widget.as_ref().and_then(|c| c.compute_bounds(&scroll_win_clone)) {
                    let bottom = rect.y() + rect.height();
                    let viewport_h = vadj_clone.page_size();
                    if (bottom as f64) > viewport_h {
                        let diff = (bottom as f64) - viewport_h + 12.0;
                        let new_val = (prev_val + diff).min(max_val);
                        vadj_clone.set_value(new_val);
                        return glib::ControlFlow::Break;
                    }
                }
                vadj_clone.set_value(prev_val.min(max_val));
                glib::ControlFlow::Break
            });
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            NetworkMsg::ToggleWifi(state) => {
                self.wifi_enabled = state;
                if !state {
                    self.is_scanning = false;
                    self.networks.clear();
                    self.needs_list_rebuild.set(true);
                    hw::toggle_wifi(false);
                } else {
                    self.is_scanning = true;
                    self.networks.clear();
                    self.needs_list_rebuild.set(true);
                    let s = sender.clone();
                    thread::spawn(move || {
                        let start = Instant::now();
                        hw::set_wifi_enabled_sync(true);

                        let ready = hw::wait_for_wifi_ready(Duration::from_secs(4));
                        if ready {
                            let cached = query_wifi_networks(false);
                            if !cached.is_empty() {
                                s.input(NetworkMsg::ScanProgress(cached));
                            }

                            let fresh = query_wifi_networks(true);
                            let min_scan_time = Duration::from_millis(1500);
                            let elapsed = start.elapsed();
                            if elapsed < min_scan_time {
                                thread::sleep(min_scan_time - elapsed);
                            }
                            s.input(NetworkMsg::RescanFinished(fresh));
                        } else {
                            s.input(NetworkMsg::RescanFinished(Vec::new()));
                        }
                    });
                }
            }
            NetworkMsg::StartRescan => {
                if !self.wifi_enabled {
                    return;
                }
                self.is_scanning = true;
                self.needs_list_rebuild.set(true);
                let s = sender.clone();
                thread::spawn(move || {
                    let start = Instant::now();
                    let _ = hw::wait_for_wifi_ready(Duration::from_secs(2));
                    let fresh = query_wifi_networks(true);
                    let min_scan_time = Duration::from_millis(1500);
                    let elapsed = start.elapsed();
                    if elapsed < min_scan_time {
                        thread::sleep(min_scan_time - elapsed);
                    }
                    s.input(NetworkMsg::RescanFinished(fresh));
                });
            }
            NetworkMsg::PollStatus => {
                let current_ssid = hw::get_active_wifi_ssid();
                let current_hotspot = hw::is_hotspot_active();
                let current_details = hw::get_active_wifi_details();
                let ssid_changed = current_ssid != self.active_ssid;
                let hotspot_changed = current_hotspot != self.hotspot_active;
                let ip_changed = current_details.as_ref().map(|d| &d.ip) != self.active_details.as_ref().map(|d| &d.ip);

                if ssid_changed || hotspot_changed || ip_changed {
                    self.active_ssid = current_ssid;
                    self.hotspot_active = current_hotspot;
                    self.active_details = current_details;
                    self.sync_active_network();
                }
            }
            NetworkMsg::ToggleExpandAp(ssid) => {
                if self.expanded_ssid.as_ref() == Some(&ssid) {
                    self.expanded_ssid = None;
                    self.auth_state = None;
                } else {
                    self.expanded_ssid = Some(ssid.clone());
                    if let Some(net) = self.networks.iter().find(|n| n.ssid == ssid) {
                        if !net.is_saved && !net.security.is_empty() && !net.security.contains("--") {
                            let is_ent = net.security.contains("802.1X") || net.security.contains("EAP");
                            self.auth_state = Some(AuthFormState {
                                ssid: ssid.clone(),
                                identity: String::new(),
                                password: String::new(),
                                no_ca_cert: true,
                                is_enterprise: is_ent,
                                error_msg: None,
                                is_connecting: false,
                            });
                        } else {
                            self.auth_state = None;
                        }
                    } else {
                        self.auth_state = None;
                    }
                }
                self.needs_list_rebuild.set(true);
            }
            NetworkMsg::ToggleAutoconnect(uuid, enable) => {
                let s = sender.clone();
                thread::spawn(move || {
                    hw::set_connection_autoconnect(&uuid, enable);
                    let fresh_saved = query_saved_wifi_connections();
                    s.input(NetworkMsg::SavedConnectionsUpdated(fresh_saved));
                });
            }
            NetworkMsg::SavedConnectionsUpdated(saved) => {
                self.saved_connections = saved;
                for net in &mut self.networks {
                    net.is_saved = self.saved_connections.contains_key(&net.ssid);
                }
                self.needs_list_rebuild.set(true);
            }
            NetworkMsg::ForgetSavedNetwork(uuid) => {
                self.expanded_ssid = None;
                self.auth_state = None;
                let s = sender.clone();
                thread::spawn(move || {
                    hw::delete_connection(&uuid);
                    let fresh = query_wifi_networks(false);
                    s.input(NetworkMsg::RescanFinished(fresh));
                });
            }
            NetworkMsg::ScanProgress(fresh) => {
                self.networks = fresh;
                self.active_ssid = hw::get_active_wifi_ssid();
                self.hotspot_active = hw::is_hotspot_active();
                self.sync_active_network();
            }
            NetworkMsg::RescanFinished(fresh) => {
                self.is_scanning = false;
                self.networks = fresh;
                self.active_ssid = hw::get_active_wifi_ssid();
                self.hotspot_active = hw::is_hotspot_active();
                self.sync_active_network();
            }
            NetworkMsg::SelectAp(net) => {
                if net.is_active {
                    let s_fin = sender.clone();
                    let ssid = net.ssid.clone();
                    thread::spawn(move || {
                        let saved_map = query_saved_wifi_connections();
                        if let Some(conn) = saved_map.get(&ssid) {
                            let _ = Command::new("nmcli")
                                .args(["connection", "down", "uuid", &conn.uuid])
                                .output();
                        } else {
                            let _ = Command::new("nmcli")
                                .args(["connection", "down", "id", &ssid])
                                .output();
                        }
                        let fresh = query_wifi_networks(false);
                        s_fin.input(NetworkMsg::DisconnectFinished(fresh));
                    });
                } else if net.is_saved {
                    let s_fin = sender.clone();
                    let ssid = net.ssid.clone();
                    thread::spawn(move || {
                        let saved_map = query_saved_wifi_connections();
                        let res = if let Some(conn) = saved_map.get(&ssid) {
                            Command::new("nmcli")
                                .args(["--wait", "15", "connection", "up", "uuid", &conn.uuid])
                                .stdin(std::process::Stdio::null())
                                .output()
                        } else {
                            Command::new("nmcli")
                                .args(["--wait", "15", "connection", "up", "id", &ssid])
                                .stdin(std::process::Stdio::null())
                                .output()
                        };
                        let (success, err_msg) = parse_cmd_output(res);
                        let fresh = query_wifi_networks(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                        });
                    });
                } else if !net.security.is_empty() && !net.security.contains("--") {
                    let ssid = net.ssid.clone();
                    sender.input(NetworkMsg::ToggleExpandAp(ssid));
                } else {
                    let s_fin = sender.clone();
                    let ssid = net.ssid.clone();
                    thread::spawn(move || {
                        let res = Command::new("nmcli")
                            .args(["--wait", "15", "dev", "wifi", "connect", &ssid])
                            .stdin(std::process::Stdio::null())
                            .output();
                        let (success, err_msg) = parse_cmd_output(res);
                        let fresh = query_wifi_networks(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                        });
                    });
                }
            }
            NetworkMsg::IdentityInput(text) => {
                if let Some(ref mut auth) = self.auth_state {
                    auth.identity = text;
                    auth.error_msg = None;
                }
            }
            NetworkMsg::PasswordInput(text) => {
                if let Some(ref mut auth) = self.auth_state {
                    auth.password = text;
                    auth.error_msg = None;
                }
            }
            NetworkMsg::ToggleNoCaCert => {
                if let Some(ref mut auth) = self.auth_state {
                    auth.no_ca_cert = !auth.no_ca_cert;
                }
                self.needs_list_rebuild.set(true);
            }
            NetworkMsg::SubmitAuth(ssid) => {
                if let Some(ref mut auth) = self.auth_state {
                    if auth.ssid != ssid || auth.password.trim().is_empty() {
                        return;
                    }
                    if !auth.is_enterprise && auth.password.len() < 8 {
                        return;
                    }
                    if auth.is_enterprise && auth.identity.trim().is_empty() {
                        return;
                    }
                    auth.is_connecting = true;
                    auth.error_msg = None;
                    self.needs_list_rebuild.set(true);

                    let is_ent = auth.is_enterprise;
                    let identity = auth.identity.clone();
                    let pass = auth.password.clone();
                    let no_ca = auth.no_ca_cert;
                    let s_fin = sender.clone();
                    let ssid_task = ssid.clone();

                    thread::spawn(move || {
                        let (success, err_msg) = if is_ent {
                            match hw::connect_enterprise_wifi(&ssid_task, &identity, &pass, no_ca) {
                                Ok(()) => (true, String::new()),
                                Err(e) => (false, e),
                            }
                        } else {
                            let res = Command::new("nmcli")
                                .args(["--wait", "15", "dev", "wifi", "connect", &ssid_task, "password", &pass])
                                .stdin(std::process::Stdio::null())
                                .output();
                            parse_cmd_output(res)
                        };
                        let fresh = query_wifi_networks(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid: ssid_task,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                        });
                    });
                }
            }
            NetworkMsg::ConnectFinished {
                ssid,
                success,
                error,
                networks,
            } => {
                self.networks = networks;
                self.active_ssid = hw::get_active_wifi_ssid();
                self.sync_active_network();
                if success {
                    self.expanded_ssid = None;
                    self.auth_state = None;
                } else {
                    let err_text = error.unwrap_or_else(|| {
                        tr("Connection error", "Ошибка подключения").to_string()
                    });
                    let clean = err_text
                        .trim_start_matches("Error: ")
                        .trim_start_matches("Ошибка: ");
                    self.expanded_ssid = Some(ssid.clone());
                    if let Some(ref mut auth) = self.auth_state {
                        auth.is_connecting = false;
                        auth.error_msg = Some(format!("󰅖 {}", clean));
                    } else {
                        let is_ent = self
                            .networks
                            .iter()
                            .find(|n| n.ssid == ssid)
                            .map(|n| n.security.contains("802.1X") || n.security.contains("EAP"))
                            .unwrap_or(false);
                        self.auth_state = Some(AuthFormState {
                            ssid,
                            identity: String::new(),
                            password: String::new(),
                            no_ca_cert: true,
                            is_enterprise: is_ent,
                            error_msg: Some(format!("󰅖 {}", clean)),
                            is_connecting: false,
                        });
                    }
                }
                self.needs_list_rebuild.set(true);
            }
            NetworkMsg::DisconnectFinished(fresh) => {
                self.networks = fresh;
                self.active_ssid = hw::get_active_wifi_ssid();
                self.sync_active_network();
            }
            NetworkMsg::StopHotspot => {
                let s_fin = sender.clone();
                thread::spawn(move || {
                    let bin = hw::home_dir().join(".local/bin/inerium-hotspot");
                    let mut cmd = if bin.exists() {
                        Command::new(bin)
                    } else {
                        Command::new("inerium-hotspot")
                    };
                    let _ = cmd.arg("toggle").output();
                    thread::sleep(Duration::from_millis(800));
                    let fresh = query_wifi_networks(false);
                    s_fin.input(NetworkMsg::RescanFinished(fresh));
                });
            }
            NetworkMsg::Back => {
                crate::set_modal_dialog_active(false);
                (self.nav_cb)("control");
            }
        }
    }
}

impl Drop for NetworkModel {
    fn drop(&mut self) {
        crate::set_modal_dialog_active(false);
    }
}

#[allow(clippy::too_many_arguments)]
fn populate_wifi_list(
    list_box: &gtk4::Box,
    networks: &[WifiAp],
    saved_conns: &HashMap<String, SavedConn>,
    active_details: Option<&hw::network::ActiveWifiDetails>,
    expanded_ssid: Option<&str>,
    auth_state: Option<&AuthFormState>,
    hotspot_active: bool,
    wifi_enabled: bool,
    is_scanning: bool,
    sender: &ComponentSender<NetworkModel>,
) -> Option<gtk4::Box> {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    if hotspot_active {
        let card = gtk4::Box::new(Orientation::Vertical, 10);
        card.add_css_class("cc-card");
        card.set_margin_top(8);
        card.set_margin_bottom(8);

        let top_row = gtk4::Box::new(Orientation::Horizontal, 8);
        let ico = Label::new(Some("󱜠"));
        ico.add_css_class("card-icon");
        top_row.append(&ico);

        let title_box = gtk4::Box::new(Orientation::Vertical, 2);
        title_box.set_hexpand(true);
        let title = Label::new(Some(tr("Hotspot Active", "Занято точкой доступа")));
        title.add_css_class("card-title");
        title.set_xalign(0.0);
        title_box.append(&title);

        let sub = Label::new(Some(tr(
            "Wi-Fi Hotspot is active",
            "Раздача Wi-Fi (точка доступа)",
        )));
        sub.add_css_class("card-sub");
        sub.set_xalign(0.0);
        title_box.append(&sub);
        top_row.append(&title_box);
        card.append(&top_row);

        let desc = Label::new(Some(tr(
            "The wireless adapter is currently broadcasting Wi-Fi (AP mode).\n\nTo scan and connect to other Wi-Fi networks, turn off the hotspot first.",
            "Беспроводной модуль сейчас работает в режиме раздачи интернета (AP).\n\nДля сканирования и подключения к другим Wi-Fi сетям сначала выключите точку доступа.",
        )));
        desc.add_css_class("cc-meta");
        desc.set_wrap(true);
        desc.set_xalign(0.0);
        card.append(&desc);

        let stop_btn = Button::with_label(tr("󱜡  Stop Hotspot", "󱜡  Выключить точку доступа"));
        stop_btn.add_css_class("cc-chip");
        stop_btn.add_css_class("on");
        let s = sender.clone();
        stop_btn.connect_clicked(move |b| {
            b.set_sensitive(false);
            b.set_label(tr("󰑐 Stopping...", "󰑐 Выключение..."));
            s.input(NetworkMsg::StopHotspot);
        });
        card.append(&stop_btn);

        list_box.append(&card);
        return None;
    }

    if networks.is_empty() {
        let empty_lbl = Label::new(Some(if is_scanning {
            tr(
                "󰑐 Searching for Wi-Fi networks...",
                "󰑐 Поиск беспроводных сетей...",
            )
        } else if wifi_enabled {
            tr(
                "No networks found. Click «Refresh»",
                "Сети не найдены. Нажмите «Обновить»",
            )
        } else {
            tr("Wi-Fi is turned off", "Wi-Fi выключен")
        }));
        empty_lbl.add_css_class("cc-meta");
        empty_lbl.set_margin_top(20);
        list_box.append(&empty_lbl);
        return None;
    }

    let mut expanded_card = None;

    for net in networks {
        let card = gtk4::Box::new(Orientation::Vertical, 0);
        card.add_css_class("card-item");
        if net.is_active {
            card.add_css_class("active");
        }
        let is_exp = expanded_ssid == Some(net.ssid.as_str());
        if is_exp {
            card.add_css_class("expanded");
            expanded_card = Some(card.clone());
        }

        // Header row
        let header_row = gtk4::Box::new(Orientation::Horizontal, 8);

        // Clickable main content (icon + text + badge)
        let main_box = gtk4::Box::new(Orientation::Horizontal, 8);
        main_box.set_hexpand(true);
        main_box.set_cursor_from_name(Some("pointer"));

        let sig_ico = if net.signal > 75 {
            "󰤨"
        } else if net.signal > 50 {
            "󰤥"
        } else if net.signal > 25 {
            "󰤢"
        } else {
            "󰤟"
        };
        let ico = Label::new(Some(sig_ico));
        ico.add_css_class("card-icon");
        if net.is_active {
            ico.add_css_class("active");
        } else if net.signal > 75 {
            ico.add_css_class("signal-high");
        } else if net.signal > 35 {
            ico.add_css_class("signal-mid");
        } else {
            ico.add_css_class("signal-low");
        }
        main_box.append(&ico);

        let txt = gtk4::Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&net.ssid));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);

        let sec_str = if net.is_active {
            if let Some(details) = active_details {
                if !details.ip.is_empty() && !details.band.is_empty() {
                    tr_fmt!(
                        "Connected • {ip} ({band})",
                        "Подключено • {ip} ({band})",
                        ip = details.ip,
                        band = details.band
                    )
                } else if !details.band.is_empty() {
                    tr_fmt!(
                        "Connected • {band}",
                        "Подключено • {band}",
                        band = details.band
                    )
                } else if !details.ip.is_empty() {
                    tr_fmt!(
                        "Connected • {ip}",
                        "Подключено • {ip}",
                        ip = details.ip
                    )
                } else {
                    tr("Connected", "Подключено").to_string()
                }
            } else {
                tr("Connected", "Подключено").to_string()
            }
        } else if net.security.is_empty() {
            tr("Open network", "Открытая сеть").to_string()
        } else {
            net.security.clone()
        };
        let sec_lbl = Label::new(Some(&sec_str));
        sec_lbl.add_css_class("card-sub");
        if !net.is_active && net.security.is_empty() {
            sec_lbl.add_css_class("warning");
        }
        sec_lbl.set_xalign(0.0);
        sec_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&sec_lbl);
        main_box.append(&txt);

        // Badge: Active ("󰄬 В сети") or Saved ("Сохранено")
        let badge = Label::new(None);
        badge.add_css_class("card-badge");
        if net.is_active {
            badge.set_label(tr("󰄬  Connected", "󰄬  В сети"));
            badge.add_css_class("active");
            badge.set_visible(true);
        } else if net.is_saved {
            badge.set_label(tr("Saved", "Сохранено"));
            badge.add_css_class("saved");
            badge.set_visible(true);
        } else {
            badge.set_visible(false);
        }
        main_box.append(&badge);

        header_row.append(&main_box);

        // Chevron expand button on the right inside the card
        let chevron_btn = Button::new();
        chevron_btn.add_css_class("card-chevron");
        let chev_lbl = Label::new(Some(if is_exp { "󰅃" } else { "󰅀" }));
        chevron_btn.set_child(Some(&chev_lbl));
        chevron_btn.set_cursor_from_name(Some("pointer"));
        chevron_btn.set_tooltip_text(Some(tr("Quick Actions", "Быстрые действия")));

        let s_chev = sender.clone();
        let ssid_chev = net.ssid.clone();
        chevron_btn.connect_clicked(move |_| {
            s_chev.input(NetworkMsg::ToggleExpandAp(ssid_chev.clone()));
        });
        header_row.append(&chevron_btn);

        card.append(&header_row);

        // Tooltip on the entire card
        let tooltip_text = if net.is_active {
            let sec_desc = if net.security.is_empty() {
                tr("Open", "Открытая").to_string()
            } else {
                net.security.clone()
            };
            let mut tip = format!(
                "SSID: {}\n{}: {}",
                net.ssid,
                tr("Security", "Безопасность"),
                sec_desc
            );
            if let Some(d) = active_details {
                if !d.ip.is_empty() {
                    tip.push_str(&format!("\nIP: {}", d.ip));
                }
                if !d.gateway.is_empty() {
                    tip.push_str(&format!("\n{}: {}", tr("Gateway", "Шлюз"), d.gateway));
                }
                if d.freq_mhz > 0 {
                    tip.push_str(&format!(
                        "\n{}: {} MHz ({})",
                        tr("Frequency", "Частота"),
                        d.freq_mhz,
                        d.band
                    ));
                }
                if d.signal_dbm != 0 {
                    tip.push_str(&format!(
                        "\n{}: {} dBm ({}%)",
                        tr("Signal", "Сигнал"),
                        d.signal_dbm,
                        net.signal
                    ));
                }
            }
            tip
        } else {
            let sec_desc = if net.security.is_empty() {
                tr("Open", "Открытая").to_string()
            } else {
                net.security.clone()
            };
            format!(
                "SSID: {}\n{}: {}\n{}: {}%",
                net.ssid,
                tr("Security", "Безопасность"),
                sec_desc,
                tr("Signal", "Сигнал"),
                net.signal
            )
        };
        card.set_tooltip_text(Some(&tooltip_text));

        // Primary click on main_box -> connect / disconnect / password prompt
        let s_click = sender.clone();
        let net_item = net.clone();
        let b_ui = badge.clone();
        let main_gesture = gtk4::GestureClick::new();
        main_gesture.set_button(gdk4::BUTTON_PRIMARY);
        main_gesture.connect_released(move |_, _, _, _| {
            if net_item.is_active {
                b_ui.set_label(tr("󰑐 Disconnecting...", "󰑐 Отключение..."));
                b_ui.set_visible(true);
            } else if net_item.is_saved
                || net_item.security.is_empty()
                || net_item.security.contains("--")
            {
                b_ui.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
                b_ui.set_visible(true);
            }
            s_click.input(NetworkMsg::SelectAp(net_item.clone()));
        });
        main_box.add_controller(main_gesture);

        // Right-click gesture (ПКМ) on the entire card -> toggle expand
        let s_right = sender.clone();
        let ssid_right = net.ssid.clone();
        let right_gesture = gtk4::GestureClick::new();
        right_gesture.set_button(BUTTON_SECONDARY);
        right_gesture.connect_pressed(move |_, _, _, _| {
            s_right.input(NetworkMsg::ToggleExpandAp(ssid_right.clone()));
        });
        card.add_controller(right_gesture);

        // Integrated Accordion Drawer INSIDE the card (for ALL networks!)
        if is_exp {
            let sep = gtk4::Separator::new(Orientation::Horizontal);
            sep.add_css_class("card-drawer-sep");
            card.append(&sep);

            if let Some(conn) = saved_conns.get(&net.ssid) {
                let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
                drawer.add_css_class("card-drawer-inner");
                drawer.set_hexpand(true);
                drawer.set_homogeneous(true);

                // 1. Autoconnect chip
                let auto_btn = Button::new();
                auto_btn.add_css_class("drawer-chip");
                auto_btn.set_hexpand(true);
                if conn.autoconnect {
                    auto_btn.add_css_class("on");
                    auto_btn.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                } else {
                    auto_btn.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                }
                auto_btn.set_tooltip_text(Some(tr(
                    "Toggle automatic connection",
                    "Автоматически подключаться к сети",
                )));
                let s_auto = sender.clone();
                let u_auto = conn.uuid.clone();
                let cur_auto = conn.autoconnect;
                let b_auto = auto_btn.clone();
                auto_btn.connect_clicked(move |_| {
                    b_auto.set_sensitive(false);
                    if cur_auto {
                        b_auto.remove_css_class("on");
                        b_auto.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                    } else {
                        b_auto.add_css_class("on");
                        b_auto.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                    }
                    s_auto.input(NetworkMsg::ToggleAutoconnect(u_auto.clone(), !cur_auto));
                });
                drawer.append(&auto_btn);

                // 2. Forget network chip
                let forget_btn = Button::with_label(tr("󰆴  Forget", "󰆴  Забыть"));
                forget_btn.add_css_class("drawer-chip");
                forget_btn.add_css_class("danger");
                forget_btn.set_hexpand(true);
                forget_btn.set_tooltip_text(Some(tr(
                    "Delete saved Wi-Fi profile and credentials",
                    "Удалить сохранённый профиль и пароль сети",
                )));
                let s_forget = sender.clone();
                let u_forget = conn.uuid.clone();
                let b_forget = forget_btn.clone();
                forget_btn.connect_clicked(move |_| {
                    b_forget.set_sensitive(false);
                    b_forget.set_label(tr("󰑐  Forgetting...", "󰑐  Удаление..."));
                    s_forget.input(NetworkMsg::ForgetSavedNetwork(u_forget.clone()));
                });
                drawer.append(&forget_btn);

                // 3. Connect / Disconnect chip
                if net.is_active {
                    let disc_btn = Button::with_label(tr("󱚼  Disconnect", "󱚼  Отключиться"));
                    disc_btn.add_css_class("drawer-chip");
                    disc_btn.set_hexpand(true);
                    let s_disc = sender.clone();
                    let net_disc = net.clone();
                    let b_disc = disc_btn.clone();
                    disc_btn.connect_clicked(move |_| {
                        b_disc.set_sensitive(false);
                        b_disc.set_label(tr("󰑐  Disconnecting...", "󰑐  Отключение..."));
                        s_disc.input(NetworkMsg::SelectAp(net_disc.clone()));
                    });
                    drawer.append(&disc_btn);
                } else {
                    let conn_btn = Button::with_label(tr("󰄬  Connect", "󰄬  Подключить"));
                    conn_btn.add_css_class("drawer-chip");
                    conn_btn.set_hexpand(true);
                    let s_c = sender.clone();
                    let net_c = net.clone();
                    let b_conn = conn_btn.clone();
                    conn_btn.connect_clicked(move |_| {
                        b_conn.set_sensitive(false);
                        b_conn.set_label(tr("󰑐  Connecting...", "󰑐  Подключение..."));
                        s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                    });
                    drawer.append(&conn_btn);
                }
                card.append(&drawer);
            } else if net.security.is_empty() || net.security.contains("--") {
                // Open network actions
                let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
                drawer.add_css_class("card-drawer-inner");
                drawer.set_hexpand(true);
                drawer.set_homogeneous(true);

                let conn_btn = Button::with_label(tr("󰄬  Connect", "󰄬  Подключить"));
                conn_btn.add_css_class("drawer-chip");
                conn_btn.set_hexpand(true);
                let s_c = sender.clone();
                let net_c = net.clone();
                let b_conn = conn_btn.clone();
                conn_btn.connect_clicked(move |_| {
                    b_conn.set_sensitive(false);
                    b_conn.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
                    s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                });
                drawer.append(&conn_btn);

                let info_label = format!("󰋼 {} • {}%", tr("Open", "Открытая"), net.signal);
                let info_chip = Label::new(Some(&info_label));
                info_chip.add_css_class("drawer-chip");
                info_chip.add_css_class("info");
                info_chip.set_hexpand(true);
                drawer.append(&info_chip);

                card.append(&drawer);
            } else {
                // In-Place Authentication Drawer (WPA2/WPA3-Personal or 802.1X Enterprise)
                let drawer = gtk4::Box::new(Orientation::Vertical, 6);
                drawer.add_css_class("card-drawer-inner");
                drawer.set_hexpand(true);

                let is_ent = net.security.contains("802.1X") || net.security.contains("EAP");
                let cur_identity = auth_state.map(|a| a.identity.as_str()).unwrap_or("");
                let cur_pass = auth_state.map(|a| a.password.as_str()).unwrap_or("");
                let cur_no_ca = auth_state.map(|a| a.no_ca_cert).unwrap_or(true);
                let cur_err = auth_state.and_then(|a| a.error_msg.as_deref());
                let is_connecting = auth_state.map(|a| a.is_connecting).unwrap_or(false);

                // Construct Connect Button first so text entry listeners can update its sensitivity
                let is_valid_init = if is_ent {
                    !cur_identity.trim().is_empty() && !cur_pass.trim().is_empty()
                } else {
                    cur_pass.trim().len() >= 8
                };
                let conn_btn = Button::new();
                conn_btn.add_css_class("drawer-chip");
                conn_btn.add_css_class("on");
                conn_btn.set_hexpand(true);
                if is_connecting {
                    conn_btn.set_sensitive(false);
                    conn_btn.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
                } else {
                    conn_btn.set_sensitive(is_valid_init);
                    conn_btn.set_label(tr("󰄬 Connect", "󰄬 Подключить"));
                }
                let s_sub = sender.clone();
                let ssid_sub = net.ssid.clone();
                conn_btn.connect_clicked(move |_| {
                    s_sub.input(NetworkMsg::SubmitAuth(ssid_sub.clone()));
                });

                // 1. Identity field (Enterprise only)
                let id_entry_opt = if is_ent {
                    let id_entry = gtk4::Entry::new();
                    id_entry.add_css_class("drawer-entry");
                    id_entry.set_placeholder_text(Some(tr("Username / Student ID", "Логин / ID студента")));
                    id_entry.set_text(cur_identity);
                    drawer.append(&id_entry);
                    Some(id_entry)
                } else {
                    None
                };

                // 2. Password field (Personal & Enterprise)
                let pass_entry = gtk4::PasswordEntry::new();
                pass_entry.add_css_class("drawer-entry");
                pass_entry.set_placeholder_text(Some(tr("Password", "Пароль сети")));
                pass_entry.set_show_peek_icon(true);
                pass_entry.set_text(cur_pass);
                drawer.append(&pass_entry);

                // Auto-focus input field and scroll it into view smoothly
                let focus_target: gtk4::Widget = if let Some(ref id) = id_entry_opt {
                    id.clone().upcast()
                } else {
                    pass_entry.clone().upcast()
                };
                glib::idle_add_local(move || {
                    focus_target.grab_focus();
                    glib::ControlFlow::Break
                });

                // Real-time sensitivity updater on keystroke
                let b_conn = conn_btn.clone();
                let p_entry_clone = pass_entry.clone();
                let id_entry_clone = id_entry_opt.clone();
                let check_valid = move || {
                    let p = p_entry_clone.text().to_string();
                    if is_ent {
                        let id_ok = id_entry_clone.as_ref().map(|id| !id.text().trim().is_empty()).unwrap_or(false);
                        id_ok && !p.trim().is_empty()
                    } else {
                        p.trim().len() >= 8
                    }
                };

                let chk_p = check_valid.clone();
                let b_p = b_conn.clone();
                let s_pass = sender.clone();
                pass_entry.connect_changed(move |e| {
                    b_p.set_sensitive(chk_p());
                    s_pass.input(NetworkMsg::PasswordInput(e.text().to_string()));
                });

                let s_act_pass = sender.clone();
                let ssid_act_pass = net.ssid.clone();
                pass_entry.connect_activate(move |_| {
                    s_act_pass.input(NetworkMsg::SubmitAuth(ssid_act_pass.clone()));
                });

                if let Some(ref id_entry) = id_entry_opt {
                    let chk_id = check_valid;
                    let b_id = b_conn;
                    let s_id = sender.clone();
                    id_entry.connect_changed(move |e| {
                        b_id.set_sensitive(chk_id());
                        s_id.input(NetworkMsg::IdentityInput(e.text().to_string()));
                    });
                    let s_act_id = sender.clone();
                    let ssid_act_id = net.ssid.clone();
                    id_entry.connect_activate(move |_| {
                        s_act_id.input(NetworkMsg::SubmitAuth(ssid_act_id.clone()));
                    });
                }

                // 3. CA bypass toggle (Enterprise only)
                if is_ent {
                    let ca_check = CheckButton::with_label(tr(
                        "Don't validate CA certificate",
                        "Не проверять сертификат CA",
                    ));
                    ca_check.add_css_class("auth-ca-check");
                    ca_check.set_active(cur_no_ca);
                    ca_check.set_tooltip_text(Some(tr(
                        "Skip CA server certificate validation (recommended for university networks)",
                        "Не проверять сертификат CA (рекомендуется для университетских сетей)",
                    )));
                    let s_ca = sender.clone();
                    ca_check.connect_toggled(move |_| {
                        s_ca.input(NetworkMsg::ToggleNoCaCert);
                    });
                    drawer.append(&ca_check);
                }

                // 4. Error label (if any)
                if let Some(err) = cur_err {
                    let err_lbl = Label::new(Some(err));
                    err_lbl.add_css_class("auth-error");
                    err_lbl.set_xalign(0.0);
                    err_lbl.set_wrap(true);
                    drawer.append(&err_lbl);
                }

                // 5. Append Connect Button
                drawer.append(&conn_btn);

                card.append(&drawer);
            }
        }

        list_box.append(&card);
    }

    expanded_card
}

pub fn build_network_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = NetworkInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let mut controller = NetworkModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 390, 420)
}
