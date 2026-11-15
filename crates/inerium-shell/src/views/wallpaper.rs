use std::collections::HashMap;
use std::path::Path;

use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{Button, FileDialog, FileFilter, Label, Orientation, Picture, PolicyType};
use inerium_i18n::{tr, tr_fmt};
use relm4::prelude::*;

use crate::hw;

pub struct WallpaperModel {
    pub selected_target: String,
    pub connected_monitors: Vec<hw::MonitorTarget>,
    pub active_wallpapers: HashMap<String, String>,
    pub wallpapers: Vec<hw::WallpaperItem>,
    pub has_multiple_monitors: bool,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum WallpaperMsg {
    SelectTarget(String),
    ApplyWallpaper(String),
    DeleteWallpaper(String),
    AddWallpapersDialog,
    WallpapersAdded,
    Back,
}

pub struct WallpaperInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
    pub current_monitor: Option<String>,
}

#[relm4::component(pub)]
impl SimpleComponent for WallpaperModel {
    type Input = WallpaperMsg;
    type Output = ();
    type Init = WallpaperInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (420, -1),

            // Header (Back, Title & Path, + Add button)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: tr("‹ Back", "‹ Назад"),
                    connect_clicked => WallpaperMsg::Back,
                },

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 2,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "cc-host",
                        set_xalign: 0.0,
                        set_label: tr("󰸉 Wallpaper Gallery", "󰸉 Галерея обоев"),
                    },

                    gtk4::Label {
                        add_css_class: "cc-meta",
                        set_xalign: 0.0,
                        set_label: "~/.local/share/inerium/wallpapers",
                    },
                },

                gtk4::Button {
                    add_css_class: "btn-secondary",
                    set_label: tr("+ Add", "+ Добавить"),
                    set_tooltip_text: Some(tr(
                        "Add custom wallpapers from files",
                        "Добавить свои обои из файлов",
                    )),
                    connect_clicked => WallpaperMsg::AddWallpapersDialog,
                },
            },

            // Multi-monitor segmented buttons bar (visible if >1 monitor)
            #[name = "seg_box"]
            gtk4::Box {
                add_css_class: "cc-seg",
                set_orientation: Orientation::Horizontal,
                set_spacing: 2,
                set_homogeneous: true,
                #[watch]
                set_visible: model.has_multiple_monitors,
            },

            // Horizontal Scrolled Gallery
            gtk4::ScrolledWindow {
                set_hscrollbar_policy: PolicyType::Automatic,
                set_vscrollbar_policy: PolicyType::Never,
                set_kinetic_scrolling: true,
                set_overlay_scrolling: true,
                set_hexpand: true,
                set_size_request: (-1, 150),

                #[name = "gallery_box"]
                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_top: 4,
                    set_margin_bottom: 4,
                    set_margin_start: 4,
                    set_margin_end: 4,
                },
            },

            // Status / Tip label
            #[name = "tip_lbl"]
            gtk4::Label {
                add_css_class: "cc-meta",
                set_xalign: 0.5,
                set_margin_top: 2,
                #[watch]
                set_label: &{
                    if model.has_multiple_monitors {
                        if model.selected_target == "all" {
                            tr(
                                "All screens · Click a card to apply",
                                "Все экраны · Кликните на карточку для применения",
                            ).to_string()
                        } else {
                            let label = model
                                .connected_monitors
                                .iter()
                                .find(|m| m.name == model.selected_target)
                                .map(|m| m.label.as_str())
                                .unwrap_or(&model.selected_target);
                            tr_fmt!(
                                "Screen {label} · Click a card to apply",
                                "Экран {label} · Кликните на карточку для применения",
                                label = label,
                            )
                        }
                    } else {
                        tr(
                            "Click a card to apply wallpaper instantly",
                            "Кликните на карточку для мгновенного применения обоев",
                        ).to_string()
                    }
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let connected_monitors = hw::list_connected_monitors();
        let has_multiple_monitors = connected_monitors.len() > 1;

        let default_target = if let Some(ref cur) = init.current_monitor {
            if connected_monitors.iter().any(|m| &m.name == cur) {
                cur.clone()
            } else if has_multiple_monitors {
                "all".to_string()
            } else if let Some(first) = connected_monitors.first() {
                first.name.clone()
            } else {
                "all".to_string()
            }
        } else if has_multiple_monitors {
            "all".to_string()
        } else if let Some(first) = connected_monitors.first() {
            first.name.clone()
        } else {
            "all".to_string()
        };

        let active_wallpapers = hw::get_active_wallpapers();
        let wallpapers = hw::list_available_wallpapers();

        let model = WallpaperModel {
            selected_target: default_target,
            connected_monitors,
            active_wallpapers,
            wallpapers,
            has_multiple_monitors,
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        populate_seg_bar(
            &widgets.seg_box,
            &model.connected_monitors,
            &model.selected_target,
            model.has_multiple_monitors,
            &sender,
        );
        populate_gallery(
            &widgets.gallery_box,
            &model.wallpapers,
            &model.selected_target,
            &model.active_wallpapers,
            &model.connected_monitors,
            &sender,
        );

        ComponentParts { model, widgets }
    }

    fn post_view() {
        populate_seg_bar(
            seg_box,
            &self.connected_monitors,
            &self.selected_target,
            self.has_multiple_monitors,
            &sender,
        );
        populate_gallery(
            gallery_box,
            &self.wallpapers,
            &self.selected_target,
            &self.active_wallpapers,
            &self.connected_monitors,
            &sender,
        );
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WallpaperMsg::SelectTarget(target) => {
                self.selected_target = target;
            }
            WallpaperMsg::ApplyWallpaper(path) => {
                if self.selected_target == "all" {
                    for m in &self.connected_monitors {
                        self.active_wallpapers.insert(m.name.clone(), path.clone());
                    }
                    hw::set_wallpaper_async(path, None);
                } else {
                    self.active_wallpapers
                        .insert(self.selected_target.clone(), path.clone());
                    hw::set_wallpaper_async(path, Some(self.selected_target.clone()));
                }
            }
            WallpaperMsg::DeleteWallpaper(path) => {
                let _ = std::fs::remove_file(&path);
                self.wallpapers = hw::list_available_wallpapers();
            }
            WallpaperMsg::AddWallpapersDialog => {
                let filter = FileFilter::new();
                filter.set_name(Some(tr(
                    "Images (*.png, *.jpg, *.jpeg, *.webp)",
                    "Изображения (*.png, *.jpg, *.jpeg, *.webp)",
                )));
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
                                if let Some(item) = files.item(i)
                                    && let Ok(file) = item.downcast::<gio::File>()
                                    && let Some(src_path) = file.path()
                                    && let Some(filename) = src_path.file_name()
                                {
                                    let target = dest_dir.join(filename);
                                    let _ = std::fs::copy(&src_path, &target);
                                }
                            }
                            s.input(WallpaperMsg::WallpapersAdded);
                        }
                    },
                );
            }
            WallpaperMsg::WallpapersAdded => {
                self.wallpapers = hw::list_available_wallpapers();
            }
            WallpaperMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

fn populate_seg_bar(
    seg_box: &gtk4::Box,
    connected_monitors: &[hw::MonitorTarget],
    selected_target: &str,
    has_multiple_monitors: bool,
    sender: &ComponentSender<WallpaperModel>,
) {
    while let Some(child) = seg_box.first_child() {
        seg_box.remove(&child);
    }

    if !has_multiple_monitors {
        return;
    }

    // "All" button
    let all_btn = Button::with_label(tr("󰍹 All", "󰍹 Все"));
    all_btn.set_tooltip_text(Some(tr(
        "Apply to all monitors",
        "Применить ко всем мониторам сразу",
    )));
    if selected_target == "all" {
        all_btn.add_css_class("active");
    }
    let s_all = sender.clone();
    all_btn.connect_clicked(move |_| {
        s_all.input(WallpaperMsg::SelectTarget("all".to_string()));
    });
    seg_box.append(&all_btn);

    // Individual monitors
    for m in connected_monitors {
        let m_btn = Button::with_label(&m.label);
        m_btn.set_tooltip_text(Some(&tr_fmt!(
            "Select monitor {name}",
            "Выбрать монитор {name}",
            name = m.name
        )));
        if selected_target == m.name {
            m_btn.add_css_class("active");
        }
        let s_m = sender.clone();
        let m_name = m.name.clone();
        m_btn.connect_clicked(move |_| {
            s_m.input(WallpaperMsg::SelectTarget(m_name.clone()));
        });
        seg_box.append(&m_btn);
    }
}

fn populate_gallery(
    gallery_box: &gtk4::Box,
    wallpapers: &[hw::WallpaperItem],
    selected_target: &str,
    active_wallpapers: &HashMap<String, String>,
    connected_monitors: &[hw::MonitorTarget],
    sender: &ComponentSender<WallpaperModel>,
) {
    while let Some(child) = gallery_box.first_child() {
        gallery_box.remove(&child);
    }

    let inerium_dir = hw::inerium_wallpapers_dir();

    if wallpapers.is_empty() {
        let empty_lbl = Label::new(Some(tr(
            "No wallpapers found in wallpaper folders",
            "Обои не найдены в папках обоев",
        )));
        empty_lbl.add_css_class("cc-meta");
        empty_lbl.set_margin_top(40);
        gallery_box.append(&empty_lbl);
        return;
    }

    // Determine active path for selected target
    let active_path_for_target = if selected_target == "all" {
        let mut common: Option<&String> = None;
        let mut all_same = true;
        for m in connected_monitors {
            if let Some(p) = active_wallpapers.get(&m.name) {
                match common {
                    None => common = Some(p),
                    Some(prev) if prev != p => {
                        all_same = false;
                        break;
                    }
                    _ => {}
                }
            }
        }
        if all_same {
            common.cloned()
        } else {
            None
        }
    } else {
        active_wallpapers.get(selected_target).cloned()
    };

    for w in wallpapers {
        let card_btn = Button::new();
        card_btn.add_css_class("wall-card");
        if Some(&w.path) == active_path_for_target.as_ref() {
            card_btn.add_css_class("active");
        }

        let card_inner = gtk4::Box::new(Orientation::Vertical, 4);

        let img_src = w.thumb_path.as_deref().unwrap_or(&w.path);
        let pic = Picture::for_filename(Path::new(img_src));
        pic.set_content_fit(gtk4::ContentFit::Contain);
        pic.set_can_shrink(true);
        pic.set_size_request(140, 80);
        pic.add_css_class("wall-thumb");
        card_inner.append(&pic);

        let cap_row = gtk4::Box::new(Orientation::Horizontal, 4);

        let mut display_name = w.name.clone();
        if display_name.len() > 14 {
            display_name.truncate(12);
            display_name.push('…');
        }
        let name_lbl = Label::new(Some(&display_name));
        name_lbl.add_css_class("wall-caption");
        name_lbl.set_hexpand(true);
        name_lbl.set_xalign(0.0);
        cap_row.append(&name_lbl);

        let is_custom = Path::new(&w.path).starts_with(&inerium_dir);
        if is_custom {
            let del_btn = Button::with_label("✕");
            del_btn.add_css_class("btn-secondary");
            del_btn.set_tooltip_text(Some(tr("Delete wallpaper", "Удалить обои")));
            let s_del = sender.clone();
            let p_del = w.path.clone();
            del_btn.connect_clicked(move |_| {
                s_del.input(WallpaperMsg::DeleteWallpaper(p_del.clone()));
            });
            cap_row.append(&del_btn);
        }

        card_inner.append(&cap_row);
        card_btn.set_child(Some(&card_inner));

        let s_card = sender.clone();
        let p_card = w.path.clone();
        card_btn.connect_clicked(move |_| {
            s_card.input(WallpaperMsg::ApplyWallpaper(p_card.clone()));
        });

        gallery_box.append(&card_btn);
    }
}

pub fn build_wallpaper_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
    current_monitor: Option<String>,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let has_multiple = hw::list_connected_monitors().len() > 1;
    let init = WallpaperInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
        current_monitor,
    };

    let mut controller = WallpaperModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    let req_h = if has_multiple { 295 } else { 265 };
    (root, 420, req_h)
}
