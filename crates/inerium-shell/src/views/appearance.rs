use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, FileDialog, FileFilter, Label, Orientation, Picture, PolicyType};
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
    #[allow(dead_code)]
    pub bg: String,
    #[allow(dead_code)]
    pub surface: String,
    #[allow(dead_code)]
    pub accent: String,
    #[allow(dead_code)]
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
    DeleteWallpaper(String),
    OpenWallpapersFolder,
    AddWallpapersDialog,
    WallpapersAdded,
    ToggleCustomBg,
    SetBgMode(BgMode),
    SetBgColorStart((u8, u8, u8)),
    SetBgColorEnd((u8, u8, u8)),
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
            set_size_request: (430, -1),

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
                set_min_content_height: 380,
                set_max_content_height: 520,
                set_vexpand: true,

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 12,

                    // ── Section 1: Themes ─────────────────────────
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 6,

                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                add_css_class: "cc-label",
                                set_label: &tr("THEMES", "ТЕМЫ ОФОРМЛЕНИЯ"),
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

                    // ── Section 2: Wallpapers ──────────────────────
                    gtk4::Box {
                        set_orientation: Orientation::Vertical,
                        set_spacing: 6,

                        gtk4::Box {
                            set_orientation: Orientation::Horizontal,
                            set_spacing: 8,

                            gtk4::Label {
                                add_css_class: "cc-label",
                                set_label: &tr("WALLPAPERS", "ОБОИ РАБОЧЕГО СТОЛА"),
                                set_xalign: 0.0,
                                set_hexpand: true,
                            },

                            gtk4::Button {
                                add_css_class: "cc-chip",
                                set_label: &tr("󰉋  Folder", "󰉋  Папка обоев"),
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
                            add_css_class: "cc-card",
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
                            set_min_content_height: 110,
                            set_propagate_natural_width: true,

                            #[name = "wall_strip"]
                            gtk4::Box {
                                set_orientation: Orientation::Horizontal,
                                set_spacing: 8,
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
                        .insert(self.selected_monitor.clone(), path.clone());
                }
            }
            AppearanceMsg::DeleteWallpaper(path_str) => {
                let p = Path::new(&path_str);
                if let Ok(canon_user) = p.canonicalize()
                    && let Ok(canon_dir) = hw::inerium_wallpapers_dir().canonicalize()
                    && canon_user.starts_with(canon_dir)
                {
                    let _ = std::fs::remove_file(&canon_user);
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
                    .accept_label(tr("Add", "Добавить"))
                    .filters(&filters)
                    .modal(true)
                    .build();

                crate::set_modal_dialog_active(true);
                let s = sender.clone();

                dialog.open_multiple(
                    Option::<&gtk4::Window>::None,
                    gio::Cancellable::NONE,
                    move |res| {
                        crate::set_modal_dialog_active(false);
                        if let Ok(files) = res {
                            let dest_dir = hw::inerium_wallpapers_dir();
                            let _ = std::fs::create_dir_all(&dest_dir);
                            let n = files.n_items();
                            for i in 0..n {
                                if let Some(file) = files.item(i).and_then(|item| item.downcast::<gio::File>().ok())
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
            }
            AppearanceMsg::SetBgColorStart(c) => {
                self.custom_color_start = c;
            }
            AppearanceMsg::SetBgColorEnd(c) => {
                self.custom_color_end = c;
            }
            AppearanceMsg::SetBgDirection(dir) => {
                self.custom_direction = dir;
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
        row.add_css_class("list-item-btn");

        let inner = GtkBox::new(Orientation::Horizontal, 8);
        inner.set_hexpand(true);

        let ico_str = if item.variant == "light" { "󰔏" } else { "󰔎" };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-icon");
        inner.append(&ico);

        let txt = GtkBox::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        title_box.append(&name_lbl);

        let variant_badge = Label::new(Some(&format!("[{}]", item.variant)));
        variant_badge.add_css_class("cc-meta");
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

        if item.is_active {
            let badge = Label::new(Some(tr("󰄬  Active", "󰄬  Активна")));
            badge.add_css_class("card-badge");
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
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if !model.show_custom_bg {
        container.set_visible(false);
        return;
    }
    container.set_visible(true);

    // --- Header row: Mode Switcher + Close button ---
    let header_row = GtkBox::new(Orientation::Horizontal, 6);

    let btn_solid = Button::with_label(tr("󰸌  Solid Color", "󰸌  Сплошной цвет"));
    btn_solid.add_css_class("cc-chip");
    if model.custom_bg_mode == BgMode::Solid {
        btn_solid.add_css_class("active");
    }
    let s = sender.clone();
    btn_solid.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Solid));
    });
    header_row.append(&btn_solid);

    let btn_gradient = Button::with_label(tr("󰔎  Gradient", "󰔎  Градиент"));
    btn_gradient.add_css_class("cc-chip");
    if model.custom_bg_mode == BgMode::Gradient {
        btn_gradient.add_css_class("active");
    }
    let s = sender.clone();
    btn_gradient.connect_clicked(move |_| {
        s.input(AppearanceMsg::SetBgMode(BgMode::Gradient));
    });
    header_row.append(&btn_gradient);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header_row.append(&spacer);

    let btn_close = Button::with_label("󰅖");
    btn_close.add_css_class("del-btn");
    btn_close.set_tooltip_text(Some(tr("Close", "Закрыть")));
    let s = sender.clone();
    btn_close.connect_clicked(move |_| {
        s.input(AppearanceMsg::ToggleCustomBg);
    });
    header_row.append(&btn_close);
    container.append(&header_row);

    // --- Controls Section ---
    match model.custom_bg_mode {
        BgMode::Solid => {
            let color_row = GtkBox::new(Orientation::Horizontal, 8);
            let lbl = Label::new(Some(tr("Color:", "Цвет:")));
            lbl.add_css_class("cc-label");
            color_row.append(&lbl);

            let dialog = gtk4::ColorDialog::builder()
                .title(tr("Select Color", "Выберите цвет"))
                .with_alpha(false)
                .build();
            let c = model.custom_color_start;
            let rgba = gdk4::RGBA::new(c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, 1.0);
            let color_btn = gtk4::ColorDialogButton::builder()
                .dialog(&dialog)
                .rgba(&rgba)
                .build();
            let s = sender.clone();
            color_btn.connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                let r = (rgba.red() * 255.0).round() as u8;
                let g = (rgba.green() * 255.0).round() as u8;
                let b = (rgba.blue() * 255.0).round() as u8;
                s.input(AppearanceMsg::SetBgColorStart((r, g, b)));
            });
            color_row.append(&color_btn);

            // Quick color chips
            let presets = [
                ("Obsidian", (0x10, 0x10, 0x12)),
                ("Sumi", (0x18, 0x16, 0x16)),
                ("Slate", (0x1e, 0x20, 0x24)),
                ("Navy", (0x0f, 0x17, 0x2a)),
                ("Snow", (0xf5, 0xf5, 0xf7)),
            ];
            for (name, col) in presets {
                let chip = Button::with_label(name);
                chip.add_css_class("cc-chip");
                if model.custom_color_start == col {
                    chip.add_css_class("active");
                }
                let s = sender.clone();
                chip.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgColorStart(col));
                });
                color_row.append(&chip);
            }
            container.append(&color_row);
        }
        BgMode::Gradient => {
            // Colors row: From & To
            let colors_row = GtkBox::new(Orientation::Horizontal, 8);

            let lbl_from = Label::new(Some(tr("From:", "От:")));
            lbl_from.add_css_class("cc-label");
            colors_row.append(&lbl_from);

            let dialog1 = gtk4::ColorDialog::builder()
                .title(tr("Start Color", "Начальный цвет"))
                .with_alpha(false)
                .build();
            let c1 = model.custom_color_start;
            let rgba1 = gdk4::RGBA::new(c1.0 as f32 / 255.0, c1.1 as f32 / 255.0, c1.2 as f32 / 255.0, 1.0);
            let btn1 = gtk4::ColorDialogButton::builder()
                .dialog(&dialog1)
                .rgba(&rgba1)
                .build();
            let s = sender.clone();
            btn1.connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                let r = (rgba.red() * 255.0).round() as u8;
                let g = (rgba.green() * 255.0).round() as u8;
                let b = (rgba.blue() * 255.0).round() as u8;
                s.input(AppearanceMsg::SetBgColorStart((r, g, b)));
            });
            colors_row.append(&btn1);

            let lbl_to = Label::new(Some(tr("To:", "До:")));
            lbl_to.add_css_class("cc-label");
            colors_row.append(&lbl_to);

            let dialog2 = gtk4::ColorDialog::builder()
                .title(tr("End Color", "Конечный цвет"))
                .with_alpha(false)
                .build();
            let c2 = model.custom_color_end;
            let rgba2 = gdk4::RGBA::new(c2.0 as f32 / 255.0, c2.1 as f32 / 255.0, c2.2 as f32 / 255.0, 1.0);
            let btn2 = gtk4::ColorDialogButton::builder()
                .dialog(&dialog2)
                .rgba(&rgba2)
                .build();
            let s = sender.clone();
            btn2.connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                let r = (rgba.red() * 255.0).round() as u8;
                let g = (rgba.green() * 255.0).round() as u8;
                let b = (rgba.blue() * 255.0).round() as u8;
                s.input(AppearanceMsg::SetBgColorEnd((r, g, b)));
            });
            colors_row.append(&btn2);
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
            for (dir, symbol, tip) in dirs {
                let btn_d = Button::with_label(symbol);
                btn_d.add_css_class("cc-chip");
                btn_d.set_tooltip_text(Some(tip));
                if model.custom_direction == dir {
                    btn_d.add_css_class("active");
                }
                let s = sender.clone();
                btn_d.connect_clicked(move |_| {
                    s.input(AppearanceMsg::SetBgDirection(dir));
                });
                dir_row.append(&btn_d);
            }
            container.append(&dir_row);

            // Gradient Presets row
            let presets_row = GtkBox::new(Orientation::Horizontal, 6);
            let lbl_presets = Label::new(Some(tr("Presets:", "Пресеты:")));
            lbl_presets.add_css_class("cc-label");
            presets_row.append(&lbl_presets);

            let grad_presets = [
                ("Kanagawa", (0x18, 0x16, 0x16), (0x28, 0x27, 0x28)),
                ("Obsidian", (0x0a, 0x0a, 0x0c), (0x1a, 0x1a, 0x24)),
                ("Nordic", (0x1e, 0x23, 0x2a), (0x38, 0x43, 0x4f)),
                ("Sunset", (0x1c, 0x10, 0x24), (0x3d, 0x1a, 0x33)),
                ("Cyber", (0x0a, 0x19, 0x2f), (0x17, 0x2a, 0x45)),
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

    let btn_apply = Button::with_label(tr("󰄬  Apply", "󰄬  Применить"));
    btn_apply.add_css_class("cc-chip");
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

    for item in &model.wallpapers {
        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class("wall-card");

        let is_active = active_wall.map(|w| w == &item.path).unwrap_or(false);
        if is_active {
            card.add_css_class("active");
        }

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
        pic.set_size_request(120, 68);
        pic.set_can_shrink(true);
        pic.add_css_class("wall-thumb");
        card.append(&pic);

        let bottom_row = GtkBox::new(Orientation::Horizontal, 4);
        let name_lbl = Label::new(Some(&item.name));
        name_lbl.add_css_class("wall-name");
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);
        bottom_row.append(&name_lbl);

        let del_btn = Button::with_label("󰅖");
        del_btn.add_css_class("del-btn");
        del_btn.set_tooltip_text(Some(tr("Delete from folder", "Удалить из папки")));
        let s_del = sender.clone();
        let path_del = item.path.clone();
        del_btn.connect_clicked(move |_| {
            s_del.input(AppearanceMsg::DeleteWallpaper(path_del.clone()));
        });
        bottom_row.append(&del_btn);
        card.append(&bottom_row);

        let btn = Button::new();
        btn.set_child(Some(&card));
        btn.add_css_class("wall-btn");

        let s_apply = sender.clone();
        let path_apply = item.path.clone();
        btn.connect_clicked(move |_| {
            s_apply.input(AppearanceMsg::ApplyWallpaper(path_apply.clone()));
        });

        strip.append(&btn);
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
    (root, 430, 480)
}
