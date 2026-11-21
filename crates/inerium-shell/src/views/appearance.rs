use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DrawingArea, Entry, FileDialog, FileFilter, GestureClick, Label,
    Orientation, Picture, PolicyType, Scale, Separator,
};
use inerium_i18n::tr;
use relm4::prelude::*;

use crate::bg_generator::{BackgroundSpec, BgMode, GradientDirection};
use crate::hw;

#[derive(Clone, Debug)]
pub struct ThemeItem {
    pub id: String,
    pub name: String,
    pub author: String,
    pub variant: String,
    pub description: String,
    pub is_active: bool,
    pub bg: String,
    pub surface: String,
    pub accent: String,
    pub fg: String,
}

pub fn query_themes() -> Vec<ThemeItem> {
    let mut items = Vec::new();
    if let Ok(out) = Command::new("inerium-theme").arg("list").arg("--json").output()
        && let Ok(val) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
    {
        for v in val {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let author = v.get("author").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let variant = v.get("variant").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let description = v.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let is_active = v.get("is_active").and_then(|x| x.as_bool()).unwrap_or(false);
            let bg = v.pointer("/colors/bg").and_then(|x| x.as_str()).unwrap_or("#221f1f").to_string();
            let surface = v.pointer("/colors/surface").and_then(|x| x.as_str()).unwrap_or("#2d2a2a").to_string();
            let accent = v.pointer("/colors/accent").and_then(|x| x.as_str()).unwrap_or("#8ba4b0").to_string();
            let fg = v.pointer("/colors/fg").and_then(|x| x.as_str()).unwrap_or("#c5c9c5").to_string();
            items.push(ThemeItem {
                id,
                name,
                author,
                variant,
                description,
                is_active,
                bg,
                surface,
                accent,
                fg,
            });
        }
    }
    items
}

pub fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

pub fn format_hex_color(c: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

fn make_color_swatch_static(r: u8, g: u8, b: u8, width: i32, height: i32, radius: f64) -> DrawingArea {
    let area = DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_draw_func(move |_area, cr, w, h| {
        let w = w as f64;
        let h = h as f64;
        let inset = 0.5;
        let w_in = (w - inset * 2.0).max(1.0);
        let h_in = (h - inset * 2.0).max(1.0);
        let rad = (radius - inset).max(0.0).min(w_in / 2.0).min(h_in / 2.0);

        if rad > 0.0 {
            cr.new_sub_path();
            cr.arc(inset + w_in - rad, inset + rad, rad, -std::f64::consts::FRAC_PI_2, 0.0);
            cr.arc(inset + w_in - rad, inset + h_in - rad, rad, 0.0, std::f64::consts::FRAC_PI_2);
            cr.arc(inset + rad, inset + h_in - rad, rad, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
            cr.arc(inset + rad, inset + rad, rad, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
            cr.close_path();
        } else {
            cr.rectangle(inset, inset, w_in, h_in);
        }

        cr.set_source_rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        let _ = cr.fill_preserve();

        // Adaptive high-contrast rim: prevents swatch from blending into any light/dark background
        let lum = 0.299 * (r as f64 / 255.0) + 0.587 * (g as f64 / 255.0) + 0.114 * (b as f64 / 255.0);
        cr.set_line_width(1.0);
        if lum < 0.45 {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.28);
        } else {
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.24);
        }
        let _ = cr.stroke();
    });
    area
}

#[derive(Clone, Debug)]
pub struct CustomBgState {
    pub mode: BgMode,
    pub color_start: (u8, u8, u8),
    pub color_end: (u8, u8, u8),
    pub direction: GradientDirection,
}

#[derive(Default)]
pub struct CustomBgWidgets {
    pub bg_controls_mode: Option<BgMode>,
    pub preview_area: Option<DrawingArea>,
    pub swatch_start: Option<DrawingArea>,
    pub swatch_end: Option<DrawingArea>,
    pub entry_start: Option<Entry>,
    pub entry_end: Option<Entry>,
    pub r_scale: Option<Scale>,
    pub g_scale: Option<Scale>,
    pub b_scale: Option<Scale>,
}

pub struct AppearanceModel {
    pub themes: Vec<ThemeItem>,
    pub selected_monitor: String,
    pub connected_monitors: Vec<hw::MonitorTarget>,
    pub active_wallpapers: HashMap<String, String>,
    pub wallpapers: Vec<hw::WallpaperItem>,
    pub has_multiple_monitors: bool,
    pub show_custom_bg: bool,
    pub custom_bg_mode: BgMode,
    pub custom_color_start: (u8, u8, u8),
    pub custom_color_end: (u8, u8, u8),
    pub custom_direction: GradientDirection,
    pub delete_confirm_path: Option<String>,
    pub bg_state: Rc<RefCell<CustomBgState>>,
    pub bg_widgets: Rc<RefCell<CustomBgWidgets>>,
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
    ToggleCustomBg,
    SetBgMode(BgMode),
    SetBgColorStart((u8, u8, u8)),
    SetBgColorEnd((u8, u8, u8)),
    SwapBgColors,
    SetBgDirection(GradientDirection),
    ApplyCustomBg,
    SaveCustomBg,
    Back,
}

pub struct AppearanceInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
    pub current_monitor: Option<String>,
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

            // Main Scrollable Area for Themes and Wallpapers
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (PolicyType::Never, PolicyType::Automatic),
                set_propagate_natural_height: true,
                set_min_content_height: 480,
                set_max_content_height: 680,
                set_vexpand: true,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 12,

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
                        gtk4::ScrolledWindow {
                            set_policy: (PolicyType::Automatic, PolicyType::Never),
                            set_overlay_scrolling: false,
                            set_propagate_natural_height: true,
                            set_min_content_height: 154,
                            set_propagate_natural_width: true,

                            #[name = "wall_strip"]
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,
                                set_margin_bottom: 6,
                                set_margin_top: 2,
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
        let wallpapers = hw::list_available_wallpapers();

        let bg_state = Rc::new(RefCell::new(CustomBgState {
            mode: BgMode::Solid,
            color_start: (0x18, 0x16, 0x16),
            color_end: (0x28, 0x27, 0x28),
            direction: GradientDirection::Vertical,
        }));

        let model = Self {
            themes,
            selected_monitor,
            connected_monitors,
            active_wallpapers,
            wallpapers,
            has_multiple_monitors,
            show_custom_bg: false,
            custom_bg_mode: BgMode::Solid,
            custom_color_start: (0x18, 0x16, 0x16),
            custom_color_end: (0x28, 0x27, 0x28),
            custom_direction: GradientDirection::Vertical,
            delete_confirm_path: None,
            bg_state,
            bg_widgets: Rc::new(RefCell::new(CustomBgWidgets::default())),
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();
        populate_theme_list(&widgets.theme_list_box, &model.themes, &sender);
        populate_monitor_pills(&widgets.monitor_box, &model, &sender);
        populate_custom_bg_controls(&widgets.custom_bg_box, &model, &sender);
        populate_wallpapers_strip(&widgets.wall_strip, &model, &sender);

        ComponentParts { model, widgets }
    }

    fn post_view() {
        populate_theme_list(theme_list_box, &self.themes, &sender);
        populate_monitor_pills(monitor_box, self, &sender);
        populate_custom_bg_controls(custom_bg_box, self, &sender);
        populate_wallpapers_strip(wall_strip, self, &sender);
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppearanceMsg::ApplyTheme(id) => {
                let id_clone = id.clone();
                let sender_clone = sender.clone();
                std::thread::spawn(move || {
                    let _ = Command::new("inerium-theme").arg("set").arg(&id_clone).status();
                    sender_clone.input(AppearanceMsg::RefreshThemes);
                });
            }
            AppearanceMsg::RefreshThemes => {
                self.themes = query_themes();
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
            }
            AppearanceMsg::RequestDeleteWallpaper(path_str) => {
                self.delete_confirm_path = Some(path_str);
            }
            AppearanceMsg::CancelDeleteWallpaper => {
                self.delete_confirm_path = None;
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
                            .join(format!("{}.84x48.png", file_name));
                        let _ = std::fs::remove_file(thumb);
                    }
                    self.wallpapers = hw::list_available_wallpapers();
                }
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
                dialog.open_multiple(
                    gtk4::Window::NONE,
                    gio::Cancellable::NONE,
                    move |res| {
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
                self.wallpapers = hw::list_available_wallpapers();
            }
            AppearanceMsg::ToggleCustomBg => {
                self.show_custom_bg = !self.show_custom_bg;
            }
            AppearanceMsg::SetBgMode(mode) => {
                self.custom_bg_mode = mode;
                self.bg_state.borrow_mut().mode = mode;
                self.bg_widgets.borrow_mut().bg_controls_mode = None; // Rebuild control layout for new mode
                if let Some(ref p) = self.bg_widgets.borrow().preview_area {
                    p.queue_draw();
                }
            }
            AppearanceMsg::SetBgColorStart(c) => {
                self.custom_color_start = c;
                self.bg_state.borrow_mut().color_start = c;
                let w = self.bg_widgets.borrow();
                if let Some(ref e) = w.entry_start
                    && parse_hex_color(&e.text()) != Some(c)
                {
                    e.set_text(&format_hex_color(c));
                }
                if let Some(ref r) = w.r_scale
                    && (r.value().round() as u8) != c.0
                {
                    r.set_value(c.0 as f64);
                }
                if let Some(ref g) = w.g_scale
                    && (g.value().round() as u8) != c.1
                {
                    g.set_value(c.1 as f64);
                }
                if let Some(ref b) = w.b_scale
                    && (b.value().round() as u8) != c.2
                {
                    b.set_value(c.2 as f64);
                }
                if let Some(ref sw) = w.swatch_start {
                    sw.queue_draw();
                }
                if let Some(ref p) = w.preview_area {
                    p.queue_draw();
                }
            }
            AppearanceMsg::SetBgColorEnd(c) => {
                self.custom_color_end = c;
                self.bg_state.borrow_mut().color_end = c;
                let w = self.bg_widgets.borrow();
                if let Some(ref e) = w.entry_end
                    && parse_hex_color(&e.text()) != Some(c)
                {
                    e.set_text(&format_hex_color(c));
                }
                if let Some(ref sw) = w.swatch_end {
                    sw.queue_draw();
                }
                if let Some(ref p) = w.preview_area {
                    p.queue_draw();
                }
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
                let w = self.bg_widgets.borrow();
                if let Some(ref e1) = w.entry_start {
                    e1.set_text(&format_hex_color(c1));
                }
                if let Some(ref e2) = w.entry_end {
                    e2.set_text(&format_hex_color(c2));
                }
                if let Some(ref sw1) = w.swatch_start {
                    sw1.queue_draw();
                }
                if let Some(ref sw2) = w.swatch_end {
                    sw2.queue_draw();
                }
                if let Some(ref p) = w.preview_area {
                    p.queue_draw();
                }
            }
            AppearanceMsg::SetBgDirection(dir) => {
                self.custom_direction = dir;
                self.bg_state.borrow_mut().direction = dir;
                if let Some(ref p) = self.bg_widgets.borrow().preview_area {
                    p.queue_draw();
                }
            }
            AppearanceMsg::ApplyCustomBg => {
                let spec = BackgroundSpec {
                    mode: self.custom_bg_mode,
                    color_start: self.custom_color_start,
                    color_end: self.custom_color_end,
                    direction: self.custom_direction,
                };
                let dir = hw::inerium_wallpapers_dir();
                if let Ok(path) = spec.generate_and_save(&dir) {
                    let path_str = path.to_string_lossy().to_string();
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
                        self.active_wallpapers.insert(self.selected_monitor.clone(), path_str);
                    }
                    self.wallpapers = hw::list_available_wallpapers();
                }
            }
            AppearanceMsg::SaveCustomBg => {
                let spec = BackgroundSpec {
                    mode: self.custom_bg_mode,
                    color_start: self.custom_color_start,
                    color_end: self.custom_color_end,
                    direction: self.custom_direction,
                };
                let dir = hw::inerium_wallpapers_dir();
                if let Ok(path) = spec.generate_and_save(&dir) {
                    let path_str = path.to_string_lossy().to_string();
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
                    self.wallpapers = hw::list_available_wallpapers();

                    let notify_title = tr("Wallpaper Saved", "Фон сохранен");
                    let notify_body = tr("Background added to wallpapers folder", "Фон добавлен в папку обоев");
                    let _ = Command::new("notify-send")
                        .args(["-a", "Обои", "-i", &path_str, notify_title, notify_body])
                        .output();
                }
            }
            AppearanceMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

fn populate_theme_list(list_box: &GtkBox, themes: &[ThemeItem], sender: &ComponentSender<AppearanceModel>) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for item in themes {
        let row = Button::new();
        row.add_css_class("theme-card");
        if item.is_active {
            row.add_css_class("active");
        }

        let inner = GtkBox::new(Orientation::Horizontal, 8);
        inner.set_hexpand(true);

        let ico_str = if item.variant == "light" { "󰔏" } else { "󰔎" };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        if item.is_active {
            ico.add_css_class("active");
        }
        inner.append(&ico);

        let txt = GtkBox::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        title_box.append(&name_lbl);

        let variant_str = if item.variant.to_lowercase() == "light" { "LIGHT" } else { "DARK" };
        let variant_badge = Label::new(Some(variant_str));
        variant_badge.add_css_class("theme-variant-pill");
        variant_badge.set_xalign(0.0);
        title_box.append(&variant_badge);
        txt.append(&title_box);

        let sub_text = if !item.description.is_empty() {
            format!("{} • {}", item.author, item.description)
        } else {
            format!("by {}", item.author)
        };
        let desc_lbl = Label::new(Some(&sub_text));
        desc_lbl.add_css_class("card-sub");
        desc_lbl.set_xalign(0.0);
        desc_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&desc_lbl);
        inner.append(&txt);

        // Theme palette dots preview in high-contrast tray
        let palette_box = GtkBox::new(Orientation::Horizontal, 4);
        palette_box.add_css_class("theme-palette-tray");
        palette_box.set_valign(gtk4::Align::Center);
        let labels = [
            tr("Background", "Фон"),
            tr("Surface", "Панели"),
            tr("Accent", "Акцент"),
            tr("Foreground", "Текст"),
        ];
        for (i, col_str) in [&item.bg, &item.surface, &item.accent, &item.fg].iter().enumerate() {
            if let Some((r, g, b)) = parse_hex_color(col_str) {
                let dot = make_color_swatch_static(r, g, b, 12, 12, 6.0);
                dot.set_tooltip_text(Some(&format!("{}: {}", labels[i], col_str)));
                palette_box.append(&dot);
            }
        }
        inner.append(&palette_box);

        if item.is_active {
            let badge = Label::new(Some(tr("󰄬  Active", "󰄬  Активна")));
            badge.add_css_class("card-badge");
            badge.add_css_class("active");
            badge.set_valign(gtk4::Align::Center);
            inner.append(&badge);
        }

        row.set_child(Some(&inner));

        let id = item.id.clone();
        let s = sender.clone();
        row.connect_clicked(move |_| {
            s.input(AppearanceMsg::ApplyTheme(id.clone()));
        });

        list_box.append(&row);
    }
}

fn populate_monitor_pills(box_widget: &GtkBox, model: &AppearanceModel, sender: &ComponentSender<AppearanceModel>) {
    while let Some(child) = box_widget.first_child() {
        box_widget.remove(&child);
    }

    if !model.has_multiple_monitors {
        return;
    }

    let all_btn = Button::with_label(tr("All Monitors", "Все экраны"));
    all_btn.add_css_class("filter-pill");
    if model.selected_monitor == "all" {
        all_btn.add_css_class("active");
    }
    let s = sender.clone();
    all_btn.connect_clicked(move |_| {
        s.input(AppearanceMsg::SelectMonitor("all".to_string()));
    });
    box_widget.append(&all_btn);

    for m in &model.connected_monitors {
        let name = m.name.clone();
        let btn = Button::with_label(&m.label);
        btn.add_css_class("filter-pill");
        if model.selected_monitor == name {
            btn.add_css_class("active");
        }
        let s2 = sender.clone();
        let n_clone = name.clone();
        btn.connect_clicked(move |_| {
            s2.input(AppearanceMsg::SelectMonitor(n_clone.clone()));
        });
        box_widget.append(&btn);
    }
}

fn populate_custom_bg_controls(
    container: &GtkBox,
    model: &AppearanceModel,
    sender: &ComponentSender<AppearanceModel>,
) {
    // Only rebuild if the mode changed or if first run
    {
        let w = model.bg_widgets.borrow();
        if w.bg_controls_mode == Some(model.custom_bg_mode) && container.first_child().is_some() {
            return;
        }
    }
    model.bg_widgets.borrow_mut().bg_controls_mode = Some(model.custom_bg_mode);

    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // --- Header Row ---
    let header_row = GtkBox::new(Orientation::Horizontal, 6);
    let title_lbl = Label::new(Some(tr("󰸌  Background Studio", "󰸌  Свой фон (генератор)")));
    title_lbl.add_css_class("cc-label");
    title_lbl.set_hexpand(true);
    title_lbl.set_xalign(0.0);
    header_row.append(&title_lbl);

    let btn_close = Button::with_label("󰅖");
    btn_close.add_css_class("wall-del-btn");
    btn_close.set_tooltip_text(Some(tr("Close", "Закрыть")));
    let s = sender.clone();
    btn_close.connect_clicked(move |_| {
        s.input(AppearanceMsg::ToggleCustomBg);
    });
    header_row.append(&btn_close);
    container.append(&header_row);

    // --- Live Cairo Background Preview Area ---
    let preview = DrawingArea::new();
    preview.set_content_height(75);
    preview.set_hexpand(true);
    preview.add_css_class("bg-preview-area");

    let st_preview = model.bg_state.clone();
    preview.set_draw_func(move |_area, cr, w, h| {
        let w = w as f64;
        let h = h as f64;
        let r = 8.0;

        cr.new_sub_path();
        cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        cr.close_path();
        cr.clip();

        let st = st_preview.borrow();
        match st.mode {
            BgMode::Solid => {
                let c = st.color_start;
                cr.set_source_rgb(
                    c.0 as f64 / 255.0,
                    c.1 as f64 / 255.0,
                    c.2 as f64 / 255.0,
                );
                let _ = cr.paint();
            }
            BgMode::Gradient => {
                let c_start = st.color_start;
                let c_end = st.color_end;
                match st.direction {
                    GradientDirection::Vertical | GradientDirection::Horizontal | GradientDirection::Diagonal => {
                        let (x0, y0, x1, y1) = match st.direction {
                            GradientDirection::Vertical => (0.0, 0.0, 0.0, h),
                            GradientDirection::Horizontal => (0.0, 0.0, w, 0.0),
                            _ => (0.0, 0.0, w, h),
                        };
                        let pat = gtk4::cairo::LinearGradient::new(x0, y0, x1, y1);
                        pat.add_color_stop_rgb(
                            0.0,
                            c_start.0 as f64 / 255.0,
                            c_start.1 as f64 / 255.0,
                            c_start.2 as f64 / 255.0,
                        );
                        pat.add_color_stop_rgb(
                            1.0,
                            c_end.0 as f64 / 255.0,
                            c_end.1 as f64 / 255.0,
                            c_end.2 as f64 / 255.0,
                        );
                        let _ = cr.set_source(&pat);
                        let _ = cr.paint();
                    }
                    GradientDirection::Radial => {
                        let rad = (w * w + h * h).sqrt() / 2.0;
                        let pat = gtk4::cairo::RadialGradient::new(w / 2.0, h / 2.0, 0.0, w / 2.0, h / 2.0, rad);
                        pat.add_color_stop_rgb(
                            0.0,
                            c_start.0 as f64 / 255.0,
                            c_start.1 as f64 / 255.0,
                            c_start.2 as f64 / 255.0,
                        );
                        pat.add_color_stop_rgb(
                            1.0,
                            c_end.0 as f64 / 255.0,
                            c_end.1 as f64 / 255.0,
                            c_end.2 as f64 / 255.0,
                        );
                        let _ = cr.set_source(&pat);
                        let _ = cr.paint();
                    }
                }
            }
        }
    });
    container.append(&preview);
    model.bg_widgets.borrow_mut().preview_area = Some(preview);

    // --- Mode Selector Row ---
    let mode_seg = GtkBox::new(Orientation::Horizontal, 4);
    mode_seg.add_css_class("cc-seg");

    let btn_solid = Button::with_label(tr("󰔏  Solid Color", "󰔏  Сплошной"));
    btn_solid.set_hexpand(true);
    if model.custom_bg_mode == BgMode::Solid {
        btn_solid.add_css_class("active");
    }
    let s = sender.clone();
    btn_solid.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Solid));
    });
    mode_seg.append(&btn_solid);

    let btn_gradient = Button::with_label(tr("󰔎  Gradient", "󰔎  Градиент"));
    btn_gradient.set_hexpand(true);
    if model.custom_bg_mode == BgMode::Gradient {
        btn_gradient.add_css_class("active");
    }
    let s = sender.clone();
    btn_gradient.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Gradient));
    });
    mode_seg.append(&btn_gradient);
    container.append(&mode_seg);

    // --- Inline Color Controls ---
    match model.custom_bg_mode {
        BgMode::Solid => {
            // Presets palette
            let presets_box = GtkBox::new(Orientation::Vertical, 4);
            let lbl_p = Label::new(Some(tr("Presets:", "Пресеты:")));
            lbl_p.add_css_class("cc-label");
            lbl_p.set_xalign(0.0);
            presets_box.append(&lbl_p);

            let dots_row = GtkBox::new(Orientation::Horizontal, 6);
            let presets = [
                ("Sumi", (0x18, 0x16, 0x16)),
                ("Obsidian", (0x0d, 0x0e, 0x11)),
                ("Slate", (0x1e, 0x20, 0x24)),
                ("Navy", (0x0f, 0x17, 0x2a)),
                ("Forest", (0x0d, 0x22, 0x18)),
                ("Wine", (0x23, 0x12, 0x26)),
                ("Earth", (0x26, 0x1d, 0x1a)),
                ("Lotus", (0xf2, 0xec, 0xbc)),
                ("White", (0xff, 0xff, 0xff)),
                ("Black", (0x00, 0x00, 0x00)),
            ];

            for (tip, col) in presets {
                let dot_btn = Button::new();
                dot_btn.add_css_class("color-preset-btn");
                if model.custom_color_start == col {
                    dot_btn.add_css_class("active");
                }
                dot_btn.set_tooltip_text(Some(tip));
                let swatch = make_color_swatch_static(col.0, col.1, col.2, 22, 22, 11.0);
                dot_btn.set_child(Some(&swatch));
                let s = sender.clone();
                dot_btn.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgColorStart(col));
                });
                dots_row.append(&dot_btn);
            }
            presets_box.append(&dots_row);
            container.append(&presets_box);

            // Hex Input and Direct Swatch Row
            let hex_row = GtkBox::new(Orientation::Horizontal, 8);
            let lbl_hex = Label::new(Some(tr("Custom Color:", "Свой цвет:")));
            lbl_hex.add_css_class("cc-label");
            hex_row.append(&lbl_hex);

            let cur_col = model.custom_color_start;
            let swatch_start = DrawingArea::new();
            swatch_start.set_content_width(28);
            swatch_start.set_content_height(28);
            swatch_start.add_css_class("color-swatch-box");
            let st_swatch = model.bg_state.clone();
            swatch_start.set_draw_func(move |_area, cr, w, h| {
                let c = st_swatch.borrow().color_start;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            hex_row.append(&swatch_start);
            model.bg_widgets.borrow_mut().swatch_start = Some(swatch_start);

            let entry = Entry::new();
            entry.add_css_class("hex-input");
            entry.set_text(&format_hex_color(cur_col));
            entry.set_max_length(7);
            entry.set_width_chars(8);
            let s = sender.clone();
            entry.connect_changed(move |e| {
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s.input(AppearanceMsg::SetBgColorStart(c));
                }
            });
            hex_row.append(&entry);
            model.bg_widgets.borrow_mut().entry_start = Some(entry);

            // RGB adjustment sliders
            let rgb_box = GtkBox::new(Orientation::Vertical, 2);
            rgb_box.set_hexpand(true);

            let r_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            r_scale.add_css_class("cc-scale");
            r_scale.set_value(cur_col.0 as f64);
            r_scale.set_hexpand(true);
            r_scale.set_draw_value(false);
            let s_r = sender.clone();
            let st_r = model.bg_state.clone();
            r_scale.connect_value_changed(move |sc| {
                let v = sc.value().round() as u8;
                let cur = st_r.borrow().color_start;
                if cur.0 != v {
                    s_r.input(AppearanceMsg::SetBgColorStart((v, cur.1, cur.2)));
                }
            });
            let r_row = GtkBox::new(Orientation::Horizontal, 4);
            let r_lbl = Label::new(Some("R"));
            r_lbl.add_css_class("cc-label");
            r_lbl.set_width_chars(2);
            r_row.append(&r_lbl);
            r_row.append(&r_scale);
            rgb_box.append(&r_row);
            model.bg_widgets.borrow_mut().r_scale = Some(r_scale);

            let g_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            g_scale.add_css_class("cc-scale");
            g_scale.set_value(cur_col.1 as f64);
            g_scale.set_hexpand(true);
            g_scale.set_draw_value(false);
            let s_g = sender.clone();
            let st_g = model.bg_state.clone();
            g_scale.connect_value_changed(move |sc| {
                let v = sc.value().round() as u8;
                let cur = st_g.borrow().color_start;
                if cur.1 != v {
                    s_g.input(AppearanceMsg::SetBgColorStart((cur.0, v, cur.2)));
                }
            });
            let g_row = GtkBox::new(Orientation::Horizontal, 4);
            let g_lbl = Label::new(Some("G"));
            g_lbl.add_css_class("cc-label");
            g_lbl.set_width_chars(2);
            g_row.append(&g_lbl);
            g_row.append(&g_scale);
            rgb_box.append(&g_row);
            model.bg_widgets.borrow_mut().g_scale = Some(g_scale);

            let b_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
            b_scale.add_css_class("cc-scale");
            b_scale.set_value(cur_col.2 as f64);
            b_scale.set_hexpand(true);
            b_scale.set_draw_value(false);
            let s_b = sender.clone();
            let st_b = model.bg_state.clone();
            b_scale.connect_value_changed(move |sc| {
                let v = sc.value().round() as u8;
                let cur = st_b.borrow().color_start;
                if cur.2 != v {
                    s_b.input(AppearanceMsg::SetBgColorStart((cur.0, cur.1, v)));
                }
            });
            let b_row = GtkBox::new(Orientation::Horizontal, 4);
            let b_lbl = Label::new(Some("B"));
            b_lbl.add_css_class("cc-label");
            b_lbl.set_width_chars(2);
            b_row.append(&b_lbl);
            b_row.append(&b_scale);
            rgb_box.append(&b_row);
            model.bg_widgets.borrow_mut().b_scale = Some(b_scale);

            hex_row.append(&rgb_box);
            container.append(&hex_row);
        }
        BgMode::Gradient => {
            let colors_row = GtkBox::new(Orientation::Horizontal, 8);

            // Start Color Box
            let start_box = GtkBox::new(Orientation::Horizontal, 4);
            let lbl_from = Label::new(Some(tr("From:", "От:")));
            lbl_from.add_css_class("cc-label");
            start_box.append(&lbl_from);

            let c1 = model.custom_color_start;
            let swatch1 = DrawingArea::new();
            swatch1.set_content_width(24);
            swatch1.set_content_height(24);
            swatch1.add_css_class("color-swatch-box");
            let st_sw1 = model.bg_state.clone();
            swatch1.set_draw_func(move |_area, cr, w, h| {
                let c = st_sw1.borrow().color_start;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            start_box.append(&swatch1);
            model.bg_widgets.borrow_mut().swatch_start = Some(swatch1);

            let entry1 = Entry::new();
            entry1.add_css_class("hex-input");
            entry1.set_text(&format_hex_color(c1));
            entry1.set_max_length(7);
            entry1.set_width_chars(8);
            let s1 = sender.clone();
            entry1.connect_changed(move |e| {
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s1.input(AppearanceMsg::SetBgColorStart(c));
                }
            });
            start_box.append(&entry1);
            model.bg_widgets.borrow_mut().entry_start = Some(entry1);
            colors_row.append(&start_box);

            // Swap Colors Button
            let btn_swap = Button::with_label("⇄");
            btn_swap.add_css_class("filter-pill");
            btn_swap.set_tooltip_text(Some(tr("Swap Colors", "Поменять цвета местами")));
            let s_swap = sender.clone();
            btn_swap.connect_clicked(move |_| {
                s_swap.input(AppearanceMsg::SwapBgColors);
            });
            colors_row.append(&btn_swap);

            // End Color Box
            let end_box = GtkBox::new(Orientation::Horizontal, 4);
            let lbl_to = Label::new(Some(tr("To:", "До:")));
            lbl_to.add_css_class("cc-label");
            end_box.append(&lbl_to);

            let c2 = model.custom_color_end;
            let swatch2 = DrawingArea::new();
            swatch2.set_content_width(24);
            swatch2.set_content_height(24);
            swatch2.add_css_class("color-swatch-box");
            let st_sw2 = model.bg_state.clone();
            swatch2.set_draw_func(move |_area, cr, w, h| {
                let c = st_sw2.borrow().color_end;
                let w = w as f64;
                let h = h as f64;
                let r = 6.0;
                cr.new_sub_path();
                cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r, r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.clip();
                cr.set_source_rgb(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0);
                let _ = cr.paint();
            });
            end_box.append(&swatch2);
            model.bg_widgets.borrow_mut().swatch_end = Some(swatch2);

            let entry2 = Entry::new();
            entry2.add_css_class("hex-input");
            entry2.set_text(&format_hex_color(c2));
            entry2.set_max_length(7);
            entry2.set_width_chars(8);
            let s2 = sender.clone();
            entry2.connect_changed(move |e| {
                let text = e.text();
                if let Some(c) = parse_hex_color(&text) {
                    s2.input(AppearanceMsg::SetBgColorEnd(c));
                }
            });
            end_box.append(&entry2);
            model.bg_widgets.borrow_mut().entry_end = Some(entry2);
            colors_row.append(&end_box);

            container.append(&colors_row);

            // Direction row
            let dir_row = GtkBox::new(Orientation::Horizontal, 6);
            let lbl_dir = Label::new(Some(tr("Direction:", "Направление:")));
            lbl_dir.add_css_class("cc-label");
            dir_row.append(&lbl_dir);

            let dirs = [
                (GradientDirection::Vertical, "↓", tr("Vertical (down)", "Сверху вниз")),
                (GradientDirection::Horizontal, "→", tr("Horizontal (right)", "Слева направо")),
                (GradientDirection::Diagonal, "↘", tr("Diagonal", "По диагонали")),
                (GradientDirection::Radial, "◉", tr("Radial", "Радиальный")),
            ];
            for (dir_spec, symbol, tip) in dirs {
                let btn_d = Button::with_label(symbol);
                btn_d.add_css_class("cc-chip");
                btn_d.set_tooltip_text(Some(tip));
                if model.custom_direction == dir_spec {
                    btn_d.add_css_class("active");
                }
                let s = sender.clone();
                btn_d.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgDirection(dir_spec));
                });
                dir_row.append(&btn_d);
            }
            container.append(&dir_row);

            // Gradient Presets
            let presets_row = GtkBox::new(Orientation::Horizontal, 5);
            let lbl_presets = Label::new(Some(tr("Presets:", "Пресеты:")));
            lbl_presets.add_css_class("cc-label");
            presets_row.append(&lbl_presets);

            let grad_presets = [
                ("Kanagawa", (0x18, 0x16, 0x16), (0x28, 0x27, 0x28)),
                ("Obsidian", (0x0a, 0x0a, 0x0c), (0x1a, 0x1a, 0x24)),
                ("Nordic", (0x1e, 0x23, 0x2a), (0x38, 0x43, 0x4f)),
                ("Sunset", (0x1c, 0x10, 0x24), (0x3d, 0x1a, 0x33)),
                ("Cyber", (0x0a, 0x19, 0x2f), (0x17, 0x2a, 0x45)),
                ("Emerald", (0x0a, 0x1c, 0x14), (0x16, 0x38, 0x28)),
                ("Lotus", (0xf2, 0xec, 0xbc), (0xde, 0xd7, 0xbc)),
            ];
            for (name, c_start, c_end) in grad_presets {
                let chip = Button::with_label(name);
                chip.add_css_class("cc-chip");
                if model.custom_color_start == c_start && model.custom_color_end == c_end {
                    chip.add_css_class("active");
                }
                let s = sender.clone();
                chip.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgColorStart(c_start));
                    s.input(AppearanceMsg::SetBgColorEnd(c_end));
                });
                presets_row.append(&chip);
            }
            container.append(&presets_row);
        }
    }

    // --- Action row: Apply & Save ---
    let action_row = GtkBox::new(Orientation::Horizontal, 8);

    let btn_apply = Button::with_label(tr("󰄬  Apply Background", "󰄬  Применить фон"));
    btn_apply.add_css_class("btn-primary-action");
    btn_apply.set_hexpand(true);
    let s = sender.clone();
    btn_apply.connect_clicked(move |_| {
        s.input(AppearanceMsg::ApplyCustomBg);
    });
    action_row.append(&btn_apply);

    let btn_save = Button::with_label(tr("󰆓  Save to Wallpapers", "󰆓  Сохранить в обои"));
    btn_save.add_css_class("cc-chip");
    btn_save.set_hexpand(true);
    let s = sender.clone();
    btn_save.connect_clicked(move |_| {
        s.input(AppearanceMsg::SaveCustomBg);
    });
    action_row.append(&btn_save);

    container.append(&action_row);
}

fn populate_wallpapers_strip(strip: &GtkBox, model: &AppearanceModel, sender: &ComponentSender<AppearanceModel>) {
    while let Some(child) = strip.first_child() {
        strip.remove(&child);
    }

    if model.wallpapers.is_empty() {
        let empty = Label::new(Some(tr("No wallpapers in folder", "Нет обоев в папке")));
        empty.add_css_class("card-sub");
        strip.append(&empty);
        return;
    }

    let active_wall = if model.selected_monitor == "all" {
        model.connected_monitors.first().and_then(|m| model.active_wallpapers.get(&m.name))
    } else {
        model.active_wallpapers.get(&model.selected_monitor)
    };

    let inerium_dir = hw::inerium_wallpapers_dir();
    let inerium_dir_str = inerium_dir.to_str().unwrap_or("").to_string();

    for item in &model.wallpapers {
        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class("wall-card");

        let is_active = active_wall.map(|w| w == &item.path).unwrap_or(false);
        if is_active {
            card.add_css_class("active");
        }

        // Thumbnail Picture wrapped in clickable button
        let pic = if let Some(thumb) = &item.thumb_path {
            Picture::for_filename(thumb)
        } else {
            let file = gio::File::for_path(&item.path);
            if let Ok(tex) = gdk4::Texture::from_file(&file) {
                Picture::for_paintable(&tex)
            } else {
                Picture::for_filename(&item.path)
            }
        };
        pic.set_content_fit(gtk4::ContentFit::Cover);
        pic.set_size_request(130, 72);
        pic.set_can_shrink(true);
        pic.add_css_class("wall-thumb");

        let top_btn = Button::new();
        top_btn.add_css_class("wall-btn");
        top_btn.set_child(Some(&pic));

        let s_apply = sender.clone();
        let path_apply = item.path.clone();
        top_btn.connect_clicked(move |_| {
            s_apply.input(AppearanceMsg::ApplyWallpaper(path_apply.clone()));
        });
        card.append(&top_btn);

        // Bottom label and clean delete action with confirmation
        let bottom_row = GtkBox::new(Orientation::Horizontal, 4);

        if model.delete_confirm_path.as_deref() == Some(&item.path) {
            let lbl_confirm = Label::new(Some(tr("Delete?", "Удалить?")));
            lbl_confirm.add_css_class("wall-name");
            lbl_confirm.set_hexpand(true);
            lbl_confirm.set_xalign(0.0);
            bottom_row.append(&lbl_confirm);

            let btn_yes = Button::with_label(tr("󰄬 Yes", "󰄬 Да"));
            btn_yes.add_css_class("wall-del-confirm");
            let s_del = sender.clone();
            let path_del = item.path.clone();
            btn_yes.connect_clicked(move |_| {
                s_del.input(AppearanceMsg::ConfirmDeleteWallpaper(path_del.clone()));
            });
            bottom_row.append(&btn_yes);

            let btn_no = Button::with_label(tr("󰅖 No", "󰅖 Нет"));
            btn_no.add_css_class("wall-del-cancel");
            let s_cancel = sender.clone();
            btn_no.connect_clicked(move |_| {
                s_cancel.input(AppearanceMsg::CancelDeleteWallpaper);
            });
            bottom_row.append(&btn_no);
        } else {
            let name_lbl = Label::new(Some(&item.name));
            name_lbl.add_css_class("wall-name");
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            name_lbl.set_hexpand(true);
            name_lbl.set_xalign(0.0);

            let click_label = GestureClick::new();
            let s_lbl = sender.clone();
            let path_lbl = item.path.clone();
            click_label.connect_released(move |_, _, _, _| {
                s_lbl.input(AppearanceMsg::ApplyWallpaper(path_lbl.clone()));
            });
            name_lbl.add_controller(click_label);
            bottom_row.append(&name_lbl);

            let is_user_deletable = !inerium_dir_str.is_empty() && item.path.starts_with(&inerium_dir_str);
            if is_user_deletable {
                let del_btn = Button::with_label("󰆴");
                del_btn.add_css_class("wall-del-btn");
                del_btn.set_tooltip_text(Some(tr("Delete from folder", "Удалить из папки")));
                let s_req = sender.clone();
                let path_req = item.path.clone();
                del_btn.connect_clicked(move |_| {
                    s_req.input(AppearanceMsg::RequestDeleteWallpaper(path_req.clone()));
                });
                bottom_row.append(&del_btn);
            }
        }
        card.append(&bottom_row);
        strip.append(&card);
    }
}

pub fn build_appearance_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
    current_monitor: Option<String>,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = AppearanceInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
        current_monitor,
    };

    let mut controller = AppearanceModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 455, 640)
}
