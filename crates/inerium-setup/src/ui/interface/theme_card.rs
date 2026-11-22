use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, glib};

use crate::i18n::{Lang, tr};
use crate::ui::StatusCallback;

pub fn build_theme_card(status_cb: &StatusCallback, lang: Lang) -> Box {
    let theme_card = Box::new(Orientation::Vertical, 8);
    theme_card.add_css_class("setup-card");

    let theme_head = Box::new(Orientation::Horizontal, 8);
    let theme_info = Box::new(Orientation::Vertical, 2);
    theme_info.set_hexpand(true);
    let theme_title = Label::new(Some(tr(
        lang,
        "System Theme & Color Palette",
        "Тема оформления и палитра",
    )));
    theme_title.add_css_class("card-title");
    theme_title.set_xalign(0.0);
    theme_info.append(&theme_title);

    let theme_sub = Label::new(Some(tr(
        lang,
        "Single Source of Truth: synchronized across GTK, Waybar, Kitty, and apps",
        "Единый источник: синхронизирует GTK, Waybar, Kitty, портал и приложения",
    )));
    theme_sub.add_css_class("card-muted");
    theme_sub.set_xalign(0.0);
    theme_info.append(&theme_sub);

    let selected_theme = Rc::new(RefCell::new(String::new()));
    let selected_theme_name = Rc::new(RefCell::new(String::new()));
    let watch_child: Rc<RefCell<Option<std::process::Child>>> =
        Rc::new(RefCell::new(None));

    let themes_flow = gtk4::FlowBox::new();
    themes_flow.set_valign(gtk4::Align::Start);
    themes_flow.set_max_children_per_line(5);
    themes_flow.set_min_children_per_line(2);
    themes_flow.set_selection_mode(gtk4::SelectionMode::None);
    themes_flow.set_row_spacing(8);
    themes_flow.set_column_spacing(8);

    populate_setup_themes(&themes_flow, &selected_theme, &selected_theme_name, status_cb, lang);

    let new_theme_btn = Button::builder()
        .label(tr(lang, "󰐕 New", "󰐕 Новая"))
        .tooltip_text(tr(lang, "Create a new custom theme", "Создать шаблон новой темы"))
        .build();
    new_theme_btn.add_css_class("flat-btn");
    {
        let flow_c = themes_flow.clone();
        let sel_id_c = selected_theme.clone();
        let sel_name_c = selected_theme_name.clone();
        let status_c = status_cb.clone();
        new_theme_btn.connect_clicked(move |_| {
            let home = inerium_hw::sysfs::home_dir();
            let themes_dir = home.join(".config/inerium/themes");
            let _ = std::fs::create_dir_all(&themes_dir);

            let mut id = "my-theme".to_string();
            let mut counter = 2;
            while themes_dir.join(format!("{}.toml", id)).exists() {
                id = format!("my-theme-{}", counter);
                counter += 1;
            }

            let status_local = status_c.clone();
            let flow_local = flow_c.clone();
            let sel_id_local = sel_id_c.clone();
            let sel_name_local = sel_name_c.clone();
            let id_clone = id.clone();

            let out = std::process::Command::new("inerium-theme")
                .arg("new")
                .arg(&id_clone)
                .output();

            if let Ok(out) = out && out.status.success() {
                let file_path = themes_dir.join(format!("{}.toml", id_clone));
                let _ = std::process::Command::new("xdg-open").arg(&file_path).spawn();
                populate_setup_themes(&flow_local, &sel_id_local, &sel_name_local, &status_local, lang);
                status_local.ok(&match lang {
                    Lang::En => format!("Theme '{}' created and opened in editor", id_clone),
                    Lang::Ru => format!("Тема '{}' создана и открыта в редакторе", id_clone),
                });
            } else {
                status_local.warn(&match lang {
                    Lang::En => "Failed to create theme".to_string(),
                    Lang::Ru => "Не удалось создать тему".to_string(),
                });
            }
        });
    }

    let edit_theme_btn = Button::builder()
        .label(tr(lang, "󰏫 Edit", "󰏫 Изменить"))
        .tooltip_text(tr(lang, "Open current theme file in editor", "Открыть файл текущей темы в текстовом редакторе"))
        .build();
    edit_theme_btn.add_css_class("flat-btn");
    {
        let sel_id_c = selected_theme.clone();
        let status_c = status_cb.clone();
        edit_theme_btn.connect_clicked(move |_| {
            let id = sel_id_c.borrow().clone();
            if id.is_empty() {
                return;
            }
            let home = inerium_hw::sysfs::home_dir();
            let user_path = home.join(format!(".config/inerium/themes/{}.toml", id));
            let final_path = if user_path.exists() {
                user_path
            } else {
                let extra_path = home.join(format!(".local/share/inerium/themes/extra/{}.toml", id));
                let sys_path = std::path::PathBuf::from(format!("/usr/share/inerium/themes/{}.toml", id));
                if extra_path.exists() {
                    let _ = std::fs::create_dir_all(home.join(".config/inerium/themes"));
                    let _ = std::fs::copy(&extra_path, &user_path);
                    user_path
                } else if sys_path.exists() {
                    let _ = std::fs::create_dir_all(home.join(".config/inerium/themes"));
                    let _ = std::fs::copy(&sys_path, &user_path);
                    user_path
                } else {
                    user_path
                }
            };

            let _ = std::process::Command::new("xdg-open").arg(&final_path).spawn();
            status_c.ok(&match lang {
                Lang::En => format!("Opened '{}' in editor", final_path.display()),
                Lang::Ru => format!("Открыт файл '{}'", final_path.display()),
            });
        });
    }

    let watch_toggle_btn = gtk4::ToggleButton::builder()
        .label(tr(lang, "󰈈 Live Watch", "󰈈 Авто-просмотр"))
        .tooltip_text(tr(lang, "Hot-reload theme on every Ctrl+S save", "Автоматически применять тему при сохранении файла (Ctrl+S)"))
        .build();
    watch_toggle_btn.add_css_class("flat-btn");
    {
        let watch_child_c = watch_child.clone();
        let sel_theme_c = selected_theme.clone();
        let status_cb_c = status_cb.clone();
        watch_toggle_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                let cur_id = sel_theme_c.borrow().clone();
                let id = if cur_id.is_empty() { "current".to_string() } else { cur_id };
                match std::process::Command::new("inerium-theme")
                    .arg("watch")
                    .arg(&id)
                    .spawn()
                {
                    Ok(child) => {
                        watch_child_c.replace(Some(child));
                        status_cb_c.ok(&match lang {
                            Lang::En => format!("Live watch started for '{}'", id),
                            Lang::Ru => format!("Live-просмотр запущен для '{}'", id),
                        });
                    }
                    Err(e) => {
                        btn.set_active(false);
                        status_cb_c.warn(&format!("Watch error: {}", e));
                    }
                }
            } else {
                if let Some(mut child) = watch_child_c.borrow_mut().take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                status_cb_c.ok(&match lang {
                    Lang::En => "Live watch stopped".to_string(),
                    Lang::Ru => "Live-просмотр остановлен".to_string(),
                });
            }
        });
    }

    let open_theme_btn = Button::builder()
        .label(tr(lang, "󰉋 Folder", "󰉋 Папка тем"))
        .tooltip_text(tr(lang, "Open ~/.config/inerium/themes", "Открыть папку ~/.config/inerium/themes"))
        .build();
    open_theme_btn.add_css_class("flat-btn");
    open_theme_btn.connect_clicked(|_| {
        let path = inerium_hw::sysfs::home_dir().join(".config/inerium/themes");
        std::thread::spawn(move || {
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
        });
    });

    let theme_actions = Box::new(Orientation::Horizontal, 6);
    theme_actions.append(&new_theme_btn);
    theme_actions.append(&edit_theme_btn);
    theme_actions.append(&watch_toggle_btn);
    theme_actions.append(&open_theme_btn);

    theme_head.append(&theme_info);
    theme_head.append(&theme_actions);
    theme_card.append(&theme_head);

    theme_card.append(&themes_flow);

    let watch_child_cleanup = watch_child.clone();
    theme_card.connect_destroy(move |_| {
        if let Some(mut child) = watch_child_cleanup.borrow_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    });

    theme_card
}

fn populate_setup_themes(
    flow: &gtk4::FlowBox,
    selected_id: &Rc<RefCell<String>>,
    selected_name: &Rc<RefCell<String>>,
    status_cb: &StatusCallback,
    lang: Lang,
) {
    while let Some(child) = flow.first_child() {
        flow.remove(&child);
    }

    let current_id = inerium_theme::get_current_theme_id();
    let available_themes = inerium_theme::load_available_themes();
    let all_themes = inerium_theme::load_all_themes();

    let mut themes = Vec::new();
    for (id, cfg) in all_themes {
        let is_active = id == current_id;
        let is_extra = !available_themes.contains_key(&id);
        let name = cfg.theme.name.clone();
        let variant = cfg.theme.variant.clone();
        if is_active {
            selected_id.replace(id.clone());
            selected_name.replace(name.clone());
        }
        themes.push((id, name, variant, is_active, is_extra));
    }

    let buttons: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));

    for (id, name, variant, is_active, is_extra) in themes {
        let btn = Button::new();
        btn.add_css_class("wall-card");
        btn.set_size_request(140, -1);
        if is_active {
            btn.add_css_class("active");
        }

        let inner = Box::new(Orientation::Vertical, 4);
        inner.set_margin_top(8);
        inner.set_margin_bottom(8);
        inner.set_margin_start(8);
        inner.set_margin_end(8);

        let top_row = Box::new(Orientation::Horizontal, 4);
        let ico_str = if variant == "light" { "󰔏" } else { "󰔎" };
        let ico = Label::new(Some(ico_str));
        ico.add_css_class("card-title");
        top_row.append(&ico);

        if is_extra {
            let extra_badge = Label::new(Some("[extra]"));
            extra_badge.add_css_class("card-meta");
            top_row.append(&extra_badge);
        }
        inner.append(&top_row);

        let title = Label::new(Some(&name));
        title.add_css_class("card-title");
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        inner.append(&title);

        let badge = Label::new(Some(&format!("[{variant}]")));
        badge.add_css_class("card-muted");
        inner.append(&badge);

        btn.set_child(Some(&inner));

        let id_click = id.clone();
        let name_click = name.clone();
        let status_cb_click = status_cb.clone();
        let btns_ref = buttons.clone();
        let sel_id_c = selected_id.clone();
        let sel_name_c = selected_name.clone();

        btn.connect_clicked(move |_| {
            for (bid, b) in btns_ref.borrow().iter() {
                if bid == &id_click {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            sel_id_c.replace(id_click.clone());
            sel_name_c.replace(name_click.clone());

            status_cb_click.warn(&match lang {
                Lang::En => format!("Applying theme '{name_click}'..."),
                Lang::Ru => format!("Применение темы '{name_click}'..."),
            });
            let status_local = status_cb_click.clone();
            let name_local = name_click.clone();
            let id_local = id_click.clone();
            glib::MainContext::default().spawn_local(async move {
                let res = gio::spawn_blocking(move || {
                    std::process::Command::new("inerium-theme")
                        .arg("set")
                        .arg(&id_local)
                        .status()
                }).await;
                if let Ok(Ok(s)) = res && s.success() {
                    crate::theme::reload_theme();
                    status_local.ok(&match lang {
                        Lang::En => format!("Theme '{name_local}' applied"),
                        Lang::Ru => format!("Тема '{name_local}' успешно применена"),
                    });
                } else {
                    status_local.error(&match lang {
                        Lang::En => format!("Failed to apply theme '{name_local}'"),
                        Lang::Ru => format!("Не удалось применить тему '{name_local}'"),
                    });
                }
            });
        });

        buttons.borrow_mut().push((id, btn.clone()));
        flow.append(&btn);
    }

    let btns_weak = Rc::downgrade(&buttons);
    let sel_id_weak = Rc::downgrade(selected_id);
    let sel_name_weak = Rc::downgrade(selected_name);
    crate::theme::add_theme_listener(move || {
        let (Some(btns), Some(sel_id), Some(sel_name)) = (
            btns_weak.upgrade(),
            sel_id_weak.upgrade(),
            sel_name_weak.upgrade(),
        ) else {
            return false;
        };

        let current_id = inerium_theme::get_current_theme_id();
        sel_id.replace(current_id.clone());

        for (bid, b) in btns.borrow().iter() {
            if bid == &current_id {
                b.add_css_class("active");
            } else {
                b.remove_css_class("active");
            }
        }

        let all_themes = inerium_theme::load_all_themes();
        if let Some(cfg) = all_themes.get(&current_id) {
            sel_name.replace(cfg.theme.name.clone());
        }
        true
    });
}
