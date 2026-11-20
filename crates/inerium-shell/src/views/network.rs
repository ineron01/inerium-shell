use std::cell::Cell;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use gdk4::BUTTON_SECONDARY;
use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation, PolicyType};
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
                let in_use = parts[0] == "*";
                let ssid = parts[1].replace("\\:", ":").trim().to_string();
                if ssid.is_empty() {
                    continue;
                }
                let signal: u32 = parts[2].parse().unwrap_or(50);
                let security = parts[3].to_string();
                let is_saved = saved_map.contains_key(&ssid);
                let is_active = in_use || active_ssid.as_deref() == Some(&ssid);

                if let Some(existing) = networks.iter_mut().find(|n| n.ssid == ssid) {
                    existing.signal = existing.signal.max(signal);
                    if is_active {
                        existing.is_active = true;
                    }
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
pub struct PasswordPromptState {
    pub ssid: String,
    pub password: String,
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
    pub password_prompt: Option<PasswordPromptState>,
    pub focus_pass_entry: Cell<bool>,
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
    ForgetSavedNetwork(String),
    SelectAp(WifiAp),
    PasswordInput(String),
    SubmitPassword,
    CancelPassword,
    ForgetPromptNetwork,
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
            set_size_request: (390, -1),

            // Header (Back, Title & Status subtitle, Power switch)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: tr("‹ Back", "‹ Назад"),
                    connect_clicked => NetworkMsg::Back,
                },

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 1,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_xalign: 0.0,
                        set_label: tr("Wi-Fi & Networks", "Wi-Fi и сети"),
                    },

                    #[name = "status_sub"]
                    gtk4::Label {
                        add_css_class: "card-sub",
                        set_xalign: 0.0,
                        #[watch]
                        set_label: &{
                            if model.hotspot_active {
                                tr("󱜠 Hotspot is active", "󱜠 Раздача Wi-Fi активна").to_string()
                            } else if !model.wifi_enabled {
                                tr("Wi-Fi is turned off", "Беспроводная сеть выключена").to_string()
                            } else if let Some(ref ssid) = model.active_ssid {
                                tr_fmt!("󰄬 Connected: {ssid}", "󰄬 Подключено: {ssid}", ssid = ssid)
                            } else {
                                tr("Not connected", "Не подключено").to_string()
                            }
                        },
                    },
                },

                #[name = "pwr_switch"]
                gtk4::Switch {
                    set_active: model.wifi_enabled,
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
                        tr("󰑐 Searching...", "󰑐 Поиск сетей...")
                    } else {
                        tr("󰑐 Refresh", "󰑐 Обновить")
                    },
                    connect_clicked => NetworkMsg::StartRescan,
                },
            },

            // Inline Password Auth Box (visible when password prompt is open)
            #[name = "auth_box"]
            gtk4::Box {
                add_css_class: "cc-card",
                set_orientation: Orientation::Vertical,
                set_spacing: 6,
                #[watch]
                set_visible: model.password_prompt.is_some(),

                #[name = "auth_target_lbl"]
                gtk4::Label {
                    add_css_class: "card-title",
                    set_xalign: 0.0,
                    #[watch]
                    set_label: &if let Some(ref p) = model.password_prompt {
                        tr_fmt!("Connecting to «{ssid}»", "Подключение к «{ssid}»", ssid = p.ssid)
                    } else {
                        String::new()
                    },
                },

                #[name = "pass_entry"]
                gtk4::PasswordEntry {
                    set_placeholder_text: Some(tr("Wi-Fi password", "Пароль Wi-Fi сети")),
                    set_show_peek_icon: true,
                    connect_changed[sender] => move |e| {
                        sender.input(NetworkMsg::PasswordInput(e.text().to_string()));
                    },
                    connect_activate => NetworkMsg::SubmitPassword,
                },

                #[name = "error_lbl"]
                gtk4::Label {
                    add_css_class: "auth-error",
                    set_xalign: 0.0,
                    set_wrap: true,
                    #[watch]
                    set_visible: model
                        .password_prompt
                        .as_ref()
                        .and_then(|p| p.error_msg.as_ref())
                        .is_some(),
                    #[watch]
                    set_label: &model
                        .password_prompt
                        .as_ref()
                        .and_then(|p| p.error_msg.as_ref())
                        .cloned()
                        .unwrap_or_default(),
                },

                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 8,

                    gtk4::Button {
                        add_css_class: "cc-chip",
                        add_css_class: "on",
                        #[watch]
                        set_sensitive: {
                            let is_connecting = model
                                .password_prompt
                                .as_ref()
                                .is_some_and(|p| p.is_connecting);
                            let is_valid = model
                                .password_prompt
                                .as_ref()
                                .is_some_and(|p| p.password.trim().len() >= 8);
                            !is_connecting && is_valid
                        },
                        #[watch]
                        set_label: if model
                            .password_prompt
                            .as_ref()
                            .is_some_and(|p| p.is_connecting)
                        {
                            tr("󰑐 Connecting...", "󰑐 Подключение...")
                        } else {
                            tr("Connect", "Подключить")
                        },
                        connect_clicked => NetworkMsg::SubmitPassword,
                    },

                    gtk4::Button {
                        add_css_class: "cc-chip",
                        set_label: tr("Cancel", "Отмена"),
                        connect_clicked => NetworkMsg::CancelPassword,
                    },

                    #[name = "forget_btn"]
                    gtk4::Button {
                        add_css_class: "cc-chip",
                        #[watch]
                        set_visible: {
                            if let Some(ref p) = model.password_prompt {
                                model.networks.iter().any(|n| n.ssid == p.ssid && n.is_saved)
                            } else {
                                false
                            }
                        },
                        set_label: tr("Forget", "Забыть"),
                        connect_clicked => NetworkMsg::ForgetPromptNetwork,
                    },
                },
            },

            // Scrolled Network List
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (PolicyType::Never, PolicyType::Automatic),
                set_propagate_natural_height: true,
                set_min_content_height: 180,
                set_max_content_height: 320,
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
            password_prompt: None,
            focus_pass_entry: Cell::new(false),
            needs_list_rebuild: Cell::new(false),
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        populate_wifi_list(
            &widgets.list_box,
            &model.networks,
            &model.saved_connections,
            model.active_details.as_ref(),
            model.expanded_ssid.as_deref(),
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
        if self.focus_pass_entry.get() {
            self.focus_pass_entry.set(false);
            pass_entry.grab_focus();
        }
        if self.password_prompt.is_none() && !pass_entry.text().is_empty() {
            pass_entry.set_text("");
        }
        if self.needs_list_rebuild.get() {
            self.needs_list_rebuild.set(false);
            populate_wifi_list(
                list_box,
                &self.networks,
                &self.saved_connections,
                self.active_details.as_ref(),
                self.expanded_ssid.as_deref(),
                self.hotspot_active,
                self.wifi_enabled,
                self.is_scanning,
                &sender,
            );
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
                } else {
                    self.expanded_ssid = Some(ssid);
                }
                self.needs_list_rebuild.set(true);
            }
            NetworkMsg::ToggleAutoconnect(uuid, enable) => {
                let s = sender.clone();
                thread::spawn(move || {
                    hw::set_connection_autoconnect(&uuid, enable);
                    s.input(NetworkMsg::PollStatus);
                });
            }
            NetworkMsg::ForgetSavedNetwork(uuid) => {
                self.expanded_ssid = None;
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
                                .args(["connection", "up", "uuid", &conn.uuid])
                                .output()
                        } else {
                            Command::new("nmcli")
                                .args(["connection", "up", "id", &ssid])
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
                    crate::set_modal_dialog_active(true);
                    self.focus_pass_entry.set(true);
                    self.password_prompt = Some(PasswordPromptState {
                        ssid: net.ssid,
                        password: String::new(),
                        error_msg: None,
                        is_connecting: false,
                    });
                } else {
                    let s_fin = sender.clone();
                    let ssid = net.ssid.clone();
                    thread::spawn(move || {
                        let res = Command::new("nmcli")
                            .args(["dev", "wifi", "connect", &ssid])
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
            NetworkMsg::PasswordInput(text) => {
                if let Some(ref mut p) = self.password_prompt {
                    p.password = text;
                    p.error_msg = None;
                }
            }
            NetworkMsg::SubmitPassword => {
                if let Some(ref mut prompt) = self.password_prompt {
                    if prompt.password.trim().len() < 8 {
                        return;
                    }
                    prompt.is_connecting = true;
                    prompt.error_msg = None;
                    let ssid = prompt.ssid.clone();
                    let pass = prompt.password.clone();
                    let s_fin = sender.clone();
                    thread::spawn(move || {
                        let saved_map = query_saved_wifi_connections();
                        if let Some(conn) = saved_map.get(&ssid) {
                            let _ = Command::new("nmcli")
                                .args(["connection", "delete", "uuid", &conn.uuid])
                                .output();
                        }
                        let _ = Command::new("nmcli")
                            .args(["connection", "delete", "id", &ssid])
                            .output();

                        let res = Command::new("nmcli")
                            .args(["dev", "wifi", "connect", &ssid, "password", &pass])
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
            NetworkMsg::CancelPassword => {
                crate::set_modal_dialog_active(false);
                self.password_prompt = None;
            }
            NetworkMsg::ForgetPromptNetwork => {
                if let Some(ref p) = self.password_prompt {
                    let ssid = p.ssid.clone();
                    crate::set_modal_dialog_active(false);
                    self.password_prompt = None;
                    let s_fin = sender.clone();
                    thread::spawn(move || {
                        let saved_map = query_saved_wifi_connections();
                        if let Some(conn) = saved_map.get(&ssid) {
                            let _ = Command::new("nmcli")
                                .args(["connection", "delete", "uuid", &conn.uuid])
                                .output();
                        }
                        let _ = Command::new("nmcli")
                            .args(["connection", "delete", "id", &ssid])
                            .output();
                        let fresh = query_wifi_networks(false);
                        s_fin.input(NetworkMsg::RescanFinished(fresh));
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
                    crate::set_modal_dialog_active(false);
                    self.password_prompt = None;
                } else if let Some(ref mut prompt) = self.password_prompt {
                    prompt.is_connecting = false;
                    let err_text = error.unwrap_or_else(|| {
                        tr("Connection error", "Ошибка подключения").to_string()
                    });
                    let clean = err_text
                        .trim_start_matches("Error: ")
                        .trim_start_matches("Ошибка: ");
                    prompt.error_msg = Some(format!("󰅖 {}", clean));
                    prompt.ssid = ssid;
                } else {
                    crate::set_modal_dialog_active(true);
                    self.focus_pass_entry.set(true);
                    let err_text = error.unwrap_or_else(|| {
                        tr("Connection error", "Ошибка подключения").to_string()
                    });
                    let clean = err_text
                        .trim_start_matches("Error: ")
                        .trim_start_matches("Ошибка: ");
                    self.password_prompt = Some(PasswordPromptState {
                        ssid,
                        password: String::new(),
                        error_msg: Some(format!("󰅖 {}", clean)),
                        is_connecting: false,
                    });
                }
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
    hotspot_active: bool,
    wifi_enabled: bool,
    is_scanning: bool,
    sender: &ComponentSender<NetworkModel>,
) {
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
        return;
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
        return;
    }

    for net in networks {
        let card = gtk4::Box::new(Orientation::Vertical, 0);
        card.add_css_class("card-item");
        if net.is_active {
            card.add_css_class("active");
        }
        let is_exp = expanded_ssid == Some(net.ssid.as_str());
        if is_exp {
            card.add_css_class("expanded");
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
        sec_lbl.set_xalign(0.0);
        txt.append(&sec_lbl);
        main_box.append(&txt);

        // Badge: Active ("󰄬 В сети") or Saved ("Сохранено")
        let badge = Label::new(None);
        badge.add_css_class("card-badge");
        if net.is_active {
            badge.set_label(tr("󰄬 Connected", "󰄬 В сети"));
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

            let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
            drawer.add_css_class("card-drawer-inner");

            if let Some(conn) = saved_conns.get(&net.ssid) {
                // 1. Autoconnect chip
                let auto_btn = Button::new();
                auto_btn.add_css_class("drawer-chip");
                if conn.autoconnect {
                    auto_btn.add_css_class("on");
                    auto_btn.set_label(tr("󰑐 Auto: On", "󰑐 Авто: Вкл"));
                } else {
                    auto_btn.set_label(tr("󰑐 Auto: Off", "󰑐 Авто: Выкл"));
                }
                auto_btn.set_tooltip_text(Some(tr(
                    "Toggle automatic connection",
                    "Автоматически подключаться к сети",
                )));
                let s_auto = sender.clone();
                let u_auto = conn.uuid.clone();
                let cur_auto = conn.autoconnect;
                auto_btn.connect_clicked(move |_| {
                    s_auto.input(NetworkMsg::ToggleAutoconnect(u_auto.clone(), !cur_auto));
                });
                drawer.append(&auto_btn);

                // 2. Forget network chip
                let forget_btn = Button::with_label(tr("󰆴 Forget Network", "󰆴 Забыть сеть"));
                forget_btn.add_css_class("drawer-chip");
                forget_btn.add_css_class("danger");
                forget_btn.set_tooltip_text(Some(tr(
                    "Delete saved Wi-Fi profile and credentials",
                    "Удалить сохранённый профиль и пароль сети",
                )));
                let s_forget = sender.clone();
                let u_forget = conn.uuid.clone();
                forget_btn.connect_clicked(move |_| {
                    s_forget.input(NetworkMsg::ForgetSavedNetwork(u_forget.clone()));
                });
                drawer.append(&forget_btn);

                // 3. Connect / Disconnect chip
                if net.is_active {
                    let disc_btn = Button::with_label(tr("󱚼 Disconnect", "󱚼 Отключиться"));
                    disc_btn.add_css_class("drawer-chip");
                    let s_disc = sender.clone();
                    let net_disc = net.clone();
                    disc_btn.connect_clicked(move |_| {
                        s_disc.input(NetworkMsg::SelectAp(net_disc.clone()));
                    });
                    drawer.append(&disc_btn);
                } else {
                    let conn_btn = Button::with_label(tr("󰄬 Connect", "󰄬 Подключить"));
                    conn_btn.add_css_class("drawer-chip");
                    let s_c = sender.clone();
                    let net_c = net.clone();
                    conn_btn.connect_clicked(move |_| {
                        s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                    });
                    drawer.append(&conn_btn);
                }
            } else {
                // Unsaved network actions
                // 1. Connect chip
                let conn_btn = Button::with_label(tr("󰄬 Connect", "󰄬 Подключить"));
                conn_btn.add_css_class("drawer-chip");
                let s_c = sender.clone();
                let net_c = net.clone();
                conn_btn.connect_clicked(move |_| {
                    s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                });
                drawer.append(&conn_btn);

                // 2. Technical info chip
                let sec_desc = if net.security.is_empty() {
                    tr("Open", "Открытая").to_string()
                } else {
                    net.security.clone()
                };
                let info_label = format!("󰋼 {} • {}%", sec_desc, net.signal);
                let info_chip = Label::new(Some(&info_label));
                info_chip.add_css_class("drawer-chip");
                info_chip.add_css_class("info");
                drawer.append(&info_chip);
            }

            card.append(&drawer);
        }

        list_box.append(&card);
    }
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
    (root, 390, 380)
}
