use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box, Button, ContentFit, FileDialog, FileFilter, Label, Orientation, Picture, PolicyType,
    ScrolledWindow, glib,
};

use crate::i18n::{Lang, tr};
use crate::model::{SetupPaths, parse_active_wallpaper, scan_wallpapers};
use crate::system::apply_wallpaper;
use crate::ui::StatusCallback;

pub fn build_wallpaper_card(
    paths: &SetupPaths,
    status_cb: &StatusCallback,
    lang: Lang,
) -> Box {
    let wall_card = Box::new(Orientation::Vertical, 8);
    wall_card.add_css_class("setup-card");

    let wall_head = Box::new(Orientation::Horizontal, 8);
    let wall_info = Box::new(Orientation::Vertical, 2);
    wall_info.set_hexpand(true);
    let wall_title = Label::new(Some(tr(
        lang,
        "Quick wallpaper picker",
        "Быстрый выбор обоев",
    )));
    wall_title.add_css_class("card-title");
    wall_title.set_xalign(0.0);
    wall_info.append(&wall_title);

    let wall_sub = Label::new(Some(tr(
        lang,
        "Source: ~/.local/share/inerium/wallpapers (applied via set-wallpaper)",
        "Источник: ~/.local/share/inerium/wallpapers (применяется через set-wallpaper)",
    )));
    wall_sub.add_css_class("card-muted");
    wall_sub.set_xalign(0.0);
    wall_info.append(&wall_sub);
    wall_head.append(&wall_info);

    let open_wall_btn = Button::builder()
        .label(tr(lang, "󰉋 Folder", "󰉋 Папка обоев"))
        .tooltip_text(tr(lang, "Open wallpapers directory", "Открыть папку с обоями"))
        .build();
    open_wall_btn.add_css_class("flat-btn");
    open_wall_btn.connect_clicked(|_| {
        let path = inerium_hw::sysfs::home_dir().join(".local/share/inerium/wallpapers");
        std::thread::spawn(move || {
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
        });
    });
    wall_head.append(&open_wall_btn);

    let add_btn = Button::builder()
        .label(tr(lang, "+ Add", "+ Добавить"))
        .tooltip_text(tr(lang, "Add wallpapers from files", "Добавить свои обои из файлов"))
        .build();
    add_btn.add_css_class("flat-btn");
    wall_head.append(&add_btn);
    wall_card.append(&wall_head);

    let wall_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Never)
        .kinetic_scrolling(true)
        .build();
    wall_scroll.set_size_request(-1, 134);

    let wall_strip = Box::new(Orientation::Horizontal, 10);
    populate_wallpaper_strip(&wall_strip, paths, status_cb, lang);

    let paths_add = paths.clone();
    let status_cb_add = status_cb.clone();
    let wall_strip_add = wall_strip.clone();
    add_btn.connect_clicked(move |_| {
        let filter = FileFilter::new();
        filter.set_name(Some(tr(
            lang,
            "Images (*.png, *.jpg, *.jpeg, *.webp)",
            "Изображения (*.png, *.jpg, *.jpeg, *.webp)",
        )));
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("image/webp");

        let filters = gio::ListStore::new::<FileFilter>();
        filters.append(&filter);

        let dialog = FileDialog::builder()
            .title(tr(lang, "Add Wallpapers", "Добавить обои"))
            .accept_label(tr(lang, "Add", "Добавить"))
            .filters(&filters)
            .modal(true)
            .build();

        let dest_dir = paths_add.inerium_wallpapers_dir();
        let status_cb_dlg = status_cb_add.clone();
        let strip = wall_strip_add.clone();
        let paths_dlg = paths_add.clone();

        dialog.open_multiple(
            Option::<&gtk4::Window>::None,
            gio::Cancellable::NONE,
            move |res| {
                if let Ok(files) = res {
                    let _ = std::fs::create_dir_all(&dest_dir);
                    let n = files.n_items();
                    let mut copied = 0;
                    for i in 0..n {
                        if let Some(item) = files.item(i)
                            && let Ok(file) = item.downcast::<gio::File>()
                            && let Some(src_path) = file.path()
                            && let Some(filename) = src_path.file_name()
                        {
                            let target = dest_dir.join(filename);
                            if std::fs::copy(&src_path, &target).is_ok() {
                                copied += 1;
                            }
                        }
                    }
                    if copied > 0 {
                        status_cb_dlg.ok(&match lang {
                            Lang::En => format!("Added {copied} wallpaper(s)"),
                            Lang::Ru => format!("Добавлено обоев: {copied}"),
                        });
                        populate_wallpaper_strip(&strip, &paths_dlg, &status_cb_dlg, lang);
                    }
                }
            },
        );
    });

    wall_scroll.set_child(Some(&wall_strip));
    wall_card.append(&wall_scroll);
    wall_card
}

fn populate_wallpaper_strip(
    wall_strip: &Box,
    paths: &SetupPaths,
    status_cb: &StatusCallback,
    lang: Lang,
) {
    while let Some(child) = wall_strip.first_child() {
        wall_strip.remove(&child);
    }

    let wallpapers = scan_wallpapers(&paths.wallpaper_dirs());
    let active_wall = std::fs::read_to_string(paths.hyprpaper_conf_path())
        .ok()
        .and_then(|s| parse_active_wallpaper(&s));

    let wall_buttons: Rc<RefCell<Vec<(PathBuf, Button)>>> = Rc::new(RefCell::new(Vec::new()));

    if wallpapers.is_empty() {
        let empty = Label::new(Some(tr(
            lang,
            "No images found in ~/.local/share/inerium/wallpapers",
            "Изображения не найдены в ~/.local/share/inerium/wallpapers",
        )));
        empty.add_css_class("card-muted");
        empty.set_margin_top(30);
        wall_strip.append(&empty);
    } else {
        for wpath in wallpapers {
            let btn = Button::new();
            btn.add_css_class("wall-card");
            if active_wall.as_ref().is_some_and(|a| a == &wpath) {
                btn.add_css_class("active");
            }

            let inner = Box::new(Orientation::Vertical, 4);
            let file = gtk4::gio::File::for_path(&wpath);
            let pic = if let Ok(tex) = gdk4::Texture::from_file(&file) {
                Picture::for_paintable(&tex)
            } else {
                Picture::for_filename(&wpath)
            };
            pic.set_content_fit(ContentFit::Cover);
            pic.set_can_shrink(true);
            pic.set_size_request(144, 82);
            pic.add_css_class("wall-thumb");
            inner.append(&pic);

            let active_dir = paths.inerium_wallpapers_dir();
            let is_extra = !wpath.starts_with(&active_dir)
                || wpath.parent() != Some(&active_dir);

            let fname = wpath
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("wallpaper");
            let short = truncate_label(fname, 18);
            let lbl = Label::new(Some(&short));
            lbl.add_css_class("wall-name");
            inner.append(&lbl);

            if is_extra {
                let badge = Label::new(Some("[extra]"));
                badge.add_css_class("card-meta");
                inner.append(&badge);
            }

            btn.set_child(Some(&inner));
            wall_buttons.borrow_mut().push((wpath.clone(), btn.clone()));
            wall_strip.append(&btn);
        }
    }

    let paths_click = paths.clone();
    for (wpath, btn) in wall_buttons.borrow().iter() {
        let wpath = wpath.clone();
        let wall_buttons = wall_buttons.clone();
        let status_cb = status_cb.clone();
        let paths = paths_click.clone();
        btn.connect_clicked(move |_| {
            for (p, b) in wall_buttons.borrow().iter() {
                if p == &wpath {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            let wpath_orig = wpath.clone();
            let status_cb = status_cb.clone();
            let active_dir = paths.inerium_wallpapers_dir();
            status_cb.warn(tr(lang, "Applying wallpaper...", "Применение обоев..."));
            glib::MainContext::default().spawn_local(async move {
                let fname = wpath_orig
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("wallpaper")
                    .to_string();
                let is_extra = !wpath_orig.starts_with(&active_dir)
                    || wpath_orig.parent() != Some(&active_dir);

                let wpath_bg = if is_extra {
                    let _ = std::fs::create_dir_all(&active_dir);
                    let target = active_dir.join(&fname);
                    let _ = std::fs::copy(&wpath_orig, &target);
                    target
                } else {
                    wpath_orig
                };

                let res = gio::spawn_blocking(move || apply_wallpaper(&wpath_bg))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task panicked: {e:?}")));
                match res {
                    Ok(()) => status_cb.ok(&match lang {
                        Lang::En => format!("Wallpaper '{fname}' applied"),
                        Lang::Ru => format!("Обои '{fname}' применены"),
                    }),
                    Err(e) => status_cb.error(&e),
                }
            });
        });
    }
}

fn truncate_label(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{collected}…")
    } else {
        collected
    }
}
