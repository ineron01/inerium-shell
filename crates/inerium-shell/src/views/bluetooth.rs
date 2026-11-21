use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use gdk4::BUTTON_SECONDARY;
use gtk4::prelude::*;
use gtk4::{Align, Button, Label, Orientation};
use inerium_i18n::tr;
use relm4::prelude::*;

use crate::hw;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
    pub trusted: bool,
    pub battery: Option<u8>,
}

pub fn query_devices() -> Vec<BtDevice> {
    let mut devices = Vec::new();
    if !crate::hw::has_bluetooth() {
        return devices;
    }

    // Query connected devices
    let mut connected_macs = std::collections::HashSet::new();
    if let Ok(out) = Command::new("timeout")
        .args(["2", "bluetoothctl", "devices", "Connected"])
        .stdin(std::process::Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                connected_macs.insert(parts[1].to_string());
            }
        }
    }

    // Query paired devices
    let mut paired_macs = std::collections::HashSet::new();
    if let Ok(out) = Command::new("timeout")
        .args(["2", "bluetoothctl", "devices", "Paired"])
        .stdin(std::process::Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                paired_macs.insert(parts[1].to_string());
            }
        }
    }

    // Query all discovered/cached devices
    if let Ok(out) = Command::new("timeout")
        .args(["2", "bluetoothctl", "devices"])
        .stdin(std::process::Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let mut seen = std::collections::HashSet::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                let mac = parts[1].to_string();
                if seen.contains(&mac) {
                    continue;
                }
                seen.insert(mac.clone());
                let name = parts[2..].join(" ");
                let connected = connected_macs.contains(&mac);
                let paired = paired_macs.contains(&mac);
                let (trusted, battery) = if connected || paired {
                    hw::get_bluetooth_device_meta(&mac)
                } else {
                    (false, None)
                };
                devices.push(BtDevice {
                    mac,
                    name,
                    connected,
                    paired,
                    trusted,
                    battery,
                });
            }
        }
    }

    fn is_friendly_name(name: &str, mac: &str) -> bool {
        let n = name.trim();
        if n.is_empty() || n.eq_ignore_ascii_case(mac) {
            return false;
        }
        let mac_hyphens = mac.replace(':', "-");
        !n.eq_ignore_ascii_case(&mac_hyphens)
    }

    // Sort order:
    // 1. Connected devices
    // 2. Paired devices
    // 3. Named discovered devices
    // 4. Raw MAC address beacons
    // Alphabetical within same tier
    devices.sort_by(|a, b| {
        let a_tier = if a.connected {
            0
        } else if a.paired {
            1
        } else if is_friendly_name(&a.name, &a.mac) {
            2
        } else {
            3
        };
        let b_tier = if b.connected {
            0
        } else if b.paired {
            1
        } else if is_friendly_name(&b.name, &b.mac) {
            2
        } else {
            3
        };

        a_tier
            .cmp(&b_tier)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    devices
}

pub struct BluetoothModel {
    enabled: bool,
    scanning: bool,
    devices: Vec<BtDevice>,
    expanded_mac: Option<String>,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum BtMsg {
    TogglePower(bool),
    StartScan,
    ScanProgress(Vec<BtDevice>),
    ScanFinished(Vec<BtDevice>),
    PollStatus,
    DevicesUpdated(Vec<BtDevice>),
    ToggleExpandDevice(String),
    ToggleTrusted(String, bool),
    ForgetDevice(String),
    ConnectDevice(String, bool, bool),
    ConnectFinished(Vec<BtDevice>),
    OpenManager,
    Back,
}

pub struct BluetoothInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for BluetoothModel {
    type Input = BtMsg;
    type Output = ();
    type Init = BluetoothInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (380, 400),

            // Header
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,
                set_valign: Align::Center,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: &tr("‹ Back", "‹ Назад"),
                    set_valign: Align::Center,
                    connect_clicked => BtMsg::Back,
                },

                gtk4::Label {
                    add_css_class: "cc-host",
                    set_label: "Bluetooth",
                    set_hexpand: true,
                    set_xalign: 0.0,
                    set_valign: Align::Center,
                },

                #[name = "pwr_switch"]
                gtk4::Switch {
                    set_active: model.enabled,
                    set_valign: Align::Center,
                    connect_state_set[sender] => move |_, state| {
                        sender.input(BtMsg::TogglePower(state));
                        glib::Propagation::Proceed
                    },
                },
            },

            // Subheader with Scan and Manager
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Label {
                    add_css_class: "cc-label",
                    set_label: &tr("DEVICES", "УСТРОЙСТВА"),
                    set_xalign: 0.0,
                    set_hexpand: true,
                },

                #[name = "mgr_btn"]
                gtk4::Button {
                    add_css_class: "cc-chip",
                    #[watch]
                    set_sensitive: model.enabled,
                    set_tooltip_text: Some(tr("Open Bluetooth Manager", "Открыть менеджер Bluetooth")),
                    set_label: tr("󰒓  Manager", "󰒓  Менеджер"),
                    connect_clicked => BtMsg::OpenManager,
                },

                #[name = "scan_btn"]
                gtk4::Button {
                    add_css_class: "cc-chip",
                    #[watch]
                    set_sensitive: model.enabled && !model.scanning,
                    #[watch]
                    set_label: if model.scanning {
                        tr("󰑐  Scanning...", "󰑐  Поиск...")
                    } else {
                        tr("󰑐  Scan", "󰑐  Поиск")
                    },
                    connect_clicked => BtMsg::StartScan,
                },
            },

            // Devices List
            #[name = "scroll_win"]
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),
                set_propagate_natural_height: false,
                set_min_content_height: 280,
                set_max_content_height: 280,
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
        let enabled = hw::is_bluetooth_enabled();
        let devices = Vec::new();

        let model = BluetoothModel {
            enabled,
            scanning: false,
            devices: devices.clone(),
            expanded_mac: None,
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        let _ = populate_list(&widgets.list_box, &devices, None, enabled, false, &sender);

        if enabled {
            let s_init = sender.clone();
            std::thread::spawn(move || {
                let fresh = query_devices();
                s_init.input(BtMsg::DevicesUpdated(fresh));
            });
        }

        let root_ref = root.clone();
        let s_poll = sender.clone();
        glib::timeout_add_local(Duration::from_millis(1500), move || {
            if !root_ref.is_mapped() {
                return glib::ControlFlow::Break;
            }
            s_poll.input(BtMsg::PollStatus);
            glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn post_view() {
        let vadj = scroll_win.vadjustment();
        let prev_val = vadj.value();

        let exp_widget = populate_list(
            list_box,
            &self.devices,
            self.expanded_mac.as_deref(),
            self.enabled,
            self.scanning,
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

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BtMsg::PollStatus => {
                if !self.scanning && self.enabled {
                    let s_poll = sender.clone();
                    std::thread::spawn(move || {
                        let fresh = query_devices();
                        s_poll.input(BtMsg::DevicesUpdated(fresh));
                    });
                }
            }
            BtMsg::DevicesUpdated(fresh) => {
                if !self.scanning && self.enabled && fresh != self.devices {
                    self.devices = fresh;
                }
            }
            BtMsg::TogglePower(state) => {
                self.enabled = state;
                if !state {
                    self.scanning = false;
                    self.devices.clear();
                    hw::toggle_bluetooth(false);
                } else {
                    self.scanning = true;
                    self.devices.clear();
                    let s = sender.clone();
                    thread::spawn(move || {
                        let start = std::time::Instant::now();
                        hw::set_bluetooth_enabled_sync(true);

                        let ready = hw::wait_for_bluetooth_ready(Duration::from_secs(3));
                        if ready {
                            let cached = query_devices();
                            if !cached.is_empty() {
                                s.input(BtMsg::ScanProgress(cached));
                            }

                            let mut child = Command::new("timeout")
                                .args(["10", "bluetoothctl", "--timeout", "7", "scan", "on"])
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn();

                            for _ in 0..3 {
                                thread::sleep(Duration::from_secs(2));
                                let interim = query_devices();
                                s.input(BtMsg::ScanProgress(interim));
                            }

                            if let Ok(ref mut proc) = child {
                                let _ = proc.wait();
                            }

                            let min_scan_time = Duration::from_millis(1500);
                            let elapsed = start.elapsed();
                            if elapsed < min_scan_time {
                                thread::sleep(min_scan_time - elapsed);
                            }

                            let fresh = query_devices();
                            s.input(BtMsg::ScanFinished(fresh));
                        } else {
                            s.input(BtMsg::ScanFinished(Vec::new()));
                        }
                    });
                }
            }
            BtMsg::StartScan => {
                if !self.enabled {
                    return;
                }
                self.scanning = true;
                let s = sender.clone();
                thread::spawn(move || {
                    let start = std::time::Instant::now();
                    let _ = hw::wait_for_bluetooth_ready(Duration::from_secs(2));

                    let mut child = Command::new("timeout")
                        .args(["10", "bluetoothctl", "--timeout", "7", "scan", "on"])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn();

                    for _ in 0..3 {
                        thread::sleep(Duration::from_secs(2));
                        let interim = query_devices();
                        s.input(BtMsg::ScanProgress(interim));
                    }

                    if let Ok(ref mut proc) = child {
                        let _ = proc.wait();
                    }

                    let min_scan_time = Duration::from_millis(1500);
                    let elapsed = start.elapsed();
                    if elapsed < min_scan_time {
                        thread::sleep(min_scan_time - elapsed);
                    }

                    let fresh = query_devices();
                    s.input(BtMsg::ScanFinished(fresh));
                });
            }
            BtMsg::ScanProgress(devices) => {
                self.devices = devices;
            }
            BtMsg::ScanFinished(fresh) => {
                self.scanning = false;
                self.devices = fresh;
            }
            BtMsg::ConnectDevice(mac, is_conn, is_paired) => {
                let s = sender.clone();
                thread::spawn(move || {
                    if is_conn {
                        let _ = Command::new("timeout")
                            .args(["5", "bluetoothctl", "disconnect", &mac])
                            .output();
                    } else {
                        // For un-paired devices, trust and pair first before connecting
                        if !is_paired {
                            let _ = Command::new("timeout")
                                .args(["3", "bluetoothctl", "trust", &mac])
                                .output();
                            let _ = Command::new("timeout")
                                .args(["6", "bluetoothctl", "pair", &mac])
                                .output();
                        }
                        let _ = Command::new("timeout")
                            .args(["8", "bluetoothctl", "connect", &mac])
                            .output();
                    }
                    let fresh = query_devices();
                    s.input(BtMsg::ConnectFinished(fresh));
                });
            }
            BtMsg::ConnectFinished(fresh) => {
                self.devices = fresh;
            }
            BtMsg::ToggleExpandDevice(mac) => {
                if self.expanded_mac.as_ref() == Some(&mac) {
                    self.expanded_mac = None;
                } else {
                    self.expanded_mac = Some(mac);
                }
            }
            BtMsg::ToggleTrusted(mac, trust) => {
                let s = sender.clone();
                thread::spawn(move || {
                    hw::set_bluetooth_trusted(&mac, trust);
                    let fresh = query_devices();
                    s.input(BtMsg::ScanFinished(fresh));
                });
            }
            BtMsg::ForgetDevice(mac) => {
                self.expanded_mac = None;
                let s = sender.clone();
                thread::spawn(move || {
                    hw::remove_bluetooth_device(&mac);
                    let fresh = query_devices();
                    s.input(BtMsg::ScanFinished(fresh));
                });
            }
            BtMsg::OpenManager => {
                for bin in ["blueman-manager", "overskride", "blueberry"] {
                    if Command::new("which")
                        .arg(bin)
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false)
                    {
                        let _ = Command::new(bin).spawn();
                        break;
                    }
                }
                (self.close_cb)();
            }
            BtMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }


}

fn populate_list(
    list_box: &gtk4::Box,
    devices: &[BtDevice],
    expanded_mac: Option<&str>,
    enabled: bool,
    scanning: bool,
    sender: &ComponentSender<BluetoothModel>,
) -> Option<gtk4::Box> {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    if devices.is_empty() {
        let empty_lbl = Label::new(Some(if scanning {
            tr(
                "󰑐 Searching for Bluetooth devices...",
                "󰑐 Поиск Bluetooth-устройств...",
            )
        } else if enabled {
            tr(
                "No devices found. Click «Scan»",
                "Устройства не найдены. Нажмите «Поиск»",
            )
        } else {
            tr("Bluetooth is disabled", "Bluetooth выключен")
        }));
        empty_lbl.add_css_class("cc-meta");
        empty_lbl.set_margin_top(20);
        list_box.append(&empty_lbl);
        return None;
    }

    let mut expanded_card = None;

    for dev in devices {
        let card = gtk4::Box::new(Orientation::Vertical, 0);
        card.add_css_class("card-item");
        if dev.connected {
            card.add_css_class("active");
        }
        let is_exp = expanded_mac == Some(dev.mac.as_str());
        if is_exp {
            card.add_css_class("expanded");
            expanded_card = Some(card.clone());
        }

        // Header row
        let header_row = gtk4::Box::new(Orientation::Horizontal, 8);

        // Clickable main area (icon + text + badge)
        let main_box = gtk4::Box::new(Orientation::Horizontal, 8);
        main_box.set_hexpand(true);
        main_box.set_cursor_from_name(Some("pointer"));

        let (ico_str, ico_class) = if dev.name.to_lowercase().contains("buds")
            || dev.name.to_lowercase().contains("head")
        {
            ("󰋋", "audio")
        } else if dev.name.to_lowercase().contains("mouse") {
            ("󰍽", "input")
        } else if dev.name.to_lowercase().contains("key") {
            ("󰌌", "input")
        } else {
            ("󰂯", "default")
        };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        if dev.connected {
            ico.add_css_class("connected");
        } else if ico_class != "default" {
            ico.add_css_class(ico_class);
        }
        main_box.append(&ico);

        let txt = gtk4::Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&dev.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);

        let sub_str = if dev.connected {
            if let Some(pct) = dev.battery {
                format!("{} • 󰥈 {}%", tr("Connected", "Подключено"), pct)
            } else {
                tr("Connected", "Подключено").to_string()
            }
        } else {
            dev.mac.clone()
        };
        let sub_lbl = Label::new(Some(&sub_str));
        sub_lbl.add_css_class("card-sub");
        sub_lbl.set_xalign(0.0);
        txt.append(&sub_lbl);
        main_box.append(&txt);

        let badge = Label::new(None);
        badge.add_css_class("card-badge");
        if dev.connected {
            badge.set_label(tr("󰄬  Connected", "󰄬  Подключено"));
            badge.add_css_class("connected");
            badge.set_visible(true);
        } else if dev.paired {
            badge.set_label(tr("Paired", "Сохранено"));
            badge.add_css_class("paired");
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
        let mac_chev = dev.mac.clone();
        chevron_btn.connect_clicked(move |_| {
            s_chev.input(BtMsg::ToggleExpandDevice(mac_chev.clone()));
        });
        header_row.append(&chevron_btn);

        card.append(&header_row);

        // Tooltip on the entire card
        let mut tip = format!(
            "{}: {}\nMAC: {}\n{}: {}",
            tr("Device", "Устройство"),
            dev.name,
            dev.mac,
            tr("Status", "Статус"),
            if dev.connected {
                tr("Connected", "Подключено")
            } else if dev.paired {
                tr("Paired", "Сопряжено")
            } else {
                tr("Available", "Доступно")
            }
        );
        if let Some(pct) = dev.battery {
            tip.push_str(&format!("\n{}: {}%", tr("Battery", "Батарея"), pct));
        }
        if dev.paired || dev.connected {
            tip.push_str(&format!(
                "\n{}: {}",
                tr("Auto-connect", "Автоподключение"),
                if dev.trusted {
                    tr("On", "Вкл")
                } else {
                    tr("Off", "Выкл")
                }
            ));
        }
        card.set_tooltip_text(Some(&tip));

        // Primary click on main_box -> connect / disconnect / pair
        let mac = dev.mac.clone();
        let is_conn = dev.connected;
        let is_paired = dev.paired;
        let s = sender.clone();
        let b_ui = badge.clone();
        let main_gesture = gtk4::GestureClick::new();
        main_gesture.set_button(gdk4::BUTTON_PRIMARY);
        main_gesture.connect_released(move |_, _, _, _| {
            b_ui.set_label(if is_conn {
                tr("󰑐 Disconnecting...", "󰑐 Отключение...")
            } else {
                tr("󰑐 Connecting...", "󰑐 Подключение...")
            });
            b_ui.set_visible(true);
            s.input(BtMsg::ConnectDevice(mac.clone(), is_conn, is_paired));
        });
        main_box.add_controller(main_gesture);

        // Right-click gesture (ПКМ) on the entire card -> toggle expand
        let s_right = sender.clone();
        let mac_right = dev.mac.clone();
        let right_gesture = gtk4::GestureClick::new();
        right_gesture.set_button(BUTTON_SECONDARY);
        right_gesture.connect_pressed(move |_, _, _, _| {
            s_right.input(BtMsg::ToggleExpandDevice(mac_right.clone()));
        });
        card.add_controller(right_gesture);

        // Integrated Accordion Drawer INSIDE the card (for ALL devices!)
        if is_exp {
            let sep = gtk4::Separator::new(Orientation::Horizontal);
            sep.add_css_class("card-drawer-sep");
            card.append(&sep);

            let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
            drawer.add_css_class("card-drawer-inner");
            drawer.set_hexpand(true);
            drawer.set_homogeneous(true);

            if dev.paired || dev.connected {
                // 1. Autoconnect (Trusted) toggle chip
                let trust_btn = Button::new();
                trust_btn.add_css_class("drawer-chip");
                trust_btn.set_hexpand(true);
                if dev.trusted {
                    trust_btn.add_css_class("on");
                    trust_btn.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                } else {
                    trust_btn.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                }
                trust_btn.set_tooltip_text(Some(tr(
                    "Allow automatic connection from this device",
                    "Автоматически подключаться к этому устройству при обнаружении",
                )));
                let s_trust = sender.clone();
                let mac_trust = dev.mac.clone();
                let cur_trust = dev.trusted;
                let b_trust = trust_btn.clone();
                trust_btn.connect_clicked(move |_| {
                    b_trust.set_sensitive(false);
                    if cur_trust {
                        b_trust.remove_css_class("on");
                        b_trust.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                    } else {
                        b_trust.add_css_class("on");
                        b_trust.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                    }
                    s_trust.input(BtMsg::ToggleTrusted(mac_trust.clone(), !cur_trust));
                });
                drawer.append(&trust_btn);

                // 2. Forget device chip
                let forget_btn = Button::with_label(tr("󰆴  Forget", "󰆴  Забыть"));
                forget_btn.add_css_class("drawer-chip");
                forget_btn.add_css_class("danger");
                forget_btn.set_hexpand(true);
                forget_btn.set_tooltip_text(Some(tr(
                    "Remove and unpair this Bluetooth device",
                    "Удалить сопряжение с устройством",
                )));
                let s_forget = sender.clone();
                let mac_forget = dev.mac.clone();
                let b_forget = forget_btn.clone();
                forget_btn.connect_clicked(move |_| {
                    b_forget.set_sensitive(false);
                    b_forget.set_label(tr("󰑐  Forgetting...", "󰑐  Удаление..."));
                    s_forget.input(BtMsg::ForgetDevice(mac_forget.clone()));
                });
                drawer.append(&forget_btn);

                // 3. Connect / Disconnect chip
                if dev.connected {
                    let disc_btn = Button::with_label(tr("󰑐  Disconnect", "󰑐  Отключить"));
                    disc_btn.add_css_class("drawer-chip");
                    disc_btn.set_hexpand(true);
                    let s_d = sender.clone();
                    let m_d = dev.mac.clone();
                    let b_disc = disc_btn.clone();
                    disc_btn.connect_clicked(move |_| {
                        b_disc.set_sensitive(false);
                        b_disc.set_label(tr("󰑐  Disconnecting...", "󰑐  Отключение..."));
                        s_d.input(BtMsg::ConnectDevice(m_d.clone(), true, true));
                    });
                    drawer.append(&disc_btn);
                } else {
                    let conn_btn = Button::with_label(tr("󰄬  Connect", "󰄬  Подключить"));
                    conn_btn.add_css_class("drawer-chip");
                    conn_btn.set_hexpand(true);
                    let s_c = sender.clone();
                    let m_c = dev.mac.clone();
                    let b_conn = conn_btn.clone();
                    conn_btn.connect_clicked(move |_| {
                        b_conn.set_sensitive(false);
                        b_conn.set_label(tr("󰑐  Connecting...", "󰑐  Подключение..."));
                        s_c.input(BtMsg::ConnectDevice(m_c.clone(), false, true));
                    });
                    drawer.append(&conn_btn);
                }
            } else {
                // New/unpaired device
                let pair_btn = Button::with_label(tr("󰄬  Pair", "󰄬  Сопряжение"));
                pair_btn.add_css_class("drawer-chip");
                pair_btn.set_hexpand(true);
                let s_p = sender.clone();
                let m_p = dev.mac.clone();
                let b_pair = pair_btn.clone();
                pair_btn.connect_clicked(move |_| {
                    b_pair.set_sensitive(false);
                    b_pair.set_label(tr("󰑐 Pairing...", "󰑐 Сопряжение..."));
                    s_p.input(BtMsg::ConnectDevice(m_p.clone(), false, false));
                });
                drawer.append(&pair_btn);

                let info_chip = Label::new(Some(&format!("󰋼 MAC: {}", dev.mac)));
                info_chip.add_css_class("drawer-chip");
                info_chip.add_css_class("info");
                info_chip.set_hexpand(true);
                drawer.append(&info_chip);
            }

            card.append(&drawer);
        }

        list_box.append(&card);
    }

    expanded_card
}

pub fn build_bluetooth_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = BluetoothInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let mut controller = BluetoothModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 380, 400)
}
