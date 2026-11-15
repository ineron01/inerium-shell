use std::process::Command;
use std::thread;

use gdk4::Key;
use gtk4::prelude::*;
use gtk4::{Align, EventControllerKey, GestureClick, Orientation};
use inerium_i18n::tr;
use relm4::prelude::*;

#[derive(Debug)]
pub enum PowerAction {
    Lock,
    Logout,
    Suspend,
    Reboot,
    Poweroff,
    Cancel,
}

pub struct PowerMenuModel {
    close_cb: Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for PowerMenuModel {
    type Input = PowerAction;
    type Output = ();
    type Init = Box<dyn Fn() + 'static>;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "power-backdrop",
            set_orientation: Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,
            set_halign: Align::Fill,
            set_valign: Align::Fill,

            #[name = "modal"]
            gtk4::Box {
                add_css_class: "power-modal",
                set_orientation: Orientation::Vertical,
                set_spacing: 20,
                set_size_request: (840, -1),
                set_hexpand: true,
                set_vexpand: true,
                set_halign: Align::Center,
                set_valign: Align::Center,

                // Header
                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 4,
                    set_halign: Align::Center,

                    gtk4::Label {
                        add_css_class: "power-title",
                        set_label: &tr("Power Management", "Управление питанием"),
                        set_halign: Align::Center,
                    },
                    gtk4::Label {
                        add_css_class: "power-subtitle",
                        set_label: &tr(
                            "Press 1–5 on keyboard or select an action",
                            "Нажмите 1–5 на клавиатуре или выберите действие",
                        ),
                        set_halign: Align::Center,
                    },
                },

                // Cards grid
                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 14,
                    set_homogeneous: true,

                    // 1. Lock
                    gtk4::Button {
                        add_css_class: "power-card",
                        connect_clicked => PowerAction::Lock,

                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,
                            set_halign: Align::Center,
                            set_valign: Align::Center,

                            gtk4::Label {
                                add_css_class: "power-card-ico",
                                set_label: "󰌾",
                            },
                            gtk4::Label {
                                add_css_class: "power-card-label",
                                set_label: &tr("Lock", "Блокировка"),
                            },
                            gtk4::Label {
                                add_css_class: "power-card-key",
                                set_label: "1",
                                set_halign: Align::Center,
                                set_valign: Align::Center,
                            },
                        },
                    },

                    // 2. Logout
                    gtk4::Button {
                        add_css_class: "power-card",
                        connect_clicked => PowerAction::Logout,

                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,
                            set_halign: Align::Center,
                            set_valign: Align::Center,

                            gtk4::Label {
                                add_css_class: "power-card-ico",
                                set_label: "󰍃",
                            },
                            gtk4::Label {
                                add_css_class: "power-card-label",
                                set_label: &tr("Logout", "Выход"),
                            },
                            gtk4::Label {
                                add_css_class: "power-card-key",
                                set_label: "2",
                                set_halign: Align::Center,
                                set_valign: Align::Center,
                            },
                        },
                    },

                    // 3. Suspend
                    gtk4::Button {
                        add_css_class: "power-card",
                        connect_clicked => PowerAction::Suspend,

                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,
                            set_halign: Align::Center,
                            set_valign: Align::Center,

                            gtk4::Label {
                                add_css_class: "power-card-ico",
                                set_label: "󰤄",
                            },
                            gtk4::Label {
                                add_css_class: "power-card-label",
                                set_label: &tr("Suspend", "Сон"),
                            },
                            gtk4::Label {
                                add_css_class: "power-card-key",
                                set_label: "3",
                                set_halign: Align::Center,
                                set_valign: Align::Center,
                            },
                        },
                    },

                    // 4. Reboot
                    gtk4::Button {
                        add_css_class: "power-card",
                        connect_clicked => PowerAction::Reboot,

                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,
                            set_halign: Align::Center,
                            set_valign: Align::Center,

                            gtk4::Label {
                                add_css_class: "power-card-ico",
                                set_label: "󰜉",
                            },
                            gtk4::Label {
                                add_css_class: "power-card-label",
                                set_label: &tr("Reboot", "Перезагрузка"),
                            },
                            gtk4::Label {
                                add_css_class: "power-card-key",
                                set_label: "4",
                                set_halign: Align::Center,
                                set_valign: Align::Center,
                            },
                        },
                    },

                    // 5. Shutdown
                    gtk4::Button {
                        add_css_class: "power-card",
                        add_css_class: "danger",
                        connect_clicked => PowerAction::Poweroff,

                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 6,
                            set_halign: Align::Center,
                            set_valign: Align::Center,

                            gtk4::Label {
                                add_css_class: "power-card-ico",
                                set_label: "⏻",
                            },
                            gtk4::Label {
                                add_css_class: "power-card-label",
                                set_label: &tr("Power Off", "Выключение"),
                            },
                            gtk4::Label {
                                add_css_class: "power-card-key",
                                set_label: "5",
                                set_halign: Align::Center,
                                set_valign: Align::Center,
                            },
                        },
                    },
                },

                // Cancel button
                gtk4::Button {
                    add_css_class: "cc-chip",
                    add_css_class: "power-cancel-btn",
                    set_label: &tr("Cancel (Esc)", "Отмена (Esc)"),
                    set_halign: Align::Center,
                    connect_clicked => PowerAction::Cancel,
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PowerMenuModel { close_cb: init };
        let widgets = view_output!();

        // Keyboard controller for keys 1..5 and Esc
        let key_sender = sender.clone();
        let key_ctrl = EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            let action = match key {
                Key::_1 | Key::KP_1 => Some(PowerAction::Lock),
                Key::_2 | Key::KP_2 => Some(PowerAction::Logout),
                Key::_3 | Key::KP_3 => Some(PowerAction::Suspend),
                Key::_4 | Key::KP_4 => Some(PowerAction::Reboot),
                Key::_5 | Key::KP_5 => Some(PowerAction::Poweroff),
                Key::Escape => Some(PowerAction::Cancel),
                _ => match key.to_unicode() {
                    Some('1') => Some(PowerAction::Lock),
                    Some('2') => Some(PowerAction::Logout),
                    Some('3') => Some(PowerAction::Suspend),
                    Some('4') => Some(PowerAction::Reboot),
                    Some('5') => Some(PowerAction::Poweroff),
                    _ => None,
                },
            };
            if let Some(act) = action {
                key_sender.input(act);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        root.add_controller(key_ctrl);

        // Click outside modal
        let modal_weak = widgets.modal.downgrade();
        let click_sender = sender;
        let click_gesture = GestureClick::new();
        click_gesture.connect_released(move |gesture, _, x, y| {
            if let Some(target) = gesture.widget()
                && let Some(m) = modal_weak.upgrade()
                && let Some(rect) = m.compute_bounds(&target)
            {
                let rx = rect.x() as f64;
                let ry = rect.y() as f64;
                let rw = rect.width() as f64;
                let rh = rect.height() as f64;
                if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                    return;
                }
            }
            click_sender.input(PowerAction::Cancel);
        });
        root.add_controller(click_gesture);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        (self.close_cb)();
        match msg {
            PowerAction::Lock => {
                thread::spawn(|| {
                    let _ = Command::new("sh")
                        .args(["-c", "hyprlock || loginctl lock-session"])
                        .output();
                });
            }
            PowerAction::Logout => {
                thread::spawn(|| {
                    let _ = Command::new("sh")
                        .args([
                            "-c",
                            "if uwsm check is-active >/dev/null 2>&1; then uwsm stop; else env -u HYPRLAND_INSTANCE_SIGNATURE hyprctl dispatch exit 2>/dev/null || hyprctl dispatch exit 2>/dev/null || loginctl terminate-session \"$XDG_SESSION_ID\"; fi",
                        ])
                        .output();
                });
            }
            PowerAction::Suspend => {
                thread::spawn(|| {
                    let _ = Command::new("systemctl").arg("suspend").output();
                });
            }
            PowerAction::Reboot => {
                thread::spawn(|| {
                    let _ = Command::new("systemctl").arg("reboot").output();
                });
            }
            PowerAction::Poweroff => {
                thread::spawn(|| {
                    let _ = Command::new("systemctl").arg("poweroff").output();
                });
            }
            PowerAction::Cancel => {}
        }
    }
}

pub fn build_power_menu<FClose>(close_cb: FClose) -> (gtk4::Box, i32, i32)
where
    FClose: Fn() + 'static + Clone,
{
    let mut controller = PowerMenuModel::builder()
        .launch(Box::new(close_cb))
        .detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 840, 320)
}
