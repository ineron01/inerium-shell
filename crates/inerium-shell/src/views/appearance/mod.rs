pub mod custom_bg;
pub mod theme_picker;
pub mod wallpaper_strip;

pub use custom_bg::{CustomBgState, CustomBgWidgets};
pub use theme_picker::{
    format_hex_color, parse_hex_color, query_themes, ThemeItem,
};

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{FileDialog, FileFilter, Orientation, PolicyType, Separator};
use inerium_i18n::tr;
use relm4::prelude::*;

use crate::bg_generator::{BackgroundSpec, BgMode, GradientDirection};
use crate::hw;
use custom_bg::populate_custom_bg_controls;
use theme_picker::populate_theme_list;
use wallpaper_strip::{populate_monitor_pills, populate_wallpapers_strip};

pub struct AppearanceModel {
    pub themes: Vec<ThemeItem>,
    pub selected_monitor: String,
    pub connected_monitors: Vec<hw::MonitorTarget>,
    pub active_wallpapers: HashMap<String, String>,
    pub wallpapers: Vec<hw::WallpaperItem>,
    pub loading_wallpapers: bool,
    pub has_multiple_monitors: bool,
    pub show_custom_bg: bool,
    pub custom_bg_mode: BgMode,
    pub custom_color_start: (u8, u8, u8),
    pub custom_color_end: (u8, u8, u8),
    pub custom_direction: GradientDirection,
    pub delete_confirm_path: Option<String>,
    pub bg_state: Rc<RefCell<CustomBgState>>,
    pub bg_widgets: Rc<RefCell<CustomBgWidgets>>,
    pub dirty_themes: Cell<bool>,
    pub dirty_monitors: Cell<bool>,
    pub dirty_wallpapers: Cell<bool>,
    pub dirty_custom_bg: Cell<bool>,
    pub syncing_bg: Rc<Cell<bool>>,
    // Night Light & Display Temperature
    pub day_temp: u32,
    pub night_light_enabled: bool,
    pub night_light_temp: u32,
    pub night_light_schedule: bool,
    pub night_light_start_h: u8,
    pub night_light_start_m: u8,
    pub night_light_end_h: u8,
    pub night_light_end_m: u8,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum AppearanceMsg {
    ApplyTheme(String),
    RefreshThemes,
    OpenThemesFolder,
    SelectMonitor(String),
    ApplyWallpaper(String),
    RequestDeleteWallpaper(String),
    CancelDeleteWallpaper,
    ConfirmDeleteWallpaper(String),
    OpenWallpapersFolder,
    AddWallpapersDialog,
    WallpapersAdded,
    WallpapersReloaded(Vec<hw::WallpaperItem>),
    CustomBgGenerated { path_str: String, notify: bool },
    ToggleCustomBg,
    SetBgMode(BgMode),
    SetBgColorStart((u8, u8, u8)),
    SetBgColorEnd((u8, u8, u8)),
    SwapBgColors,
    SetBgDirection(GradientDirection),
    ApplyCustomBg,
    SaveCustomBg,
    SetDayTemp(u32),
    SetNightLightEnabled(bool),
    SetNightLightTemp(u32),
    SetNightLightSchedule(bool),
    SetNightLightStartText(String),
    SetNightLightEndText(String),
    Back,
}

pub struct AppearanceInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
    pub current_monitor: Option<String>,
    pub scroll_to_night_light: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for AppearanceModel {
    type Input = AppearanceMsg;
    type Output = ();
    type Init = AppearanceInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (450, -1),

            // Header (Back, Title & Overview)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: &tr("‹ Back", "‹ Назад"),
                    connect_clicked => AppearanceMsg::Back,
                },

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 1,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_label: &tr("Appearance & Personalization", "Внешний вид и темы"),
                        set_xalign: 0.0,
                    },

                    gtk4::Label {
                        add_css_class: "cc-meta",
                        set_label: "~/.config/inerium/themes/",
                        set_xalign: 0.0,
                    },
                },
            },

            // Main Scrollable Area for Themes, Wallpapers & Display Temperature
            #[name = "main_scroll"]
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (PolicyType::Never, PolicyType::External),
                set_overlay_scrolling: false,
                set_min_content_height: 350,
                set_vexpand: true,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_bottom: 28,

                    // ── Section 1: Themes ─────────────────────────
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 8,

                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,
                            add_css_class: "section-header-box",

                            gtk4::Label {
                                add_css_class: "cc-label",
                                set_label: &tr("󰔎  THEMES", "󰔎  ТЕМЫ ОФОРМЛЕНИЯ"),
                                set_xalign: 0.0,
                                set_hexpand: true,
                            },

                            gtk4::Button {
                                add_css_class: "cc-chip",
                                set_label: &tr("󰉋  Folder", "󰉋  Папка тем"),
                                set_tooltip_text: Some("~/.config/inerium/themes/"),
                                connect_clicked => AppearanceMsg::OpenThemesFolder,
                            },
                        },

                        #[name = "theme_list_box"]
                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 4,
                        },
                    },

                    // ── Subtle Divider ────────────────────────────
                    Separator {
                        set_orientation: Orientation::Horizontal,
                        add_css_class: "cc-sep",
                    },

                    // ── Section 2: Wallpapers ──────────────────────
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 8,

                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 6,
                            add_css_class: "section-header-box",

                            gtk4::Label {
                                add_css_class: "cc-label",
                                set_label: &tr("󰸌  WALLPAPERS", "󰸌  ОБОИ РАБОЧЕГО СТОЛА"),
                                set_xalign: 0.0,
                                set_hexpand: true,
                            },

                            gtk4::Button {
                                add_css_class: "cc-chip",
                                set_label: &tr("󰉋  Folder", "󰉋  Папка"),
                                set_tooltip_text: Some("~/.local/share/inerium/wallpapers/"),
                                connect_clicked => AppearanceMsg::OpenWallpapersFolder,
                            },

                            gtk4::Button {
                                add_css_class: "cc-chip",
                                set_label: &tr("󰐕  Add", "󰐕  Добавить"),
                                set_tooltip_text: Some(tr("Add wallpapers from files", "Добавить свои обои из файлов")),
                                connect_clicked => AppearanceMsg::AddWallpapersDialog,
                            },

                            gtk4::Button {
                                add_css_class: "cc-chip",
                                #[watch]
                                set_css_classes: if model.show_custom_bg { &["cc-chip", "active"] } else { &["cc-chip"] },
                                set_label: &tr("󰸌  Custom", "󰸌  Свой фон"),
                                set_tooltip_text: Some(tr("Create solid color or gradient background", "Создать сплошной цвет или градиент")),
                                connect_clicked => AppearanceMsg::ToggleCustomBg,
                            },
                        },

                        // Custom Background Drawer
                        #[name = "custom_bg_box"]
                        gtk4::Box {
                            set_orientation: Orientation::Vertical,
                            set_spacing: 8,
                            add_css_class: "custom-bg-card",
                            #[watch]
                            set_visible: model.show_custom_bg,
                        },

                        // Monitor target pills (if multiple monitors)
                        #[name = "monitor_box"]
                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 6,
                            #[watch]
                            set_visible: model.has_multiple_monitors,
                        },

                        // Horizontal Wallpapers Strip
                        #[name = "wall_scroller"]
                        gtk4::ScrolledWindow {
                            set_policy: (PolicyType::External, PolicyType::Never),
                            set_overlay_scrolling: false,
                            set_propagate_natural_height: true,
                            set_min_content_height: 122,
                            set_propagate_natural_width: true,

                            #[name = "wall_strip"]
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,
                                set_margin_bottom: 2,
                                set_margin_top: 2,
                            },
                        },

                        // Dedicated horizontal scrollbar positioned strictly BELOW wallpaper cards
                        #[name = "wall_scrollbar"]
                        gtk4::Scrollbar {
                            set_orientation: Orientation::Horizontal,
                            add_css_class: "wall-scrollbar",
                            set_margin_top: 2,
                            set_margin_bottom: 3,
                            set_margin_start: 24,
                            set_margin_end: 24,
                            set_visible: false,
                        },
                    },

                    // ── Subtle Divider ────────────────────────────
                    Separator {
                        set_orientation: Orientation::Horizontal,
                        add_css_class: "cc-sep",
                    },

                    // ── Section 3: Display Temperature & Night Light ──
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 8,

                        gtk4::Label {
                            add_css_class: "cc-label",
                            set_label: &tr("󰖔  DISPLAY TEMPERATURE & NIGHT LIGHT", "󰖔  ТЕПЛОТА ЭКРАНА И НОЧНОЙ СВЕТ"),
                            set_xalign: 0.0,
                        },

                        gtk4::Box {
                            add_css_class: "cc-card",
                            set_orientation: Orientation::Vertical,
                            set_spacing: 8,

                            // Day (baseline) temperature
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    add_css_class: "cc-meta",
                                    set_width_chars: 9,
                                    set_xalign: 0.0,
                                    set_label: tr("Daylight", "Дневная"),
                                },

                                gtk4::Scale {
                                    add_css_class: "cc-scale",
                                    set_orientation: Orientation::Horizontal,
                                    set_range: (4000.0, 6500.0),
                                    set_increments: (50.0, 250.0),
                                    set_hexpand: true,
                                    set_value: model.day_temp as f64,
                                    connect_value_changed[sender] => move |sc| {
                                        sender.input(AppearanceMsg::SetDayTemp(sc.value() as u32));
                                    },
                                },

                                gtk4::Label {
                                    add_css_class: "cc-pct",
                                    set_xalign: 1.0,
                                    #[watch]
                                    set_label: &format!("{}K", model.day_temp),
                                },
                            },

                            Separator {
                                set_orientation: Orientation::Horizontal,
                                add_css_class: "cc-sep",
                            },

                            // Night Light Toggle
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    add_css_class: "cc-tile-title",
                                    set_hexpand: true,
                                    set_xalign: 0.0,
                                    set_label: tr("Night Light", "Ночной свет"),
                                },

                                gtk4::Switch {
                                    set_valign: gtk4::Align::Center,
                                    #[watch]
                                    set_active: model.night_light_enabled,
                                    connect_state_set[sender] => move |_, state| {
                                        sender.input(AppearanceMsg::SetNightLightEnabled(state));
                                        glib::Propagation::Proceed
                                    },
                                },
                            },

                            // Night Warmth Slider
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    add_css_class: "cc-meta",
                                    set_width_chars: 9,
                                    set_xalign: 0.0,
                                    set_label: tr("Night", "Ночная"),
                                },

                                gtk4::Scale {
                                    add_css_class: "cc-scale",
                                    add_css_class: "amber",
                                    set_orientation: Orientation::Horizontal,
                                    set_range: (2500.0, 6500.0),
                                    set_increments: (50.0, 250.0),
                                    set_hexpand: true,
                                    set_value: model.night_light_temp as f64,
                                    connect_value_changed[sender] => move |sc| {
                                        sender.input(AppearanceMsg::SetNightLightTemp(sc.value() as u32));
                                    },
                                },

                                gtk4::Label {
                                    add_css_class: "cc-pct",
                                    set_xalign: 1.0,
                                    #[watch]
                                    set_label: &format!("{}K", model.night_light_temp),
                                },
                            },

                            // Schedule Toggle & HH:MM Inputs
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Label {
                                    add_css_class: "cc-meta",
                                    set_hexpand: true,
                                    set_xalign: 0.0,
                                    set_label: tr("Schedule (HH:MM)", "По расписанию (ЧЧ:ММ)"),
                                },

                                gtk4::Box {
                                    set_orientation: Orientation::Horizontal,
                                    set_spacing: 6,
                                    #[watch]
                                    set_sensitive: model.night_light_schedule,

                                    gtk4::Label {
                                        add_css_class: "cc-meta",
                                        set_label: tr("From", "С"),
                                    },

                                    #[name = "nl_start_entry"]
                                    gtk4::Entry {
                                        set_width_chars: 5,
                                        set_max_length: 5,
                                        set_text: &format!("{:02}:{:02}", model.night_light_start_h, model.night_light_start_m),
                                        connect_activate[sender] => move |e| {
                                            sender.input(AppearanceMsg::SetNightLightStartText(e.text().to_string()));
                                        },
                                    },

                                    gtk4::Label {
                                        add_css_class: "cc-meta",
                                        set_label: tr("To", "До"),
                                    },

                                    #[name = "nl_end_entry"]
                                    gtk4::Entry {
                                        set_width_chars: 5,
                                        set_max_length: 5,
                                        set_text: &format!("{:02}:{:02}", model.night_light_end_h, model.night_light_end_m),
                                        connect_activate[sender] => move |e| {
                                            sender.input(AppearanceMsg::SetNightLightEndText(e.text().to_string()));
                                        },
                                    },
                                },

                                gtk4::Switch {
                                    set_valign: gtk4::Align::Center,
                                    #[watch]
                                    set_active: model.night_light_schedule,
                                    connect_state_set[sender] => move |_, state| {
                                        sender.input(AppearanceMsg::SetNightLightSchedule(state));
                                        glib::Propagation::Proceed
                                    },
                                },
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let themes = query_themes();
        let connected_monitors = hw::list_connected_monitors();
        let has_multiple_monitors = connected_monitors.len() > 1;
        let selected_monitor = init.current_monitor.clone().unwrap_or_else(|| "all".to_string());
        let active_wallpapers = hw::get_active_wallpapers();
        let wallpapers = Vec::new();

        let sender_init = sender.clone();
        std::thread::spawn(move || {
            let walls = hw::list_available_wallpapers();
            let _ = sender_init.input_sender().send(AppearanceMsg::WallpapersReloaded(walls));
        });

        let bg_state = Rc::new(RefCell::new(CustomBgState {
            mode: BgMode::Solid,
            color_start: (0x18, 0x16, 0x16),
            color_end: (0x28, 0x27, 0x28),
            direction: GradientDirection::Vertical,
        }));

        let nl_cfg = hw::night_light::load_config();

        let model = Self {
            themes,
            selected_monitor,
            connected_monitors,
            active_wallpapers,
            wallpapers,
            loading_wallpapers: true,
            has_multiple_monitors,
            show_custom_bg: false,
            custom_bg_mode: BgMode::Solid,
            custom_color_start: (0x18, 0x16, 0x16),
            custom_color_end: (0x28, 0x27, 0x28),
            custom_direction: GradientDirection::Vertical,
            delete_confirm_path: None,
            bg_state,
            bg_widgets: Rc::new(RefCell::new(CustomBgWidgets::default())),
            dirty_themes: Cell::new(false),
            dirty_monitors: Cell::new(false),
            dirty_wallpapers: Cell::new(true),
            dirty_custom_bg: Cell::new(false),
            syncing_bg: Rc::new(Cell::new(false)),
            day_temp: nl_cfg.day_temperature,
            night_light_enabled: nl_cfg.enabled,
            night_light_temp: nl_cfg.temperature,
            night_light_schedule: nl_cfg.schedule_enabled,
            night_light_start_h: nl_cfg.start_hour,
            night_light_start_m: nl_cfg.start_minute,
            night_light_end_h: nl_cfg.end_hour,
            night_light_end_m: nl_cfg.end_minute,
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();
        let bg_w_destroy = model.bg_widgets.clone();
        root.connect_destroy(move |_| {
            bg_w_destroy.borrow_mut().clear();
        });

        let hadj = widgets.wall_scroller.hadjustment();
        widgets.wall_scrollbar.set_adjustment(Some(&hadj));
        let sb_changed = widgets.wall_scrollbar.clone();
        hadj.connect_changed(move |a| {
            sb_changed.set_visible(a.upper() > a.page_size() + 1.0);
        });

        populate_theme_list(&widgets.theme_list_box, &model.themes, &sender);
        populate_monitor_pills(&widgets.monitor_box, &model, &sender);
        populate_custom_bg_controls(&widgets.custom_bg_box, &model, &sender);
        populate_wallpapers_strip(&widgets.wall_scroller, &widgets.wall_strip, &model, &sender);

        // Apply HH:MM on focus leave for time entries
        for (entry, is_start) in [(&widgets.nl_start_entry, true), (&widgets.nl_end_entry, false)] {
            gtk4::prelude::EntryExt::set_alignment(entry, 0.5);
            let e = entry.clone();
            let s = sender.clone();
            let fc = gtk4::EventControllerFocus::new();
            fc.connect_leave(move |_| {
                let text = e.text().to_string();
                let msg = if is_start {
                    AppearanceMsg::SetNightLightStartText(text)
                } else {
                    AppearanceMsg::SetNightLightEndText(text)
                };
                s.input(msg);
            });
            entry.add_controller(fc);
        }

        if init.scroll_to_night_light {
            let scroller = widgets.main_scroll.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(60), move || {
                let vadj = scroller.vadjustment();
                let target = (vadj.upper() - vadj.page_size()).max(0.0);
                vadj.set_value(target);
            });
        }

        ComponentParts { model, widgets }
    }

    fn post_view() {
        if !nl_start_entry.has_focus() {
            let expected = format!("{:02}:{:02}", self.night_light_start_h, self.night_light_start_m);
            if nl_start_entry.text() != expected {
                nl_start_entry.set_text(&expected);
            }
        }
        if !nl_end_entry.has_focus() {
            let expected = format!("{:02}:{:02}", self.night_light_end_h, self.night_light_end_m);
            if nl_end_entry.text() != expected {
                nl_end_entry.set_text(&expected);
            }
        }
        if self.dirty_themes.get() {
            self.dirty_themes.set(false);
            populate_theme_list(theme_list_box, &self.themes, &sender);
        }
        if self.dirty_monitors.get() {
            self.dirty_monitors.set(false);
            populate_monitor_pills(monitor_box, self, &sender);
        }
        if self.dirty_custom_bg.get() {
            self.dirty_custom_bg.set(false);
            populate_custom_bg_controls(custom_bg_box, self, &sender);
        }
        if self.dirty_wallpapers.get() {
            self.dirty_wallpapers.set(false);
            populate_wallpapers_strip(wall_scroller, wall_strip, self, &sender);
            let hadj = wall_scroller.hadjustment();
            wall_scrollbar.set_visible(hadj.upper() > hadj.page_size() + 1.0);
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppearanceMsg::ApplyTheme(id) => {
                let id_clone = id.clone();
                let sender_clone = sender.clone();
                std::thread::spawn(move || {
                    let _ = Command::new("inerium-theme").arg("set").arg(&id_clone).status();
                    let _ = sender_clone.input_sender().send(AppearanceMsg::RefreshThemes);
                });
            }
            AppearanceMsg::RefreshThemes => {
                self.themes = query_themes();
                self.dirty_themes.set(true);
            }
            AppearanceMsg::OpenThemesFolder => {
                let home = hw::home_dir();
                let path = home.join(".config/inerium/themes");
                std::thread::spawn(move || {
                    let _ = hw::open_folder(&path);
                });
            }
            AppearanceMsg::SelectMonitor(target) => {
                self.selected_monitor = target;
                self.dirty_monitors.set(true);
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::ApplyWallpaper(path) => {
                self.delete_confirm_path = None;
                let mon_opt = if self.selected_monitor == "all" {
                    None
                } else {
                    Some(self.selected_monitor.clone())
                };
                hw::set_wallpaper_async(path.clone(), mon_opt);
                if self.selected_monitor == "all" {
                    for m in &self.connected_monitors {
                        self.active_wallpapers.insert(m.name.clone(), path.clone());
                    }
                } else {
                    self.active_wallpapers
                        .insert(self.selected_monitor.clone(), path);
                }
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::RequestDeleteWallpaper(path_str) => {
                self.delete_confirm_path = Some(path_str);
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::CancelDeleteWallpaper => {
                self.delete_confirm_path = None;
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::ConfirmDeleteWallpaper(path_str) => {
                self.delete_confirm_path = None;
                let p = Path::new(&path_str);
                if let Ok(canon_user) = p.canonicalize()
                    && let Ok(canon_dir) = hw::inerium_wallpapers_dir().canonicalize()
                    && canon_user.starts_with(canon_dir)
                {
                    let _ = std::fs::remove_file(&canon_user);
                    if let Some(file_name) = canon_user.file_name().and_then(|f| f.to_str()) {
                        let thumb = hw::cache_dir()
                            .join("inerium-wall-thumbs")
                            .join(format!("{}.160x90.png", file_name));
                        let _ = std::fs::remove_file(thumb);
                    }
                    let sender_del = sender.clone();
                    std::thread::spawn(move || {
                        let walls = hw::list_available_wallpapers();
                        let _ = sender_del.input_sender().send(AppearanceMsg::WallpapersReloaded(walls));
                    });
                }
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::OpenWallpapersFolder => {
                let dir = hw::inerium_wallpapers_dir();
                std::thread::spawn(move || {
                    let _ = hw::open_folder(&dir);
                });
            }
            AppearanceMsg::AddWallpapersDialog => {
                let filter = FileFilter::new();
                filter.set_name(Some(tr("Images", "Изображения")));
                filter.add_mime_type("image/png");
                filter.add_mime_type("image/jpeg");
                filter.add_mime_type("image/webp");

                let filters = gio::ListStore::new::<FileFilter>();
                filters.append(&filter);

                let dialog = FileDialog::builder()
                    .title(tr("Add Wallpapers", "Добавить обои"))
                    .filters(&filters)
                    .build();

                let s = sender.clone();
                crate::set_modal_dialog_active(true);
                dialog.open_multiple(
                    gtk4::Window::NONE,
                    gio::Cancellable::NONE,
                    move |res| {
                        crate::set_modal_dialog_active(false);
                        if let Ok(files) = res {
                            let dest_dir = hw::inerium_wallpapers_dir();
                            for i in 0..files.n_items() {
                                if let Some(file) = files.item(i).and_downcast::<gio::File>()
                                    && let Some(src_path) = file.path()
                                    && let Some(file_name) = src_path.file_name()
                                {
                                    let dest_path = dest_dir.join(file_name);
                                    let _ = std::fs::copy(&src_path, &dest_path);
                                }
                            }
                            s.input(AppearanceMsg::WallpapersAdded);
                        }
                    },
                );
            }
            AppearanceMsg::WallpapersAdded => {
                let sender_add = sender.clone();
                std::thread::spawn(move || {
                    let walls = hw::list_available_wallpapers();
                    let _ = sender_add.input_sender().send(AppearanceMsg::WallpapersReloaded(walls));
                });
            }
            AppearanceMsg::WallpapersReloaded(walls) => {
                self.loading_wallpapers = false;
                self.wallpapers = walls;
                self.dirty_wallpapers.set(true);
            }
            AppearanceMsg::ToggleCustomBg => {
                self.show_custom_bg = !self.show_custom_bg;
                self.dirty_custom_bg.set(true);
            }
            AppearanceMsg::SetBgMode(mode) => {
                self.custom_bg_mode = mode;
                self.bg_state.borrow_mut().mode = mode;
                self.bg_widgets.borrow_mut().bg_controls_mode = None; // Rebuild control layout for new mode
                self.dirty_custom_bg.set(true);
                let w = self.bg_widgets.borrow();
                if let Some(p) = w.preview_area.upgrade() {
                    p.queue_draw();
                }
            }
            AppearanceMsg::SetBgColorStart(c) => {
                self.custom_color_start = c;
                self.bg_state.borrow_mut().color_start = c;
                self.syncing_bg.set(true);
                let w = self.bg_widgets.borrow();
                if let Some(e) = w.entry_start.upgrade()
                    && parse_hex_color(&e.text()) != Some(c)
                {
                    e.set_text(&format_hex_color(c));
                }
                if let Some(r) = w.r_scale.upgrade()
                    && (r.value().round() as u8) != c.0
                {
                    r.set_value(c.0 as f64);
                }
                if let Some(g) = w.g_scale.upgrade()
                    && (g.value().round() as u8) != c.1
                {
                    g.set_value(c.1 as f64);
                }
                if let Some(b) = w.b_scale.upgrade()
                    && (b.value().round() as u8) != c.2
                {
                    b.set_value(c.2 as f64);
                }
                self.syncing_bg.set(false);
                if let Some(sw) = w.swatch_start.upgrade() {
                    sw.queue_draw();
                }
                if let Some(p) = w.preview_area.upgrade() {
                    p.queue_draw();
                }
                w.update_active_presets_and_dirs(
                    self.custom_bg_mode,
                    self.custom_color_start,
                    self.custom_color_end,
                    self.custom_direction,
                );
            }
            AppearanceMsg::SetBgColorEnd(c) => {
                self.custom_color_end = c;
                self.bg_state.borrow_mut().color_end = c;
                self.syncing_bg.set(true);
                let w = self.bg_widgets.borrow();
                if let Some(e) = w.entry_end.upgrade()
                    && parse_hex_color(&e.text()) != Some(c)
                {
                    e.set_text(&format_hex_color(c));
                }
                self.syncing_bg.set(false);
                if let Some(sw) = w.swatch_end.upgrade() {
                    sw.queue_draw();
                }
                if let Some(p) = w.preview_area.upgrade() {
                    p.queue_draw();
                }
                w.update_active_presets_and_dirs(
                    self.custom_bg_mode,
                    self.custom_color_start,
                    self.custom_color_end,
                    self.custom_direction,
                );
            }
            AppearanceMsg::SwapBgColors => {
                std::mem::swap(&mut self.custom_color_start, &mut self.custom_color_end);
                let c1 = self.custom_color_start;
                let c2 = self.custom_color_end;
                {
                    let mut st = self.bg_state.borrow_mut();
                    st.color_start = c1;
                    st.color_end = c2;
                }
                self.syncing_bg.set(true);
                let w = self.bg_widgets.borrow();
                if let Some(e1) = w.entry_start.upgrade() {
                    e1.set_text(&format_hex_color(c1));
                }
                if let Some(e2) = w.entry_end.upgrade() {
                    e2.set_text(&format_hex_color(c2));
                }
                self.syncing_bg.set(false);
                if let Some(sw1) = w.swatch_start.upgrade() {
                    sw1.queue_draw();
                }
                if let Some(sw2) = w.swatch_end.upgrade() {
                    sw2.queue_draw();
                }
                if let Some(p) = w.preview_area.upgrade() {
                    p.queue_draw();
                }
                w.update_active_presets_and_dirs(
                    self.custom_bg_mode,
                    self.custom_color_start,
                    self.custom_color_end,
                    self.custom_direction,
                );
            }
            AppearanceMsg::SetBgDirection(dir) => {
                self.custom_direction = dir;
                self.bg_state.borrow_mut().direction = dir;
                let w = self.bg_widgets.borrow();
                if let Some(p) = w.preview_area.upgrade() {
                    p.queue_draw();
                }
                w.update_active_presets_and_dirs(
                    self.custom_bg_mode,
                    self.custom_color_start,
                    self.custom_color_end,
                    self.custom_direction,
                );
            }
            AppearanceMsg::ApplyCustomBg => {
                let spec = BackgroundSpec {
                    mode: self.custom_bg_mode,
                    color_start: self.custom_color_start,
                    color_end: self.custom_color_end,
                    direction: self.custom_direction,
                };
                let dir = hw::inerium_wallpapers_dir();
                let sender_gen = sender.clone();
                std::thread::spawn(move || {
                    if let Ok(path) = spec.generate_and_save(&dir) {
                        let _ = sender_gen.input_sender().send(AppearanceMsg::CustomBgGenerated {
                            path_str: path.to_string_lossy().to_string(),
                            notify: false,
                        });
                    }
                });
            }
            AppearanceMsg::SaveCustomBg => {
                let spec = BackgroundSpec {
                    mode: self.custom_bg_mode,
                    color_start: self.custom_color_start,
                    color_end: self.custom_color_end,
                    direction: self.custom_direction,
                };
                let dir = hw::inerium_wallpapers_dir();
                let sender_gen = sender.clone();
                std::thread::spawn(move || {
                    if let Ok(path) = spec.generate_and_save(&dir) {
                        let _ = sender_gen.input_sender().send(AppearanceMsg::CustomBgGenerated {
                            path_str: path.to_string_lossy().to_string(),
                            notify: true,
                        });
                    }
                });
            }
            AppearanceMsg::CustomBgGenerated { path_str, notify } => {
                let mon_opt = if self.selected_monitor == "all" {
                    None
                } else {
                    Some(self.selected_monitor.clone())
                };
                hw::set_wallpaper_async(path_str.clone(), mon_opt);
                if self.selected_monitor == "all" {
                    for m in &self.connected_monitors {
                        self.active_wallpapers.insert(m.name.clone(), path_str.clone());
                    }
                } else {
                    self.active_wallpapers.insert(self.selected_monitor.clone(), path_str.clone());
                }

                if notify {
                    let notify_title = tr("Wallpaper Saved", "Фон сохранен");
                    let notify_body = tr("Background added to wallpapers folder", "Фон добавлен в папку обоев");
                    let app_name = tr("Wallpaper", "Обои");
                    let _ = Command::new("notify-send")
                        .args(["-a", app_name, "-i", &path_str, notify_title, notify_body])
                        .spawn();
                }

                let sender_reload = sender.clone();
                std::thread::spawn(move || {
                    let walls = hw::list_available_wallpapers();
                    let _ = sender_reload.input_sender().send(AppearanceMsg::WallpapersReloaded(walls));
                });
            }
            AppearanceMsg::SetDayTemp(raw) => {
                let rounded = ((raw + 25) / 50 * 50).clamp(
                    hw::night_light::MIN_DAY_TEMP,
                    hw::night_light::MAX_TEMP,
                );
                if self.day_temp != rounded {
                    let cfg = hw::night_light::set_day_temperature(rounded);
                    self.day_temp = cfg.day_temperature;
                }
            }
            AppearanceMsg::SetNightLightEnabled(enabled) => {
                if self.night_light_enabled != enabled {
                    let cfg = hw::night_light::set_enabled(enabled);
                    self.night_light_enabled = cfg.enabled;
                }
            }
            AppearanceMsg::SetNightLightTemp(raw) => {
                let rounded = ((raw + 25) / 50 * 50).clamp(
                    hw::night_light::MIN_NIGHT_TEMP,
                    hw::night_light::MAX_TEMP,
                );
                if self.night_light_temp != rounded {
                    let cfg = hw::night_light::set_temperature(rounded);
                    self.night_light_temp = cfg.temperature;
                }
            }
            AppearanceMsg::SetNightLightSchedule(enabled) => {
                if self.night_light_schedule != enabled {
                    let cfg = hw::night_light::set_schedule_enabled(enabled);
                    self.night_light_schedule = cfg.schedule_enabled;
                    self.night_light_enabled = cfg.enabled;
                }
            }
            AppearanceMsg::SetNightLightStartText(text) => {
                if let Some((h, m)) = hw::night_light::parse_hhmm(&text)
                    && (h != self.night_light_start_h || m != self.night_light_start_m)
                {
                    let cfg = hw::night_light::set_schedule_times(
                        h,
                        m,
                        self.night_light_end_h,
                        self.night_light_end_m,
                    );
                    self.night_light_start_h = cfg.start_hour;
                    self.night_light_start_m = cfg.start_minute;
                    self.night_light_enabled = cfg.enabled;
                }
            }
            AppearanceMsg::SetNightLightEndText(text) => {
                if let Some((h, m)) = hw::night_light::parse_hhmm(&text)
                    && (h != self.night_light_end_h || m != self.night_light_end_m)
                {
                    let cfg = hw::night_light::set_schedule_times(
                        self.night_light_start_h,
                        self.night_light_start_m,
                        h,
                        m,
                    );
                    self.night_light_end_h = cfg.end_hour;
                    self.night_light_end_m = cfg.end_minute;
                    self.night_light_enabled = cfg.enabled;
                }
            }
            AppearanceMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

pub fn build_appearance_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
    current_monitor: Option<String>,
    scroll_to_night_light: bool,
) -> (gtk4::Box, i32, i32, Box<dyn std::any::Any>)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = AppearanceInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
        current_monitor,
        scroll_to_night_light,
    };

    let controller = AppearanceModel::builder().launch(init).detach();
    let root = controller.widget().clone();
    (root, 455, 660, Box::new(controller))
}
