use std::process::Command;
use std::thread;

use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation};
use inerium_i18n::tr;
use relm4::prelude::*;

use crate::hw;

#[derive(Clone, Debug)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
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
                devices.push(BtDevice {
                    mac,
                    name,
                    connected,
                    paired,
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
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum BtMsg {
    TogglePower(bool),
    StartScan,
    ScanProgress(Vec<BtDevice>),
    ScanFinished(Vec<BtDevice>),
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
            set_size_request: (380, -1),

            // Header
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: &tr("‹ Back", "‹ Назад"),
                    connect_clicked => BtMsg::Back,
                },

                gtk4::Label {
                    add_css_class: "cc-host",
                    set_label: "Bluetooth",
                    set_hexpand: true,
                    set_xalign: 0.0,
                },

                #[name = "pwr_switch"]
                gtk4::Switch {
                    set_active: model.enabled,
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
                    set_tooltip_text: Some(tr("Open Bluetooth Manager", "Открыть менеджер Bluetooth")),
                    set_label: tr("󰒓 Manager", "󰒓 Менеджер"),
                    connect_clicked => BtMsg::OpenManager,
                },

                #[name = "scan_btn"]
                gtk4::Button {
                    add_css_class: "cc-chip",
                    #[watch]
                    set_sensitive: !model.scanning,
                    #[watch]
                    set_label: if model.scanning {
                        tr("󰑐 Scanning...", "󰑐 Поиск...")
                    } else {
                        tr("󰑐 Scan", "󰑐 Поиск")
                    },
                    connect_clicked => BtMsg::StartScan,
                },
            },

            // Devices List
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),
                set_propagate_natural_height: true,
                set_min_content_height: 160,
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
        let enabled = hw::is_bluetooth_enabled();
        let devices = if enabled { query_devices() } else { Vec::new() };

        let model = BluetoothModel {
            enabled,
            scanning: false,
            devices: devices.clone(),
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        populate_list(&widgets.list_box, &devices, enabled, &sender);

        ComponentParts { model, widgets }
    }

    fn post_view() {
        populate_list(list_box, &self.devices, self.enabled, &sender);
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BtMsg::TogglePower(state) => {
                self.enabled = state;
                hw::toggle_bluetooth(state);

                let s = sender.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                    let devs = if state { query_devices() } else { Vec::new() };
                    s.input(BtMsg::ScanFinished(devs));
                });
            }
            BtMsg::StartScan => {
                self.scanning = true;
                let s = sender.clone();
                thread::spawn(move || {
                    // Non-interactive discovery requires passing --timeout to bluetoothctl,
                    // otherwise it exits immediately in 5ms without discovering devices.
                    let mut child = Command::new("timeout")
                        .args(["10", "bluetoothctl", "--timeout", "7", "scan", "on"])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();

                    // Periodically query devices so discovered hardware pops up live
                    for _ in 0..3 {
                        thread::sleep(std::time::Duration::from_secs(2));
                        let interim = query_devices();
                        s.input(BtMsg::ScanProgress(interim));
                    }

                    if let Ok(ref mut proc) = child {
                        let _ = proc.wait();
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
    enabled: bool,
    sender: &ComponentSender<BluetoothModel>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    if devices.is_empty() {
        let empty_lbl = Label::new(Some(if enabled {
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
        return;
    }

    for dev in devices {
        let row = Button::new();
        row.add_css_class("card-item");
        if dev.connected {
            row.add_css_class("active");
        }

        let inner = gtk4::Box::new(Orientation::Horizontal, 8);
        let ico_str = if dev.name.to_lowercase().contains("buds")
            || dev.name.to_lowercase().contains("head")
        {
            "󰋋"
        } else if dev.name.to_lowercase().contains("mouse") {
            "󰍽"
        } else if dev.name.to_lowercase().contains("key") {
            "󰌌"
        } else {
            "󰂯"
        };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        inner.append(&ico);

        let txt = gtk4::Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&dev.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);

        let mac_lbl = Label::new(Some(&dev.mac));
        mac_lbl.add_css_class("card-sub");
        mac_lbl.set_xalign(0.0);
        txt.append(&mac_lbl);
        inner.append(&txt);

        let badge_text = if dev.connected {
            tr("󰄬 Connected", "󰄬 Подключено")
        } else if dev.paired {
            tr("Paired", "Сохранено")
        } else {
            tr("New", "Новое")
        };
        let badge = Label::new(Some(badge_text));
        badge.add_css_class("card-badge");
        badge.set_visible(true);
        inner.append(&badge);

        row.set_child(Some(&inner));

        let mac = dev.mac.clone();
        let is_conn = dev.connected;
        let is_paired = dev.paired;
        let s = sender.clone();
        let r_ui = row.clone();
        let b_ui = badge.clone();

        row.connect_clicked(move |_| {
            r_ui.set_sensitive(false);
            b_ui.set_label(if is_conn {
                tr("󰑐 Disconnecting...", "󰑐 Отключение...")
            } else {
                tr("󰑐 Connecting...", "󰑐 Подключение...")
            });
            b_ui.set_visible(true);
            s.input(BtMsg::ConnectDevice(mac.clone(), is_conn, is_paired));
        });

        list_box.append(&row);
    }
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
    (root, 380, 360)
}
