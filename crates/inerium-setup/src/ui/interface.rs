use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Align, Box, Button, ContentFit, DropDown, Entry, FileDialog, FileFilter, Label, Orientation,
    Picture, PolicyType, Scale, ScrolledWindow, StringList, Switch, glib,
};

use crate::i18n::{Lang, tr};
use crate::model::{
    IdleConfig, InputOverrides, SetupPaths, detect_battery_charge_threshold,
    ensure_default_wallpapers, parse_active_wallpaper, parse_hypridle_conf,
    parse_inputs_lua_with_fallback, parse_launcher_bind, scan_wallpapers, set_battery_charge_threshold,
};
use crate::system::{
    apply_hypridle, apply_wallpaper, query_live_inputs, reset_interface_overrides,
    save_inputs_and_reload,
};
use crate::ui::StatusCallback;

const KB_TOGGLE_OPTIONS: &[(&str, &str, &str)] = &[
    ("grp:alt_shift_toggle", "Alt + Shift (Default)", "Alt + Shift (По умолчанию)"),
    ("grp:caps_toggle", "Caps Lock", "Caps Lock"),
    ("grp:ctrl_shift_toggle", "Ctrl + Shift", "Ctrl + Shift"),
    ("grp:win_space_toggle", "Super + Space", "Super + Space"),
];

const LAUNCHER_BIND_OPTIONS: &[(&str, &str, &str)] = &[
    ("SUPER + R", "Super + R (Run style)", "Super + R (в стиле Run диалога)"),
    ("ALT + Space", "Alt + Space (Spotlight style)", "Alt + Space (как в macOS / Spotlight)"),
    ("SUPER + slash", "Super + / (Search style)", "Super + / (быстрый поиск)"),
    ("SUPER + grave", "Super + ` (Quick access)", "Super + ` (тильда над Tab)"),
    ("ALT + F2", "Alt + F2 (Classic Linux)", "Alt + F2 (классический диалог Linux)"),
    ("SUPER + Z", "Super + Z (Ergonomic)", "Super + Z (быстрая клавиша левой руки)"),
];

const IDLE_PRESETS: &[(&str, &str, u32, u32, u32)] = &[
    ("5 min (3m dim / 5m lock / 8m off)", "5 мин (3м экран / 5м блок / 8м выкл)", 180, 300, 480),
    ("10 min (5m dim / 10m lock / 15m off)", "10 мин (5м экран / 10м блок / 15м выкл)", 300, 600, 900),
    ("15 min (10m dim / 15m lock / 20m off)", "15 мин (10м экран / 15м блок / 20м выкл)", 600, 900, 1200),
    ("Never (disabled)", "Никогда (отключить автоблокировку)", 0, 0, 0),
];

pub fn build_interface_tab(
    paths: SetupPaths,
    status_cb: StatusCallback,
    lang: Lang,
) -> (Box, Rc<dyn Fn() -> Result<(), String>>) {
    ensure_default_wallpapers(&paths);
    let outer = Box::new(Orientation::Vertical, 10);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .build();

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_end(4);

    // 0. Theme Selection Card
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

    let selected_theme = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let selected_theme_name = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let watch_child: std::rc::Rc<std::cell::RefCell<Option<std::process::Child>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    let themes_flow = gtk4::FlowBox::new();
    themes_flow.set_valign(gtk4::Align::Start);
    themes_flow.set_max_children_per_line(5);
    themes_flow.set_min_children_per_line(2);
    themes_flow.set_selection_mode(gtk4::SelectionMode::None);
    themes_flow.set_row_spacing(8);
    themes_flow.set_column_spacing(8);

    populate_setup_themes(&themes_flow, &selected_theme, &selected_theme_name, &status_cb, lang);

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
                let sys_path = std::path::PathBuf::from(format!("/usr/share/inerium/themes/{}.toml", id));
                if sys_path.exists() {
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
            let _ = inerium_hw::open_folder(&path);
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
    content.append(&theme_card);

    // 1. Quick Wallpaper Picker Card
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
            let _ = inerium_hw::open_folder(&path);
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
    populate_wallpaper_strip(&wall_strip, &paths, &status_cb, lang);

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
    content.append(&wall_card);

    // 3. Keyboard, Mouse, Touchpad & Power Overrides Card
    let ovr_card = Box::new(Orientation::Vertical, 10);
    ovr_card.add_css_class("setup-card");

    let ovr_head = Box::new(Orientation::Horizontal, 8);
    let ovr_title_box = Box::new(Orientation::Vertical, 2);
    ovr_title_box.set_hexpand(true);

    let ovr_title = Label::new(Some(tr(
        lang,
        "Input, keyboard & power settings",
        "Настройки клавиатуры, ввода и питания",
    )));
    ovr_title.add_css_class("card-title");
    ovr_title.set_xalign(0.0);
    ovr_title_box.append(&ovr_title);

    let ovr_sub = Label::new(Some(tr(
        lang,
        "Saved to ~/.config/hypr/config/{input,keybinds}.lua & hypridle_local.conf",
        "Сохраняется в ~/.config/hypr/config/{input,keybinds}.lua и hypridle_local.conf",
    )));
    ovr_sub.add_css_class("card-muted");
    ovr_sub.set_xalign(0.0);
    ovr_title_box.append(&ovr_sub);
    ovr_head.append(&ovr_title_box);

    let open_binds_btn = Button::builder()
        .label(tr(lang, "󰌌 Keybinds", "󰌌 Хоткеи"))
        .build();
    open_binds_btn.add_css_class("flat-btn");
    open_binds_btn.set_tooltip_text(Some(tr(
        lang,
        "Open ~/.config/hypr/config/keybinds.lua in default editor",
        "Открыть ~/.config/hypr/config/keybinds.lua в редакторе",
    )));
    {
        let paths_clone = paths.clone();
        open_binds_btn.connect_clicked(move |_| {
            let p = paths_clone.keybinds_lua_path();
            if p.exists() {
                let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            }
        });
    }
    ovr_head.append(&open_binds_btn);
    ovr_card.append(&ovr_head);

    let live_inputs = query_live_inputs();
    let saved_inputs = std::fs::read_to_string(paths.input_lua_path())
        .or_else(|_| std::fs::read_to_string(paths.inputs_local_path()))
        .map(|s| parse_inputs_lua_with_fallback(&s, live_inputs.clone()))
        .unwrap_or(live_inputs);
    let saved_binds_launcher = std::fs::read_to_string(paths.keybinds_lua_path())
        .ok()
        .and_then(|s| parse_launcher_bind(&s))
        .or_else(|| {
            std::fs::read_to_string(paths.binds_local_path())
                .ok()
                .and_then(|s| parse_launcher_bind(&s))
        });
    let saved_idle = std::fs::read_to_string(paths.hypridle_local_conf_path())
        .map(|s| parse_hypridle_conf(&s))
        .unwrap_or_default();

    // --- Section: Keyboard ---
    let kb_section = Box::new(Orientation::Vertical, 6);
    kb_section.add_css_class("setup-subcard");

    let kb_sec_title = Label::new(Some(tr(lang, "Keyboard layout & shortcuts", "Раскладка клавиатуры и хоткеи")));
    kb_sec_title.add_css_class("card-meta");
    kb_sec_title.set_xalign(0.0);
    kb_section.append(&kb_sec_title);

    let kb_row = Box::new(Orientation::Horizontal, 10);
    let kb_layout_box = Box::new(Orientation::Vertical, 4);
    kb_layout_box.set_hexpand(true);
    let kb_lbl = Label::new(Some(tr(
        lang,
        "Layouts (kb_layout)",
        "Раскладки (kb_layout)",
    )));
    kb_lbl.add_css_class("card-muted");
    kb_lbl.set_xalign(0.0);
    kb_layout_box.append(&kb_lbl);

    let kb_entry_row = Box::new(Orientation::Horizontal, 6);
    let kb_entry = Entry::new();
    kb_entry.add_css_class("setup-entry");
    kb_entry.set_hexpand(true);
    kb_entry.set_text(&saved_inputs.kb_layout);
    kb_entry_row.append(&kb_entry);

    for preset in ["us,ru", "us", "us,ru,ua"] {
        let pbtn = Button::with_label(preset);
        pbtn.add_css_class("btn-secondary");
        let e_clone = kb_entry.clone();
        pbtn.connect_clicked(move |_| e_clone.set_text(preset));
        kb_entry_row.append(&pbtn);
    }
    kb_layout_box.append(&kb_entry_row);
    kb_row.append(&kb_layout_box);

    // Layout switch shortcut dropdown
    let opt_box = Box::new(Orientation::Vertical, 4);
    let opt_lbl = Label::new(Some(tr(
        lang,
        "Layout switch shortcut",
        "Переключение раскладки",
    )));
    opt_lbl.add_css_class("card-muted");
    opt_lbl.set_xalign(0.0);
    opt_box.append(&opt_lbl);

    let opt_labels: Vec<&str> = KB_TOGGLE_OPTIONS
        .iter()
        .map(|(_, en, ru)| tr(lang, en, ru))
        .collect();
    let opt_dd = DropDown::new(Some(StringList::new(&opt_labels)), gtk4::Expression::NONE);
    opt_dd.add_css_class("setup-dropdown");
    let opt_idx = KB_TOGGLE_OPTIONS
        .iter()
        .position(|(val, _, _)| *val == saved_inputs.kb_options)
        .unwrap_or(0);
    opt_dd.set_selected(opt_idx as u32);
    opt_box.append(&opt_dd);
    kb_row.append(&opt_box);
    kb_section.append(&kb_row);

    // Dynamic conflict warning & launcher shortcut selector
    let conflict_box = Box::new(Orientation::Vertical, 4);
    conflict_box.add_css_class("setup-subcard");
    conflict_box.set_margin_top(4);

    let conflict_warn_lbl = Label::new(Some(tr(
        lang,
        "󰀦 Super + Space is bound to layout toggle. Choose a new shortcut for Application Launcher:",
        "󰀦 Super + Space занят сменой языка. Выберите новую клавишу для Лаунчера приложений:",
    )));
    conflict_warn_lbl.add_css_class("card-title");
    conflict_warn_lbl.set_xalign(0.0);
    conflict_warn_lbl.set_wrap(true);
    conflict_box.append(&conflict_warn_lbl);

    let launcher_row = Box::new(Orientation::Horizontal, 8);
    let launcher_lbl = Label::new(Some(tr(lang, "Launcher Shortcut:", "Клавиша лаунчера:")));
    launcher_lbl.add_css_class("card-meta");
    launcher_row.append(&launcher_lbl);

    let launcher_labels: Vec<&str> = LAUNCHER_BIND_OPTIONS
        .iter()
        .map(|(_, en, ru)| tr(lang, en, ru))
        .collect();
    let launcher_dd = DropDown::new(Some(StringList::new(&launcher_labels)), gtk4::Expression::NONE);
    launcher_dd.add_css_class("setup-dropdown");
    launcher_dd.set_hexpand(true);

    let cur_launcher = saved_binds_launcher.as_deref().unwrap_or("SUPER + R");
    let launcher_idx = LAUNCHER_BIND_OPTIONS
        .iter()
        .position(|(val, _, _)| *val == cur_launcher)
        .unwrap_or(1);
    launcher_dd.set_selected(launcher_idx as u32);
    launcher_row.append(&launcher_dd);
    conflict_box.append(&launcher_row);

    conflict_box.set_visible(saved_inputs.kb_options == "grp:win_space_toggle");

    {
        let conflict_box_c = conflict_box.clone();
        opt_dd.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            let opt = KB_TOGGLE_OPTIONS.get(idx).map(|(v, _, _)| *v).unwrap_or("");
            conflict_box_c.set_visible(opt == "grp:win_space_toggle");
        });
    }

    kb_section.append(&conflict_box);
    ovr_card.append(&kb_section);

    // --- Section: Mouse ---
    let mouse_section = Box::new(Orientation::Vertical, 6);
    mouse_section.add_css_class("setup-subcard");

    let mouse_sec_title = Label::new(Some(tr(lang, "Mouse settings", "Настройки мыши")));
    mouse_sec_title.add_css_class("card-meta");
    mouse_sec_title.set_xalign(0.0);
    mouse_section.append(&mouse_sec_title);

    // Sensitivity slider [-1.0 .. 1.0]
    let sens_row = Box::new(Orientation::Horizontal, 10);
    let sens_lbl = Label::new(Some(tr(
        lang,
        "Sensitivity [-1.0 .. 1.0]:",
        "Чувствительность [-1.0 .. 1.0]:",
    )));
    sens_lbl.add_css_class("card-meta");
    sens_row.append(&sens_lbl);

    let sens_scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.05);
    sens_scale.set_hexpand(true);
    sens_scale.set_value(saved_inputs.sensitivity);
    sens_row.append(&sens_scale);

    let sens_val_lbl = Label::new(Some(&format!("{:.2}", saved_inputs.sensitivity)));
    sens_val_lbl.add_css_class("card-meta");
    sens_val_lbl.set_width_chars(5);
    sens_row.append(&sens_val_lbl);

    let sens_reset_btn = Button::with_label("⟲ 0.0");
    sens_reset_btn.add_css_class("btn-secondary");
    sens_reset_btn.set_tooltip_text(Some(tr(lang, "Reset to default (0.0)", "Сбросить на 0.0")));
    {
        let sens_scale_c = sens_scale.clone();
        sens_reset_btn.connect_clicked(move |_| {
            sens_scale_c.set_value(0.0);
        });
    }
    sens_row.append(&sens_reset_btn);

    {
        let sens_val_lbl_c = sens_val_lbl.clone();
        sens_scale.connect_value_changed(move |sc| {
            sens_val_lbl_c.set_text(&format!("{:.2}", sc.value()));
        });
    }
    mouse_section.append(&sens_row);

    // Mouse Accel profile switch
    let sw_flat = make_switch_row(
        &mouse_section,
        tr(
            lang,
            "Flat profile (No acceleration — recommended for gaming)",
            "Без ускорения (Flat profile — идеально для игр)",
        ),
        saved_inputs.accel_profile == "flat",
    );

    // Scroll speed slider [0.2 .. 3.0]
    let scroll_row = Box::new(Orientation::Horizontal, 10);
    let scroll_lbl = Label::new(Some(tr(
        lang,
        "Scroll speed [0.2 .. 3.0]:",
        "Скорость прокрутки [0.2 .. 3.0]:",
    )));
    scroll_lbl.add_css_class("card-meta");
    scroll_row.append(&scroll_lbl);

    let scroll_scale = Scale::with_range(Orientation::Horizontal, 0.2, 3.0, 0.1);
    scroll_scale.set_hexpand(true);
    scroll_scale.set_value(saved_inputs.scroll_factor);
    scroll_row.append(&scroll_scale);

    let scroll_val_lbl = Label::new(Some(&format!("{:.1}", saved_inputs.scroll_factor)));
    scroll_val_lbl.add_css_class("card-meta");
    scroll_val_lbl.set_width_chars(5);
    scroll_row.append(&scroll_val_lbl);

    let scroll_reset_btn = Button::with_label("⟲ 1.0");
    scroll_reset_btn.add_css_class("btn-secondary");
    scroll_reset_btn.set_tooltip_text(Some(tr(lang, "Reset to default (1.0)", "Сбросить на 1.0")));
    {
        let scroll_scale_c = scroll_scale.clone();
        scroll_reset_btn.connect_clicked(move |_| {
            scroll_scale_c.set_value(1.0);
        });
    }
    scroll_row.append(&scroll_reset_btn);

    {
        let scroll_val_lbl_c = scroll_val_lbl.clone();
        scroll_scale.connect_value_changed(move |sc| {
            scroll_val_lbl_c.set_text(&format!("{:.1}", sc.value()));
        });
    }
    mouse_section.append(&scroll_row);

    ovr_card.append(&mouse_section);

    // --- Section: Touchpad ---
    let tp_section = Box::new(Orientation::Vertical, 6);
    tp_section.add_css_class("setup-subcard");

    let tp_sec_title = Label::new(Some(tr(lang, "Touchpad settings", "Настройки тачпада")));
    tp_sec_title.add_css_class("card-meta");
    tp_sec_title.set_xalign(0.0);
    tp_section.append(&tp_sec_title);

    let sw_natural = make_switch_row(
        &tp_section,
        tr(
            lang,
            "Natural scrolling (content follows fingers, smartphone-like)",
            "Естественная прокрутка (инверсия как на смартфоне)",
        ),
        saved_inputs.natural_scroll,
    );
    let sw_tap = make_switch_row(
        &tp_section,
        tr(lang, "Tap to click", "Касание для клика (Tap-to-click)"),
        saved_inputs.tap_to_click,
    );
    let sw_dwt = make_switch_row(
        &tp_section,
        tr(lang, "Disable while typing", "Отключать при наборе текста (Disable while typing)"),
        saved_inputs.disable_while_typing,
    );
    ovr_card.append(&tp_section);

    // --- Section: Screen & Power ---
    let pwr_section = Box::new(Orientation::Vertical, 6);
    pwr_section.add_css_class("setup-subcard");

    let pwr_sec_title = Label::new(Some(tr(lang, "Screen & Power", "Экран и электропитание")));
    pwr_sec_title.add_css_class("card-meta");
    pwr_sec_title.set_xalign(0.0);
    pwr_section.append(&pwr_sec_title);

    // Autolock row
    let idle_row = Box::new(Orientation::Horizontal, 12);
    let idle_lbl = Label::new(Some(tr(
        lang,
        "Screen lock timeout (hypridle):",
        "Тайм-аут автоблокировки экрана:",
    )));
    idle_lbl.add_css_class("card-meta");
    idle_row.append(&idle_lbl);

    let idle_labels: Vec<&str> = IDLE_PRESETS
        .iter()
        .map(|(en, ru, _, _, _)| tr(lang, en, ru))
        .collect();
    let idle_dd = DropDown::new(Some(StringList::new(&idle_labels)), gtk4::Expression::NONE);
    idle_dd.add_css_class("setup-dropdown");
    idle_dd.set_hexpand(true);

    let idle_idx = if !saved_idle.enabled {
        3 // Never
    } else {
        IDLE_PRESETS
            .iter()
            .position(|&(_, _, d, l, dp)| {
                d == saved_idle.dim_secs && l == saved_idle.lock_secs && dp == saved_idle.dpms_secs
            })
            .unwrap_or(1) // Default: 10 min
    };
    idle_dd.set_selected(idle_idx as u32);
    idle_row.append(&idle_dd);
    pwr_section.append(&idle_row);

    // Battery threshold row (if supported)
    let bat_supported = detect_battery_charge_threshold();
    let sw_bat_limit: Option<Switch> = if let Some((_, current_th)) = bat_supported {
        let sw = make_switch_row(
            &pwr_section,
            tr(
                lang,
                "Battery charge limit 80% (Preserves battery lifespan on AC)",
                "Лимит заряда батареи 80% (Продлевает срок службы аккумулятора)",
            ),
            current_th <= 85,
        );
        Some(sw)
    } else {
        None
    };

    ovr_card.append(&pwr_section);

    // Action buttons for settings
    let ovr_actions = Box::new(Orientation::Horizontal, 8);
    let ovr_reset_btn = Button::with_label(tr(lang, "Reset settings", "Сбросить настройки"));
    ovr_reset_btn.add_css_class("btn-danger");
    ovr_actions.append(&ovr_reset_btn);
    ovr_card.append(&ovr_actions);

    content.append(&ovr_card);

    let save_fn: Rc<dyn Fn() -> Result<(), String>> = {
        let paths = paths.clone();
        let kb_entry = kb_entry.clone();
        let opt_dd = opt_dd.clone();
        let launcher_dd = launcher_dd.clone();
        let sens_scale = sens_scale.clone();
        let sw_flat = sw_flat.clone();
        let scroll_scale = scroll_scale.clone();
        let sw_natural = sw_natural.clone();
        let sw_tap = sw_tap.clone();
        let sw_dwt = sw_dwt.clone();
        let idle_dd = idle_dd.clone();
        let sw_bat_limit = sw_bat_limit.clone();

        Rc::new(move || {
            let layout = kb_entry.text().trim().to_string();
            if layout.is_empty() {
                return Err(match lang {
                    Lang::En => "kb_layout cannot be empty".to_string(),
                    Lang::Ru => "Поле kb_layout не может быть пустым".to_string(),
                });
            }
            let opt_idx = opt_dd.selected() as usize;
            let kb_options = KB_TOGGLE_OPTIONS
                .get(opt_idx)
                .map(|(v, _, _)| *v)
                .unwrap_or("grp:alt_shift_toggle")
                .to_string();

            let launcher_bind = if kb_options == "grp:win_space_toggle" {
                let l_idx = launcher_dd.selected() as usize;
                LAUNCHER_BIND_OPTIONS
                    .get(l_idx)
                    .map(|(v, _, _)| v.to_string())
            } else {
                None
            };

            let input_cfg = InputOverrides {
                kb_layout: layout,
                kb_options,
                natural_scroll: sw_natural.is_active(),
                tap_to_click: sw_tap.is_active(),
                disable_while_typing: sw_dwt.is_active(),
                sensitivity: sens_scale.value(),
                accel_profile: if sw_flat.is_active() {
                    "flat".to_string()
                } else {
                    "adaptive".to_string()
                },
                scroll_factor: scroll_scale.value(),
                launcher_bind,
            };

            let idle_idx = idle_dd.selected() as usize;
            let &(_, _, dim_secs, lock_secs, dpms_secs) =
                IDLE_PRESETS.get(idle_idx).unwrap_or(&IDLE_PRESETS[1]);
            let idle_cfg = IdleConfig {
                enabled: idle_idx != 3, // 3 is "Never"
                dim_secs,
                lock_secs,
                dpms_secs,
            };

            if let Some(ref sw_bat) = sw_bat_limit {
                let target_th = if sw_bat.is_active() { 80 } else { 100 };
                let _ = set_battery_charge_threshold(target_th);
            }

            save_inputs_and_reload(&paths, &input_cfg)?;
            apply_hypridle(&paths, &idle_cfg)?;
            Ok(())
        })
    };

    // Wire Reset Overrides
    {
        let paths = paths.clone();
        let status_cb = status_cb.clone();
        let kb_entry = kb_entry.clone();
        let opt_dd = opt_dd.clone();
        let sens_scale = sens_scale.clone();
        let sw_flat = sw_flat.clone();
        let scroll_scale = scroll_scale.clone();
        let sw_natural = sw_natural.clone();
        let sw_tap = sw_tap.clone();
        let sw_dwt = sw_dwt.clone();
        let idle_dd = idle_dd.clone();
        let sw_bat_limit = sw_bat_limit.clone();

        ovr_reset_btn.connect_clicked(move |_| match reset_interface_overrides(&paths) {
            Ok(()) => {
                let def_in = query_live_inputs();
                kb_entry.set_text(&def_in.kb_layout);
                let opt_idx = KB_TOGGLE_OPTIONS
                    .iter()
                    .position(|(val, _, _)| *val == def_in.kb_options)
                    .unwrap_or(0);
                opt_dd.set_selected(opt_idx as u32);
                sens_scale.set_value(def_in.sensitivity);
                sw_flat.set_active(def_in.accel_profile == "flat");
                scroll_scale.set_value(def_in.scroll_factor);
                sw_natural.set_active(def_in.natural_scroll);
                sw_tap.set_active(def_in.tap_to_click);
                sw_dwt.set_active(def_in.disable_while_typing);
                idle_dd.set_selected(1);
                if let Some(ref sw_bat) = sw_bat_limit {
                    sw_bat.set_active(true);
                    let _ = set_battery_charge_threshold(80);
                }
                status_cb.ok(tr(
                    lang,
                    "Local overrides reset to base config",
                    "Локальные overrides сброшены на базовый конфиг",
                ));
            }
            Err(e) => status_cb.error(&e),
        });
    }

    scroll.set_child(Some(&content));
    outer.append(&scroll);
    (outer, save_fn)
}

fn make_switch_row(parent: &Box, label: &str, active: bool) -> Switch {
    let row = Box::new(Orientation::Horizontal, 8);
    row.set_hexpand(true);
    let lbl = Label::new(Some(label));
    lbl.add_css_class("card-meta");
    lbl.set_xalign(0.0);
    lbl.set_hexpand(true);
    row.append(&lbl);

    let sw = Switch::new();
    sw.add_css_class("setup-switch");
    sw.set_active(active);
    sw.set_valign(Align::Center);
    row.append(&sw);
    parent.append(&row);
    sw
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


fn populate_setup_themes(
    flow: &gtk4::FlowBox,
    selected_id: &std::rc::Rc<std::cell::RefCell<String>>,
    selected_name: &std::rc::Rc<std::cell::RefCell<String>>,
    status_cb: &StatusCallback,
    lang: Lang,
) {
    while let Some(child) = flow.first_child() {
        flow.remove(&child);
    }

    let mut themes = Vec::new();
    if let Ok(out) = std::process::Command::new("inerium-theme").arg("list").arg("--all").arg("--json").output()
        && let Ok(val) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
    {
        for v in val {
            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let variant = v.get("variant").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let is_active = v.get("is_active").and_then(|x| x.as_bool()).unwrap_or(false);
            let is_extra = v.get("is_extra").and_then(|x| x.as_bool()).unwrap_or(false);
            if is_active {
                selected_id.replace(id.clone());
                selected_name.replace(name.clone());
            }
            themes.push((id, name, variant, is_active, is_extra));
        }
    }

    let buttons: std::rc::Rc<std::cell::RefCell<Vec<(String, Button)>>> = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

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
                        .arg("install")
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
            return;
        };
        let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
        if let Ok(content) = std::fs::read_to_string(home.join(".config/inerium/current.json"))
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(active_id) = val.get("id").and_then(|v| v.as_str())
        {
            sel_id.replace(active_id.to_string());
            if let Some(active_name) = val.get("name").and_then(|v| v.as_str()) {
                sel_name.replace(active_name.to_string());
            }
            for (bid, b) in btns.borrow().iter() {
                if bid == active_id {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
        }
    });
}
