pub mod graphics;
pub mod header;
pub mod power_cooling;
pub mod sliders;
pub mod tiles;

pub use header::get_theme_info;
pub use power_cooling::compute_fan_string;
pub use tiles::get_wifi_sub;

use std::cell::RefCell;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType};
use inerium_i18n::{tr, tr_fmt, tr_plural};
use relm4::prelude::*;

use crate::hw;

thread_local! {
    static DDC_SENDER: RefCell<Option<(u64, ComponentSender<ControlCenterModel>)>> = const { RefCell::new(None) };
}
static DDC_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Default)]
struct CliStateSnapshot {
    has_ppd: bool,
    current_profile: String,
    has_envycontrol: bool,
    gpu_mode: String,
    wifi_sub: String,
}

static LAST_CLI_STATE: std::sync::Mutex<Option<CliStateSnapshot>> = std::sync::Mutex::new(None);

pub fn on_ddc_updated(val: u32) {
    DDC_SENDER.with(|s| {
        if let Some((_, ref sender)) = *s.borrow() {
            sender.input(ControlCenterMsg::DdcUpdated(val));
        }
    });
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

    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
    pub instance_gen: u64,
}

#[derive(Debug)]
pub enum ControlCenterMsg {
    CliStateLoaded {
        has_ppd: bool,
        current_profile: String,
        has_envycontrol: bool,
        gpu_mode: String,
        wifi_sub: String,
    },
    // Header
    CheckUpdates,
    UpdatesChecked(hw::SystemUpdates),
    LaunchUpdateTerminal,
    ToggleHypridle,
    ToggleTheme,
    ThemeUpdated {
        is_dark: bool,
        name: String,
        author: String,
        count: usize,
    },
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
                set_valign: Align::Center,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 2,
                    set_hexpand: true,
                    set_valign: Align::Center,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_xalign: 0.0,
                        set_label: &model.hostname,
                    },

                    gtk4::Box {
                        set_orientation: Orientation::Horizontal,
                        set_spacing: 8,
                        set_valign: Align::Center,

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

                // Right icon buttons
                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 6,
                    set_valign: Align::Center,

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
                            set_valign: Align::Center,

                            gtk4::Box {
                                set_orientation: Orientation::Vertical,
                                set_spacing: 2,
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
                                add_css_class: "cc-seg-wide",
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 3,
                                set_valign: Align::Center,

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
        let is_bt = hw::is_bluetooth_enabled();
        let is_dnd = hw::is_dnd_active();
        let is_mic_muted = hw::is_mic_muted();
        let is_idle_active = hw::is_hypridle_active();
        let (is_dark_theme, theme_name, theme_author, available_themes_count) = get_theme_info();

        let (volume, volume_muted) = hw::get_sink_volume_state();
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

        let (has_ppd, current_profile, has_envycontrol, gpu_mode, wifi_sub) = {
            let guard = LAST_CLI_STATE.lock().unwrap();
            if let Some(ref snap) = *guard {
                let w_sub = if !is_wifi {
                    tr("Disabled", "Выключен").to_string()
                } else {
                    snap.wifi_sub.clone()
                };
                (
                    snap.has_ppd,
                    snap.current_profile.clone(),
                    snap.has_envycontrol,
                    snap.gpu_mode.clone(),
                    w_sub,
                )
            } else {
                (
                    false,
                    String::new(),
                    false,
                    String::new(),
                    if is_wifi {
                        tr("Enabled", "Включен").to_string()
                    } else {
                        tr("Disabled", "Выключен").to_string()
                    },
                )
            }
        };

        let sender_cli = sender.clone();
        std::thread::spawn(move || {
            let has_ppd = hw::has_power_profiles();
            let current_profile = if has_ppd {
                hw::get_power_profile()
            } else {
                String::new()
            };
            let has_envycontrol = hw::has_envycontrol();
            let gpu_mode = if has_envycontrol {
                hw::get_gpu_mode()
            } else {
                String::new()
            };
            let wifi_sub = get_wifi_sub(is_wifi);

            let snap = CliStateSnapshot {
                has_ppd,
                current_profile: current_profile.clone(),
                has_envycontrol,
                gpu_mode: gpu_mode.clone(),
                wifi_sub: wifi_sub.clone(),
            };
            if let Ok(mut g) = LAST_CLI_STATE.lock() {
                *g = Some(snap);
            }

            sender_cli.input(ControlCenterMsg::CliStateLoaded {
                has_ppd,
                current_profile,
                has_envycontrol,
                gpu_mode,
                wifi_sub,
            });
        });

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

        let instance_gen = DDC_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

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
            instance_gen,
        };

        // Register DDC sender with generation in thread-local for hw/backlight.rs:on_ddc_updated
        DDC_SENDER.with(|s| {
            *s.borrow_mut() = Some((instance_gen, sender.clone()));
        });

        // Trigger asynchronous DDC hardware refresh if external monitor exists
        if has_external_monitor {
            hw::refresh_ddc_brightness_async();
        }

        let widgets = view_output!();

        let gen_dest = instance_gen;
        root.connect_destroy(move |_| {
            DDC_SENDER.with(|s| {
                if let Some((g, _)) = *s.borrow()
                    && g == gen_dest
                {
                    *s.borrow_mut() = None;
                }
            });
        });

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
        if let ControlCenterMsg::CliStateLoaded {
            has_ppd,
            current_profile,
            has_envycontrol,
            gpu_mode,
            wifi_sub,
        } = msg
        {
            self.has_ppd = has_ppd;
            self.current_profile = current_profile;
            self.has_envycontrol = has_envycontrol;
            self.gpu_mode = gpu_mode;
            self.wifi_sub = wifi_sub;
            return;
        }
        if header::handle_header_msg(self, &msg, &sender) {
            return;
        }
        if tiles::handle_tiles_msg(self, &msg) {
            return;
        }
        if sliders::handle_sliders_msg(self, &msg) {
            return;
        }
        if power_cooling::handle_power_cooling_msg(self, &msg) {
            return;
        }
        graphics::handle_graphics_msg(self, &msg, &sender);
    }
}

impl Drop for ControlCenterModel {
    fn drop(&mut self) {
        let instance_gen = self.instance_gen;
        DDC_SENDER.with(|s| {
            if let Some((g, _)) = *s.borrow()
                && g == instance_gen
            {
                *s.borrow_mut() = None;
            }
        });
    }
}

pub fn build_control_center<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32, Box<dyn std::any::Any>)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = ControlCenterInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let controller = ControlCenterModel::builder().launch(init).detach();
    let root = controller.widget().clone();
    (root, 384, 540, Box::new(controller))
}
