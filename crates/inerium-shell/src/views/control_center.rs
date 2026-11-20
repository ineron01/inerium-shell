use std::cell::RefCell;
use std::process::Command;
use std::thread;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType};
use inerium_i18n::{tr, tr_fmt, tr_plural};
use relm4::prelude::*;

use crate::hw;

thread_local! {
    static DDC_SENDER: RefCell<Option<ComponentSender<ControlCenterModel>>> = const { RefCell::new(None) };
}

pub fn on_ddc_updated(val: u32) {
    DDC_SENDER.with(|s| {
        if let Some(ref sender) = *s.borrow() {
            sender.input(ControlCenterMsg::DdcUpdated(val));
        }
    });
}

fn get_wifi_sub(wifi_on: bool) -> String {
    if wifi_on {
        if hw::is_hotspot_active() {
            tr("Hotspot", "Точка доступа").to_string()
        } else if let Some(ref ssid) = hw::get_active_wifi_ssid() {
            ssid.clone()
        } else {
            tr("Connected", "Включен").to_string()
        }
    } else {
        tr("Disabled", "Выключен").to_string()
    }
}

fn get_theme_info() -> (bool, String, String, usize) {
    let home = hw::home_dir();

    let mut theme_ids = std::collections::HashSet::new();
    let user_themes = home.join(".config/inerium/themes");
    if let Ok(entries) = std::fs::read_dir(&user_themes) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                theme_ids.insert(stem.to_string());
            }
        }
    }
    let count = theme_ids.len().max(2);

    let cur_json = home.join(".config/inerium/current.json");
    if let Ok(content) = std::fs::read_to_string(&cur_json)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
    {
        let variant = val.get("variant").and_then(|v| v.as_str()).unwrap_or("dark");
        let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("Kanagawa Dragon");
        let author = val.get("author").and_then(|v| v.as_str()).unwrap_or("rebelot");
        return (variant == "dark", name.to_string(), author.to_string(), count);
    }
    let is_dark = crate::theme::detect_theme() == crate::theme::ThemeMode::Dark;
    if is_dark {
        (true, "Kanagawa Dragon".to_string(), "rebelot".to_string(), count)
    } else {
        (false, "Inerium Light".to_string(), "Inerium Core".to_string(), count)
    }
}

fn compute_fan_string(is_cb: bool) -> String {
    let (cpu_rpm, gpu_rpm, cpu_pct, gpu_pct) = hw::get_fan_telemetry();
    let c_pct = cpu_pct.unwrap_or(0);
    let g_pct = gpu_pct.unwrap_or(0);

    if is_cb {
        match (cpu_rpm, gpu_rpm) {
            (Some(cr), Some(gr)) if cr > 0 && gr > 0 => {
                format!("󰈐 Cooler Boost: CPU {} RPM  ·  GPU {} RPM", cr, gr)
            }
            _ => tr(
                "󰈐 Cooler Boost: maximum speed (100%)",
                "󰈐 Cooler Boost: максимальные обороты (100%)",
            )
            .to_string(),
        }
    } else {
        let mut parts = Vec::new();
        if let Some(cr) = cpu_rpm {
            if cr > 0 {
                if cpu_pct.is_some() {
                    parts.push(format!("CPU {}% ({} RPM)", c_pct, cr));
                } else {
                    parts.push(format!("Fan 1: {} RPM", cr));
                }
            } else if cpu_pct.is_some() {
                parts.push(format!("CPU {}%", c_pct));
            }
        } else if cpu_pct.is_some() {
            parts.push(format!("CPU {}%", c_pct));
        }

        if let Some(gr) = gpu_rpm {
            if gr > 0 {
                if gpu_pct.is_some() {
                    parts.push(format!("GPU {}% ({} RPM)", g_pct, gr));
                } else {
                    parts.push(format!("Fan 2: {} RPM", gr));
                }
            } else if gpu_pct.is_some() {
                parts.push(format!("GPU {}%", g_pct));
            }
        } else if gpu_pct.is_some() {
            parts.push(format!("GPU {}%", g_pct));
        }

        if parts.is_empty() {
            tr("󰈨  Fans: active", "󰈨  Кулеры: активный режим").to_string()
        } else {
            format!("󰈨  {}: {}", tr("Fans", "Кулеры"), parts.join(" · "))
        }
    }
}

pub struct ControlCenterModel {
    pub hostname: String,
    pub battery_line: String,
    pub uptime_line: String,
    pub updates: hw::SystemUpdates,
    pub is_updating_check: bool,

    // Toggles
    pub is_wifi: bool,
    pub wifi_sub: String,
    pub is_bt: bool,
    pub is_dnd: bool,
    pub is_mic_muted: bool,
    pub is_idle_active: bool,
    pub is_dark_theme: bool,
    pub theme_name: String,
    pub theme_author: String,
    pub available_themes_count: usize,

    // Sliders
    pub volume: u32,
    pub volume_muted: bool,
    pub has_backlight: bool,
    pub laptop_brightness: u32,
    pub has_external_monitor: bool,
    pub ddc_brightness: u32,

    // Power & Cooling
    pub has_ppd: bool,
    pub current_profile: String,
    pub has_fan_telem: bool,
    pub fan_str: String,
    pub has_ec: bool,
    pub cooler_boost: bool,
    pub has_bat_thresh: bool,
    pub battery_threshold: u32,

    // Graphics
    pub has_envycontrol: bool,
    pub gpu_mode: String,
    pub gpu_switching: bool,
    pub gpu_status_text: String,
    pub gpu_status_ok: bool,
    pub show_reboot: bool,

    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum ControlCenterMsg {
    // Header
    CheckUpdates,
    UpdatesChecked(hw::SystemUpdates),
    LaunchUpdateTerminal,
    ToggleHypridle,
    ToggleTheme,
    LockScreen,
    PowerMenu,
    NavAppearance,

    // Quick Tiles
    ToggleWifi,
    ToggleBt,
    ToggleDnd,
    ToggleMic,
    NavNetwork,
    NavBluetooth,
    NavAudio,

    // Sliders
    ToggleVolumeMute,
    SetVolume(u32),
    SetLaptopBrightness(u32),
    SetDdcBrightness(u32),
    DdcUpdated(u32),

    // Power & Cooling
    SetProfile(String),
    RefreshFanTelemetry,
    ToggleCoolerBoost,
    ToggleBatteryThreshold,

    // GPU
    SwitchGpuMode(&'static str),
    GpuSwitchFinished(bool, &'static str),
    Reboot,
}

pub struct ControlCenterInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for ControlCenterModel {
    type Input = ControlCenterMsg;
    type Output = ();
    type Init = ControlCenterInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "cc",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (384, -1),

            // 1. Header (Hostname, battery/uptime/updates, icon buttons)
            gtk4::Box {
                add_css_class: "cc-head",
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 2,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_xalign: 0.0,
                        set_label: &model.hostname,
                    },

                    gtk4::Box {
                        set_orientation: Orientation::Horizontal,
                        set_spacing: 8,

                        gtk4::Label {
                            add_css_class: "cc-meta-accent",
                            set_xalign: 0.0,
                            #[watch]
                            set_label: &model.battery_line,
                        },

                        gtk4::Label {
                            add_css_class: "cc-meta",
                            set_xalign: 0.0,
                            #[watch]
                            set_label: &model.uptime_line,
                        },

                        #[name = "upd_btn"]
                        gtk4::Button {
                            add_css_class: "cc-meta-btn",
                            set_valign: Align::Center,
                            #[watch]
                            set_css_classes: if model.updates.total > 0 {
                                &["cc-meta-btn", "has-updates"]
                            } else {
                                &["cc-meta-btn"]
                            },
                            #[watch]
                            set_tooltip_text: Some(&{
                                if model.updates.total > 0 {
                                    let pkg_word = tr_plural(
                                        model.updates.total,
                                        "package",
                                        "packages",
                                        "пакет",
                                        "пакета",
                                        "пакетов",
                                    );
                                    let mut tt = tr_fmt!(
                                        "Ready to update: {total} {pkg_word}\n• Repositories: {repo}\n• AUR: {aur}",
                                        "Готово к обновлению: {total} {pkg_word}\n• Репозитории: {repo}\n• AUR: {aur}",
                                        total = model.updates.total,
                                        pkg_word = pkg_word,
                                        repo = model.updates.repo,
                                        aur = model.updates.aur,
                                    );
                                    if !model.updates.sample_packages.is_empty() {
                                        tt.push_str(tr("\n\nPackages:\n  ", "\n\nПакеты:\n  "));
                                        tt.push_str(&model.updates.sample_packages.join(", "));
                                    }
                                    tt.push_str(tr(
                                        "\n\nLMB: Update system (paru)\nRMB: Recheck",
                                        "\n\nЛКМ: Обновить систему (paru)\nПКМ: Перепроверить",
                                    ));
                                    tt
                                } else {
                                    tr(
                                        "All packages up to date\nLMB: Update (paru)\nRMB: Recheck",
                                        "Все пакеты обновлены\nЛКМ: Обновить (paru)\nПКМ: Перепроверить",
                                    ).to_string()
                                }
                            }),

                            gtk4::Label {
                                add_css_class: "cc-meta-btn-lbl",
                                set_xalign: 0.0,
                                #[watch]
                                set_label: &if model.is_updating_check {
                                    "󰑐 ...".to_string()
                                } else {
                                    format!("󰚰  {}", model.updates.total)
                                },
                            },
                        },
                    },
                },

                #[name = "appearance_btn"]
                gtk4::Button {
                    add_css_class: "cc-icon-btn",
                    set_valign: Align::Center,
                    set_label: "󰏘",
                    #[watch]
                    set_tooltip_text: Some(&format!(
                        "{}: {} — {}",
                        tr("Appearance", "Оформление"),
                        model.theme_name,
                        tr("LMB: themes & wallpapers, RMB: toggle theme", "ЛКМ: темы и обои, ПКМ: быстрая смена")
                    )),
                    connect_clicked => ControlCenterMsg::NavAppearance,
                },

                #[name = "idle_btn"]
                gtk4::Button {
                    add_css_class: "cc-icon-btn",
                    set_valign: Align::Center,
                    #[watch]
                    set_css_classes: if model.is_idle_active {
                        &["cc-icon-btn"]
                    } else {
                        &["cc-icon-btn", "amber", "on"]
                    },
                    #[watch]
                    set_label: if model.is_idle_active { "󰒳" } else { "󰈈" },
                    #[watch]
                    set_tooltip_text: Some(if model.is_idle_active {
                        tr("Autolock: On", "Автоблокировка: вкл")
                    } else {
                        tr("Autolock: Off", "Автоблокировка: выкл")
                    }),
                    connect_clicked => ControlCenterMsg::ToggleHypridle,
                },

                gtk4::Button {
                    add_css_class: "cc-icon-btn",
                    set_valign: Align::Center,
                    set_label: "󰌾",
                    set_tooltip_text: Some(tr("Lock", "Заблокировать")),
                    connect_clicked => ControlCenterMsg::LockScreen,
                },

                gtk4::Button {
                    add_css_class: "cc-icon-btn",
                    add_css_class: "danger",
                    set_valign: Align::Center,
                    set_label: "⏻",
                    set_tooltip_text: Some(tr("Power menu", "Меню питания")),
                    connect_clicked => ControlCenterMsg::PowerMenu,
                },
            },

            // Scrolled Body
            gtk4::ScrolledWindow {
                set_hscrollbar_policy: PolicyType::Never,
                set_vscrollbar_policy: PolicyType::Automatic,
                set_propagate_natural_width: true,
                set_vexpand: true,
                set_hexpand: true,
                set_min_content_height: 480,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 8,
                    set_vexpand: true,
                    set_hexpand: true,

                    // 2. 2x2 Quick Tiles (Two homogeneous rows)
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 8,

                        // Row 1: Wi-Fi & Bluetooth
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,
                            set_homogeneous: true,

                            // Tile 1: Wi-Fi
                            #[name = "wifi_tile"]
                            gtk4::Box {
                                add_css_class: "cc-tile",
                                set_orientation: Orientation::Horizontal,
                                #[watch]
                                set_css_classes: if model.is_wifi {
                                    &["cc-tile", "on"]
                                } else {
                                    &["cc-tile"]
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-main",
                                    set_hexpand: true,
                                    connect_clicked => ControlCenterMsg::ToggleWifi,

                                    gtk4::Box {
                                        set_orientation: Orientation::Horizontal,
                                        set_spacing: 9,

                                        gtk4::Label {
                                            add_css_class: "cc-tile-ico",
                                            set_xalign: 0.5,
                                            set_label: "󰤨",
                                        },

                                        gtk4::Box {
                                            set_orientation: Orientation::Vertical,
                                            set_spacing: 1,
                                            set_valign: Align::Center,

                                            gtk4::Label {
                                                add_css_class: "cc-tile-title",
                                                set_xalign: 0.0,
                                                set_label: "Wi-Fi",
                                            },

                                            gtk4::Label {
                                                add_css_class: "cc-tile-sub",
                                                set_xalign: 0.0,
                                                set_ellipsize: gtk4::pango::EllipsizeMode::End,
                                                set_max_width_chars: 14,
                                                #[watch]
                                                set_label: &model.wifi_sub,
                                            },
                                        },
                                    },
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-more",
                                    set_label: "›",
                                    set_tooltip_text: Some(tr("Details", "Подробнее")),
                                    connect_clicked => ControlCenterMsg::NavNetwork,
                                },
                            },

                            // Tile 2: Bluetooth
                            #[name = "bt_tile"]
                            gtk4::Box {
                                add_css_class: "cc-tile",
                                add_css_class: "blue",
                                set_orientation: Orientation::Horizontal,
                                #[watch]
                                set_css_classes: if model.is_bt {
                                    &["cc-tile", "blue", "on"]
                                } else {
                                    &["cc-tile", "blue"]
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-main",
                                    set_hexpand: true,
                                    connect_clicked => ControlCenterMsg::ToggleBt,

                                    gtk4::Box {
                                        set_orientation: Orientation::Horizontal,
                                        set_spacing: 9,

                                        gtk4::Label {
                                            add_css_class: "cc-tile-ico",
                                            set_xalign: 0.5,
                                            set_label: "󰂯",
                                        },

                                        gtk4::Box {
                                            set_orientation: Orientation::Vertical,
                                            set_spacing: 1,
                                            set_valign: Align::Center,

                                            gtk4::Label {
                                                add_css_class: "cc-tile-title",
                                                set_xalign: 0.0,
                                                set_label: "Bluetooth",
                                            },

                                            gtk4::Label {
                                                add_css_class: "cc-tile-sub",
                                                set_xalign: 0.0,
                                                set_ellipsize: gtk4::pango::EllipsizeMode::End,
                                                set_max_width_chars: 14,
                                                #[watch]
                                                set_label: if model.is_bt {
                                                    tr("Active", "Активен")
                                                } else {
                                                    tr("Disabled", "Выключен")
                                                },
                                            },
                                        },
                                    },
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-more",
                                    set_label: "›",
                                    set_tooltip_text: Some(tr("Details", "Подробнее")),
                                    connect_clicked => ControlCenterMsg::NavBluetooth,
                                },
                            },
                        },

                        // Row 2: DND & Mic
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,
                            set_homogeneous: true,

                            // Tile 3: DND
                            #[name = "dnd_tile"]
                            gtk4::Box {
                                add_css_class: "cc-tile",
                                add_css_class: "amber",
                                set_orientation: Orientation::Horizontal,
                                #[watch]
                                set_css_classes: if model.is_dnd {
                                    &["cc-tile", "amber", "on"]
                                } else {
                                    &["cc-tile", "amber"]
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-main",
                                    set_hexpand: true,
                                    connect_clicked => ControlCenterMsg::ToggleDnd,

                                    gtk4::Box {
                                        set_orientation: Orientation::Horizontal,
                                        set_spacing: 9,

                                        gtk4::Label {
                                            add_css_class: "cc-tile-ico",
                                            set_xalign: 0.5,
                                            set_label: "󰂛",
                                        },

                                        gtk4::Box {
                                            set_orientation: Orientation::Vertical,
                                            set_spacing: 1,
                                            set_valign: Align::Center,

                                            gtk4::Label {
                                                add_css_class: "cc-tile-title",
                                                set_xalign: 0.0,
                                                set_label: tr("Do Not Disturb", "Не беспокоить"),
                                            },

                                            gtk4::Label {
                                                add_css_class: "cc-tile-sub",
                                                set_xalign: 0.0,
                                                set_ellipsize: gtk4::pango::EllipsizeMode::End,
                                                set_max_width_chars: 14,
                                                #[watch]
                                                set_label: if model.is_dnd {
                                                    tr("Notifications hidden", "Уведомления скрыты")
                                                } else {
                                                    tr("Off", "Выключено")
                                                },
                                            },
                                        },
                                    },
                                },
                            },

                            // Tile 4: Mic
                            #[name = "mic_tile"]
                            gtk4::Box {
                                add_css_class: "cc-tile",
                                add_css_class: "red",
                                set_orientation: Orientation::Horizontal,
                                #[watch]
                                set_css_classes: if model.is_mic_muted {
                                    &["cc-tile", "red", "on"]
                                } else {
                                    &["cc-tile", "red"]
                                },

                                gtk4::Button {
                                    add_css_class: "cc-tile-main",
                                    set_hexpand: true,
                                    connect_clicked => ControlCenterMsg::ToggleMic,

                                    gtk4::Box {
                                        set_orientation: Orientation::Horizontal,
                                        set_spacing: 9,

                                        gtk4::Label {
                                            add_css_class: "cc-tile-ico",
                                            set_xalign: 0.5,
                                            #[watch]
                                            set_label: if model.is_mic_muted { "󰍭" } else { "󰍬" },
                                        },

                                        gtk4::Box {
                                            set_orientation: Orientation::Vertical,
                                            set_spacing: 1,
                                            set_valign: Align::Center,

                                            gtk4::Label {
                                                add_css_class: "cc-tile-title",
                                                set_xalign: 0.0,
                                                set_label: tr("Microphone", "Микрофон"),
                                            },

                                            gtk4::Label {
                                                add_css_class: "cc-tile-sub",
                                                set_xalign: 0.0,
                                                set_ellipsize: gtk4::pango::EllipsizeMode::End,
                                                set_max_width_chars: 14,
                                                #[watch]
                                                set_label: if model.is_mic_muted {
                                                    tr("Muted", "Выключен")
                                                } else {
                                                    tr("Active", "Включен")
                                                },
                                            },
                                        },
                                    },
                                },
                            },
                        },
                    },

                    // 3. Sliders Card
                    gtk4::Box {
                        add_css_class: "cc-card",
                        set_orientation: Orientation::Vertical,
                        set_spacing: 4,

                        // Volume Slider
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 6,

                            gtk4::Button {
                                add_css_class: "cc-slider-ico",
                                set_tooltip_text: Some("Mute"),
                                #[watch]
                                set_label: if model.volume_muted { "󰝟" } else { "󰕾" },
                                connect_clicked => ControlCenterMsg::ToggleVolumeMute,
                            },

                            #[name = "vol_sc"]
                            gtk4::Scale {
                                add_css_class: "cc-scale",
                                set_orientation: Orientation::Horizontal,
                                set_range: (0.0, 150.0),
                                set_hexpand: true,
                                set_value: model.volume as f64,
                                connect_value_changed[sender] => move |sc| {
                                    sender.input(ControlCenterMsg::SetVolume(sc.value() as u32));
                                },
                            },

                            gtk4::Label {
                                add_css_class: "cc-pct",
                                set_xalign: 1.0,
                                #[watch]
                                set_label: &format!("{}%", model.volume),
                            },

                            gtk4::Button {
                                add_css_class: "cc-slider-ico",
                                set_label: "›",
                                set_tooltip_text: Some(tr("Audio Devices", "Аудиоустройства")),
                                connect_clicked => ControlCenterMsg::NavAudio,
                            },
                        },

                        // Laptop Brightness Slider
                        #[name = "laptop_br_box"]
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 6,
                            #[watch]
                            set_visible: model.has_backlight,

                            gtk4::Button {
                                add_css_class: "cc-slider-ico",
                                set_label: "󰖨",
                                set_tooltip_text: Some(tr(
                                    "Laptop display brightness",
                                    "Яркость экрана ноутбука",
                                )),
                            },

                            #[name = "br_sc"]
                            gtk4::Scale {
                                add_css_class: "cc-scale",
                                set_orientation: Orientation::Horizontal,
                                set_range: (5.0, 100.0),
                                set_hexpand: true,
                                set_value: model.laptop_brightness as f64,
                                connect_value_changed[sender] => move |sc| {
                                    sender.input(ControlCenterMsg::SetLaptopBrightness(sc.value() as u32));
                                },
                            },

                            gtk4::Label {
                                add_css_class: "cc-pct",
                                set_xalign: 1.0,
                                #[watch]
                                set_label: &format!("{}%", model.laptop_brightness),
                            },

                            gtk4::Box {
                                set_size_request: (28, 28),
                            },
                        },

                        // External DDC Brightness Slider
                        #[name = "ext_br_box"]
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 6,
                            #[watch]
                            set_visible: model.has_external_monitor,

                            gtk4::Button {
                                add_css_class: "cc-slider-ico",
                                set_label: "󰍹",
                                set_tooltip_text: Some(tr(
                                    "External monitor brightness (DDC)",
                                    "Яркость внешнего монитора (DDC)",
                                )),
                            },

                            #[name = "ext_sc"]
                            gtk4::Scale {
                                add_css_class: "cc-scale",
                                add_css_class: "amber",
                                set_orientation: Orientation::Horizontal,
                                set_range: (0.0, 100.0),
                                set_hexpand: true,
                                set_value: model.ddc_brightness as f64,
                                connect_value_changed[sender] => move |sc| {
                                    sender.input(ControlCenterMsg::SetDdcBrightness(sc.value() as u32));
                                },
                            },

                            #[name = "ext_pct_lbl"]
                            gtk4::Label {
                                add_css_class: "cc-pct",
                                set_xalign: 1.0,
                                #[watch]
                                set_label: &format!("{}%", model.ddc_brightness),
                            },

                            gtk4::Box {
                                set_size_request: (28, 28),
                            },
                        },
                    },

                    // 4. Power & Cooling
                    gtk4::Label {
                        add_css_class: "cc-label",
                        set_xalign: 0.0,
                        #[watch]
                        set_visible: model.has_ppd || model.has_fan_telem || model.has_bat_thresh,
                        #[watch]
                        set_label: if model.has_fan_telem {
                            tr("POWER & COOLING", "ПИТАНИЕ И ОХЛАЖДЕНИЕ")
                        } else {
                            tr("POWER", "ПИТАНИЕ")
                        },
                    },

                    // Power Profiles Segment Bar
                    #[name = "ppd_seg"]
                    gtk4::Box {
                        add_css_class: "cc-seg",
                        set_orientation: Orientation::Horizontal,
                        set_spacing: 2,
                        set_homogeneous: true,
                        #[watch]
                        set_visible: model.has_ppd,

                        #[name = "eco_btn"]
                        gtk4::Button {
                            add_css_class: "eco",
                            #[watch]
                            set_css_classes: if model.current_profile == "power-saver" {
                                &["eco", "active"]
                            } else {
                                &["eco"]
                            },
                            set_label: &format!("󰌪  {}", tr("Eco", "Эконом")),
                            connect_clicked[sender] => move |_| {
                                sender.input(ControlCenterMsg::SetProfile("power-saver".to_string()));
                            },
                        },

                        #[name = "bal_btn"]
                        gtk4::Button {
                            #[watch]
                            set_css_classes: if model.current_profile == "balanced" {
                                &["active"]
                            } else {
                                &[]
                            },
                            set_label: &format!("󰾅  {}", tr("Balance", "Баланс")),
                            connect_clicked[sender] => move |_| {
                                sender.input(ControlCenterMsg::SetProfile("balanced".to_string()));
                            },
                        },

                        #[name = "turbo_btn"]
                        gtk4::Button {
                            add_css_class: "turbo",
                            #[watch]
                            set_css_classes: if model.current_profile == "performance" {
                                &["turbo", "active"]
                            } else {
                                &["turbo"]
                            },
                            set_label: &format!("󰓅  {}", tr("Turbo", "Турбо")),
                            connect_clicked[sender] => move |_| {
                                sender.input(ControlCenterMsg::SetProfile("performance".to_string()));
                            },
                        },
                    },

                    // Fan Telemetry Label
                    #[name = "fan_lbl"]
                    gtk4::Label {
                        add_css_class: "cc-meta",
                        set_xalign: 0.5,
                        set_margin_top: 2,
                        set_margin_bottom: 2,
                        #[watch]
                        set_visible: model.has_fan_telem,
                        #[watch]
                        set_label: &model.fan_str,
                    },

                    // Cooler Boost & Battery Threshold Chips
                    gtk4::Box {
                        set_orientation: Orientation::Horizontal,
                        set_spacing: 8,
                        set_homogeneous: true,
                        #[watch]
                        set_visible: model.has_ec || model.has_bat_thresh,

                        #[name = "cb_chip"]
                        gtk4::Button {
                            add_css_class: "cc-chip",
                            add_css_class: "red",
                            #[watch]
                            set_visible: model.has_ec,
                            #[watch]
                            set_css_classes: if model.cooler_boost {
                                &["cc-chip", "red", "on"]
                            } else {
                                &["cc-chip", "red"]
                            },
                            #[watch]
                            set_label: &format!(
                                "󰈐  {}: {}",
                                tr("Cooler Boost", "Cooler Boost"),
                                if model.cooler_boost { tr("on", "вкл") } else { tr("off", "выкл") }
                            ),
                            connect_clicked => ControlCenterMsg::ToggleCoolerBoost,
                        },

                        #[name = "bat_chip"]
                        gtk4::Button {
                            add_css_class: "cc-chip",
                            #[watch]
                            set_visible: model.has_bat_thresh,
                            #[watch]
                            set_css_classes: if model.battery_threshold < 100 {
                                &["cc-chip", "on"]
                            } else {
                                &["cc-chip"]
                            },
                            #[watch]
                            set_label: &if model.battery_threshold < 100 {
                                format!("󱈑  {}: {}%", tr("Battery Limit", "Лимит заряда"), model.battery_threshold)
                            } else {
                                format!("󱈑  {}", tr("Charge to 100%", "Заряд до 100%"))
                            },
                            connect_clicked => ControlCenterMsg::ToggleBatteryThreshold,
                        },
                    },

                    // 5. Graphics (EnvyControl)
                    #[name = "gpu_sec"]
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 6,
                        #[watch]
                        set_visible: model.has_envycontrol,

                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Box {
                                set_orientation: Orientation::Vertical,
                                set_hexpand: true,
                                set_valign: Align::Center,

                                gtk4::Label {
                                    add_css_class: "cc-label",
                                    set_xalign: 0.0,
                                    set_label: tr("GRAPHICS", "ГРАФИКА"),
                                },

                                gtk4::Label {
                                    add_css_class: "cc-foot",
                                    set_xalign: 0.0,
                                    #[watch]
                                    set_label: if model.gpu_mode == "integrated" {
                                        tr("Integrated graphics only (iGPU)", "Только встроенная графика (iGPU)")
                                    } else {
                                        tr("Hybrid graphics (iGPU + dGPU)", "Гибридная графика (iGPU + dGPU)")
                                    },
                                },
                            },

                            gtk4::Box {
                                add_css_class: "cc-seg",
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 2,

                                gtk4::Button {
                                    #[watch]
                                    set_sensitive: !model.gpu_switching,
                                    #[watch]
                                    set_css_classes: if model.gpu_mode == "integrated" {
                                        &["active"]
                                    } else {
                                        &[]
                                    },
                                    set_label: "󰢮  iGPU",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(ControlCenterMsg::SwitchGpuMode("integrated"));
                                    },
                                },

                                gtk4::Button {
                                    #[watch]
                                    set_sensitive: !model.gpu_switching,
                                    #[watch]
                                    set_css_classes: if model.gpu_mode != "integrated" {
                                        &["active"]
                                    } else {
                                        &[]
                                    },
                                    set_label: "󰾲  Hybrid",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(ControlCenterMsg::SwitchGpuMode("hybrid"));
                                    },
                                },
                            },
                        },

                        gtk4::Label {
                            add_css_class: "cc-status",
                            set_xalign: 0.0,
                            set_wrap: true,
                            #[watch]
                            set_visible: !model.gpu_status_text.is_empty(),
                            #[watch]
                            set_css_classes: if model.gpu_status_ok {
                                &["cc-status", "ok"]
                            } else {
                                &["cc-status", "err"]
                            },
                            #[watch]
                            set_label: &model.gpu_status_text,
                        },

                        gtk4::Button {
                            add_css_class: "cc-chip",
                            add_css_class: "on",
                            set_label: tr("󰜉  Reboot now", "󰜉  Перезагрузить сейчас"),
                            #[watch]
                            set_visible: model.show_reboot,
                            connect_clicked => ControlCenterMsg::Reboot,
                        },
                    },
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let hostname = hw::get_hostname();
        let battery_line = hw::get_battery_line();
        let uptime_line = hw::get_uptime_line();
        let updates = hw::get_updates_info();

        let is_wifi = hw::is_wifi_enabled();
        let wifi_sub = get_wifi_sub(is_wifi);
        let is_bt = hw::is_bluetooth_enabled();
        let is_dnd = hw::is_dnd_active();
        let is_mic_muted = hw::is_mic_muted();
        let is_idle_active = hw::is_hypridle_active();
        let (is_dark_theme, theme_name, theme_author, available_themes_count) = get_theme_info();

        let volume = hw::get_volume();
        let volume_muted = hw::is_volume_muted();
        let has_backlight = hw::has_backlight();
        let laptop_brightness = if has_backlight {
            hw::get_laptop_brightness()
        } else {
            100
        };
        let has_external_monitor = hw::has_external_monitor();
        let ddc_brightness = if has_external_monitor {
            hw::get_cached_ddc_brightness()
        } else {
            0
        };

        let has_ppd = hw::has_power_profiles();
        let current_profile = if has_ppd {
            hw::get_power_profile()
        } else {
            String::new()
        };

        let has_fan_telem = hw::has_fan_telemetry();
        let has_ec = hw::has_msi_ec();
        let cooler_boost = if has_ec {
            hw::get_cooler_boost()
        } else {
            false
        };
        let fan_str = if has_fan_telem {
            compute_fan_string(cooler_boost)
        } else {
            String::new()
        };

        let has_bat_thresh = hw::has_battery_threshold();
        let battery_threshold = if has_bat_thresh {
            hw::get_battery_threshold()
        } else {
            100
        };

        let has_envycontrol = hw::has_envycontrol();
        let gpu_mode = if has_envycontrol {
            hw::get_gpu_mode()
        } else {
            String::new()
        };

        let model = ControlCenterModel {
            hostname,
            battery_line,
            uptime_line,
            updates,
            is_updating_check: false,

            is_wifi,
            wifi_sub,
            is_bt,
            is_dnd,
            is_mic_muted,
            is_idle_active,
            is_dark_theme,
            theme_name,
            theme_author,
            available_themes_count,

            volume,
            volume_muted,
            has_backlight,
            laptop_brightness,
            has_external_monitor,
            ddc_brightness,

            has_ppd,
            current_profile,
            has_fan_telem,
            fan_str,
            has_ec,
            cooler_boost,
            has_bat_thresh,
            battery_threshold,

            has_envycontrol,
            gpu_mode,
            gpu_switching: false,
            gpu_status_text: String::new(),
            gpu_status_ok: false,
            show_reboot: false,

            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        // Register DDC sender in thread-local for hw/backlight.rs:on_ddc_updated
        DDC_SENDER.with(|s| {
            *s.borrow_mut() = Some(sender.clone());
        });

        // Trigger asynchronous DDC hardware refresh if external monitor exists
        if has_external_monitor {
            hw::refresh_ddc_brightness_async();
        }

        let widgets = view_output!();

        // Updates button LMB / RMB gesture
        let s_upd = sender.clone();
        let click_gesture = gtk4::GestureClick::new();
        click_gesture.set_button(0);
        click_gesture.connect_pressed(move |gesture, _, _, _| {
            let btn = gesture.current_button();
            if btn == 1 {
                s_upd.input(ControlCenterMsg::LaunchUpdateTerminal);
            } else if btn == 3 {
                s_upd.input(ControlCenterMsg::CheckUpdates);
            }
        });
        widgets.upd_btn.add_controller(click_gesture);

        // Appearance button RMB gesture for quick theme toggle
        let s_app = sender.clone();
        let app_gesture = gtk4::GestureClick::new();
        app_gesture.set_button(3);
        app_gesture.connect_pressed(move |_, _, _, _| {
            s_app.input(ControlCenterMsg::ToggleTheme);
        });
        widgets.appearance_btn.add_controller(app_gesture);

        // Fan telemetry 1-second timer while root widget is mapped
        if has_fan_telem {
            let root_ref = root.clone();
            let s_fan = sender.clone();
            glib::timeout_add_local(Duration::from_millis(1000), move || {
                if !root_ref.is_mapped() {
                    return glib::ControlFlow::Break;
                }
                s_fan.input(ControlCenterMsg::RefreshFanTelemetry);
                glib::ControlFlow::Continue
            });
        }

        ComponentParts { model, widgets }
    }

    fn post_view() {
        if self.has_external_monitor {
            let cur = ext_sc.value() as u32;
            if cur != self.ddc_brightness {
                ext_sc.set_value(self.ddc_brightness as f64);
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ControlCenterMsg::CheckUpdates => {
                self.is_updating_check = true;
                let s = sender.clone();
                thread::spawn(move || {
                    let _ = Command::new("inerium-updates").arg("--check").output();
                    let fresh = hw::get_updates_info();
                    s.input(ControlCenterMsg::UpdatesChecked(fresh));
                });
            }
            ControlCenterMsg::UpdatesChecked(fresh) => {
                self.is_updating_check = false;
                self.updates = fresh;
            }
            ControlCenterMsg::LaunchUpdateTerminal => {
                (self.close_cb)();
                thread::spawn(|| {
                    let _ = Command::new("uwsm")
                        .args([
                            "app",
                            "--",
                            "kitty",
                            "-e",
                            "sh",
                            "-c",
                            "paru; rc=$?; inerium-updates --check >/dev/null 2>&1 || true; if [ \"$rc\" -ne 0 ]; then printf '\\n[paru завершился с кодом %s] Нажмите Enter для закрытия...' \"$rc\"; read -r _; fi",
                        ])
                        .spawn();
                });
            }
            ControlCenterMsg::ToggleHypridle => {
                self.is_idle_active = !self.is_idle_active;
                hw::toggle_hypridle(self.is_idle_active);
            }
            ControlCenterMsg::ToggleTheme => {
                let _ = Command::new("inerium-theme").arg("toggle").status();
                let (is_dark, name, author, count) = get_theme_info();
                self.is_dark_theme = is_dark;
                self.theme_name = name;
                self.theme_author = author;
                self.available_themes_count = count;
            }
            ControlCenterMsg::NavAppearance => {
                (self.nav_cb)("appearance");
            }
            ControlCenterMsg::LockScreen => {
                (self.close_cb)();
                thread::spawn(|| {
                    let _ = Command::new("sh")
                        .args(["-c", "hyprlock || loginctl lock-session"])
                        .output();
                });
            }
            ControlCenterMsg::PowerMenu => {
                (self.nav_cb)("power");
            }

            // Quick Tiles
            ControlCenterMsg::ToggleWifi => {
                self.is_wifi = !self.is_wifi;
                self.wifi_sub = get_wifi_sub(self.is_wifi);
                hw::toggle_wifi(self.is_wifi);
            }
            ControlCenterMsg::ToggleBt => {
                self.is_bt = !self.is_bt;
                hw::toggle_bluetooth(self.is_bt);
            }
            ControlCenterMsg::ToggleDnd => {
                self.is_dnd = !self.is_dnd;
                hw::toggle_dnd();
            }
            ControlCenterMsg::ToggleMic => {
                self.is_mic_muted = !self.is_mic_muted;
                hw::toggle_mic_mute();
            }
            ControlCenterMsg::NavNetwork => {
                (self.nav_cb)("network");
            }
            ControlCenterMsg::NavBluetooth => {
                (self.nav_cb)("bluetooth");
            }
            ControlCenterMsg::NavAudio => {
                (self.nav_cb)("audio");
            }

            // Sliders
            ControlCenterMsg::ToggleVolumeMute => {
                hw::toggle_volume_mute();
                self.volume_muted = hw::is_volume_muted();
            }
            ControlCenterMsg::SetVolume(val) => {
                self.volume = val;
                hw::set_volume(val);
            }
            ControlCenterMsg::SetLaptopBrightness(val) => {
                self.laptop_brightness = val;
                hw::set_laptop_brightness(val);
            }
            ControlCenterMsg::SetDdcBrightness(val) => {
                self.ddc_brightness = val;
                hw::set_ddc_brightness_async(val);
            }
            ControlCenterMsg::DdcUpdated(val) => {
                self.ddc_brightness = val;
            }

            // Power & Cooling
            ControlCenterMsg::SetProfile(profile) => {
                self.current_profile = profile.clone();
                hw::set_power_profile(&profile);
            }
            ControlCenterMsg::RefreshFanTelemetry => {
                if self.has_fan_telem {
                    self.fan_str = compute_fan_string(self.cooler_boost);
                }
            }
            ControlCenterMsg::ToggleCoolerBoost => {
                self.cooler_boost = !self.cooler_boost;
                hw::set_cooler_boost(self.cooler_boost);
                if self.has_fan_telem {
                    self.fan_str = compute_fan_string(self.cooler_boost);
                }
            }
            ControlCenterMsg::ToggleBatteryThreshold => {
                let next = if self.battery_threshold < 100 { 100 } else { 80 };
                self.battery_threshold = next;
                hw::set_battery_threshold(next);
            }

            // Graphics
            ControlCenterMsg::SwitchGpuMode(target) => {
                self.gpu_switching = true;
                self.gpu_status_text = format!(
                    "󰑐 Переключение на {}: пересборка initramfs...",
                    target
                );
                let s = sender.clone();
                thread::spawn(move || {
                    let ok = hw::switch_gpu_mode(target);
                    s.input(ControlCenterMsg::GpuSwitchFinished(ok, target));
                });
            }
            ControlCenterMsg::GpuSwitchFinished(ok, target) => {
                self.gpu_switching = false;
                if ok {
                    self.gpu_mode = target.to_string();
                    self.gpu_status_ok = true;
                    self.gpu_status_text = tr(
                        "󰄬 Ready. Mode will apply after reboot.",
                        "󰄬 Готово. Режим применится после перезагрузки.",
                    )
                    .to_string();
                    self.show_reboot = true;
                } else {
                    self.gpu_status_ok = false;
                    self.gpu_status_text = tr(
                        "󰅙 Envycontrol switch error",
                        "󰅙 Ошибка переключения envycontrol",
                    )
                    .to_string();
                }
            }
            ControlCenterMsg::Reboot => {
                let _ = Command::new("systemctl").arg("reboot").spawn();
            }
        }
    }
}

pub fn build_control_center<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = ControlCenterInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let mut controller = ControlCenterModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 384, 540)
}
