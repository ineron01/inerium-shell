pub mod auth_form;
pub mod backend;
pub mod wifi_list;

pub use auth_form::AuthFormState;
pub use backend::{query_saved_wifi_connections, query_wifi_networks, SavedConn, WifiAp};

use std::cell::Cell;
use std::collections::HashMap;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType};
use inerium_i18n::tr;
use relm4::prelude::*;

use backend::parse_cmd_output;
use wifi_list::populate_wifi_list;
use crate::hw;

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
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

impl NetworkModel {
    fn sync_active_network(&mut self) {
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
    InitialScanLoaded {
        saved: HashMap<String, SavedConn>,
        networks: Vec<WifiAp>,
        active_details: Option<hw::ActiveWifiDetails>,
    },
    ScanProgress {
        networks: Vec<WifiAp>,
        saved: HashMap<String, SavedConn>,
        active_ssid: Option<String>,
        hotspot_active: bool,
    },
    RescanFinished {
        networks: Vec<WifiAp>,
        saved: HashMap<String, SavedConn>,
        active_ssid: Option<String>,
        hotspot_active: bool,
    },
    PollStatus,
    StatusPolled {
        ssid: Option<String>,
        hotspot: bool,
        details: Option<hw::ActiveWifiDetails>,
    },
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
        saved: HashMap<String, SavedConn>,
        active_ssid: Option<String>,
        hotspot_active: bool,
    },
    DisconnectFinished {
        networks: Vec<WifiAp>,
        saved: HashMap<String, SavedConn>,
        active_ssid: Option<String>,
        hotspot_active: bool,
    },
    StopHotspot,
    Back,
}

pub struct NetworkInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

fn fetch_wifi_scan_data(rescan: bool) -> (Vec<WifiAp>, HashMap<String, SavedConn>, Option<String>, bool) {
    let saved = query_saved_wifi_connections();
    let mut nets = query_wifi_networks(rescan);
    for n in &mut nets {
        n.is_saved = saved.contains_key(&n.ssid);
    }
    let act = hw::get_active_wifi_ssid();
    let hs = hw::is_hotspot_active();
    (nets, saved, act, hs)
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
                set_policy: (PolicyType::Never, PolicyType::External),
                set_overlay_scrolling: false,
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
        let active_details = None;
        let saved_connections = HashMap::new();
        let networks = Vec::new();

        let s_init = sender.clone();
        thread::spawn(move || {
            let saved = query_saved_wifi_connections();
            let mut nets = if wifi_enabled && !hotspot_active {
                query_wifi_networks(false)
            } else {
                Vec::new()
            };
            for net in &mut nets {
                net.is_saved = saved.contains_key(&net.ssid);
            }
            let details = hw::get_active_wifi_details();
            s_init.input(NetworkMsg::InitialScanLoaded {
                saved,
                networks: nets,
                active_details: details,
            });
        });

        let model = NetworkModel {
            wifi_enabled,
            is_scanning: wifi_enabled && !hotspot_active,
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
                    self.auth_state = None;
                    crate::set_modal_dialog_active(false);
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
                            let (cached, saved, act, hs) = fetch_wifi_scan_data(false);
                            if !cached.is_empty() {
                                s.input(NetworkMsg::ScanProgress {
                                    networks: cached,
                                    saved,
                                    active_ssid: act,
                                    hotspot_active: hs,
                                });
                            }

                            let fresh_data = fetch_wifi_scan_data(true);
                            let min_scan_time = Duration::from_millis(1500);
                            let elapsed = start.elapsed();
                            if elapsed < min_scan_time {
                                thread::sleep(min_scan_time - elapsed);
                            }
                            s.input(NetworkMsg::RescanFinished {
                                networks: fresh_data.0,
                                saved: fresh_data.1,
                                active_ssid: fresh_data.2,
                                hotspot_active: fresh_data.3,
                            });
                        } else {
                            s.input(NetworkMsg::RescanFinished {
                                networks: Vec::new(),
                                saved: HashMap::new(),
                                active_ssid: None,
                                hotspot_active: false,
                            });
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
                    let (fresh, saved, act, hs) = fetch_wifi_scan_data(true);
                    let min_scan_time = Duration::from_millis(1500);
                    let elapsed = start.elapsed();
                    if elapsed < min_scan_time {
                        thread::sleep(min_scan_time - elapsed);
                    }
                    s.input(NetworkMsg::RescanFinished {
                        networks: fresh,
                        saved,
                        active_ssid: act,
                        hotspot_active: hs,
                    });
                });
            }
            NetworkMsg::PollStatus => {
                let s_poll = sender.clone();
                thread::spawn(move || {
                    let ssid = hw::get_active_wifi_ssid();
                    let hotspot = hw::is_hotspot_active();
                    let details = hw::get_active_wifi_details();
                    s_poll.input(NetworkMsg::StatusPolled {
                        ssid,
                        hotspot,
                        details,
                    });
                });
            }
            NetworkMsg::StatusPolled {
                ssid,
                hotspot,
                details,
            } => {
                let ssid_changed = ssid != self.active_ssid;
                let hotspot_changed = hotspot != self.hotspot_active;
                let ip_changed = details.as_ref().map(|d| &d.ip) != self.active_details.as_ref().map(|d| &d.ip);

                if ssid_changed || hotspot_changed || ip_changed {
                    self.active_ssid = ssid;
                    self.hotspot_active = hotspot;
                    self.active_details = details;
                    self.sync_active_network();
                    hw::notify_network_changed();
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
                crate::set_modal_dialog_active(self.auth_state.is_some());
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
                crate::set_modal_dialog_active(false);
                let s = sender.clone();
                thread::spawn(move || {
                    hw::delete_connection(&uuid);
                    let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                    s.input(NetworkMsg::RescanFinished {
                        networks: fresh,
                        saved,
                        active_ssid: act,
                        hotspot_active: hs,
                    });
                });
            }
            NetworkMsg::InitialScanLoaded {
                saved,
                mut networks,
                active_details,
            } => {
                self.is_scanning = false;
                if !self.wifi_enabled {
                    return;
                }
                self.saved_connections = saved;
                self.active_details = active_details;
                if let Some(ref act) = self.active_ssid {
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
                self.networks = networks;
                self.sync_active_network();
            }
            NetworkMsg::ScanProgress {
                networks,
                saved,
                active_ssid,
                hotspot_active,
            } => {
                if !self.wifi_enabled {
                    return;
                }
                self.saved_connections = saved;
                self.active_ssid = active_ssid;
                self.hotspot_active = hotspot_active;
                self.networks = networks;
                self.sync_active_network();
            }
            NetworkMsg::RescanFinished {
                networks,
                saved,
                active_ssid,
                hotspot_active,
            } => {
                self.is_scanning = false;
                if !self.wifi_enabled {
                    self.networks.clear();
                    self.active_ssid = None;
                    self.sync_active_network();
                    return;
                }
                self.saved_connections = saved;
                self.active_ssid = active_ssid;
                self.hotspot_active = hotspot_active;
                self.networks = networks;
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
                        let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                        s_fin.input(NetworkMsg::DisconnectFinished {
                            networks: fresh,
                            saved,
                            active_ssid: act,
                            hotspot_active: hs,
                        });
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
                        let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                            saved,
                            active_ssid: act,
                            hotspot_active: hs,
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
                        let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                            saved,
                            active_ssid: act,
                            hotspot_active: hs,
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
                        let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                        s_fin.input(NetworkMsg::ConnectFinished {
                            ssid: ssid_task,
                            success,
                            error: if !success { Some(err_msg) } else { None },
                            networks: fresh,
                            saved,
                            active_ssid: act,
                            hotspot_active: hs,
                        });
                    });
                }
            }
            NetworkMsg::ConnectFinished {
                ssid,
                success,
                error,
                networks,
                saved,
                active_ssid,
                hotspot_active,
            } => {
                self.saved_connections = saved;
                self.active_ssid = active_ssid;
                self.hotspot_active = hotspot_active;
                self.networks = networks;
                self.sync_active_network();
                hw::notify_network_changed();
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
                crate::set_modal_dialog_active(self.auth_state.is_some());
            }
            NetworkMsg::DisconnectFinished {
                networks,
                saved,
                active_ssid,
                hotspot_active,
            } => {
                self.saved_connections = saved;
                self.active_ssid = active_ssid;
                self.hotspot_active = hotspot_active;
                self.networks = networks;
                self.sync_active_network();
                hw::notify_network_changed();
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
                    let (fresh, saved, act, hs) = fetch_wifi_scan_data(false);
                    s_fin.input(NetworkMsg::RescanFinished {
                        networks: fresh,
                        saved,
                        active_ssid: act,
                        hotspot_active: hs,
                    });
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

pub fn build_network_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32, Box<dyn std::any::Any>)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = NetworkInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let controller = NetworkModel::builder().launch(init).detach();
    let root = controller.widget().clone();
    (root, 390, 420, Box::new(controller))
}
