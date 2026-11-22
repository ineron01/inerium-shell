use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use inerium_i18n::tr;
pub use inerium_theme::{
    apply_theme, generate_new_theme_template, get_current_theme_id, get_home,
    install_extra_theme, load_all_themes, load_available_themes, load_theme_by_id,
    parse_theme_toml, resolve_theme_alias, DisplayConfig, Palette, Rgba, ThemeConfig,
    ThemeMeta, Typography,
};

fn simple_time() -> String {
    if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let secs = duration.as_secs() as i64;
        let tm = crate::sys::posix::posix_localtime_r(&secs);
        format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
    } else {
        "00:00:00".to_string()
    }
}

fn cmd_new(id: &str) {
    let clean_id = id.trim().to_lowercase().replace(' ', "-");
    if clean_id.is_empty() {
        eprintln!(
            "{}",
            tr(
                "󰅖 Error: Theme ID cannot be empty.",
                "󰅖 Ошибка: ID темы не может быть пустым."
            )
        );
        eprintln!(
            "{}",
            tr(
                "Usage: inerium-theme new <theme_id>",
                "Использование: inerium-theme new <id_темы>"
            )
        );
        std::process::exit(1);
    }

    let home = get_home();
    let theme_dir = home.join(".config/inerium/themes");
    let _ = fs::create_dir_all(&theme_dir);
    let theme_path = theme_dir.join(format!("{}.toml", clean_id));

    if theme_path.exists() {
        eprintln!(
            "{} {}",
            tr(
                "󰅖 Theme file already exists:",
                "󰅖 Файл темы уже существует:"
            ),
            theme_path.display()
        );
        eprintln!(
            "{}",
            tr(
                "  Edit it or delete it before recreating.",
                "  Отредактируйте его или удалите перед созданием заново."
            )
        );
        std::process::exit(1);
    }

    let content = generate_new_theme_template(&clean_id);
    if let Err(e) = fs::write(&theme_path, content) {
        eprintln!(
            "{} {}: {}",
            tr("󰅖 Failed to create file", "󰅖 Не удалось создать файл"),
            theme_path.display(),
            e
        );
        std::process::exit(1);
    }

    println!(
        "{} {}",
        tr(
            "󰄬 New theme successfully created:",
            "󰄬 Новая тема успешно создана:"
        ),
        theme_path.display()
    );
    println!(
        "{}",
        tr(
            "  Edit it in any editor and launch live preview:",
            "  Редактируйте её в любом редакторе и запустите live-просмотр:"
        )
    );
    println!("   inerium-theme watch {}", clean_id);
}

fn cmd_watch(id: &str) {
    let clean_id = if id.trim().eq_ignore_ascii_case("current") {
        get_current_theme_id()
    } else {
        id.trim().to_lowercase().replace(' ', "-")
    };
    let home = get_home();

    let target_path = if clean_id.ends_with(".toml") && Path::new(&clean_id).exists() {
        PathBuf::from(&clean_id)
    } else {
        let user_theme = home.join(format!(".config/inerium/themes/{}.toml", clean_id));
        let extra_theme = home.join(format!(".local/share/inerium/themes/extra/{}.toml", clean_id));
        let sys_theme = PathBuf::from(format!("/usr/share/inerium/themes/{}.toml", clean_id));
        if user_theme.exists() {
            user_theme
        } else if extra_theme.exists() {
            extra_theme
        } else if sys_theme.exists() {
            sys_theme
        } else {
            eprintln!(
                "{} '{}' {}",
                tr("󰅖 Theme", "󰅖 Тема"),
                clean_id,
                tr("not found in ~/.config/inerium/themes/", "не найдена в ~/.config/inerium/themes/")
            );
            eprintln!(
                "{} inerium-theme new {}",
                tr("  Create it with:", "  Создайте её командой:"),
                clean_id
            );
            std::process::exit(1);
        }
    };

    println!(
        "{} {}",
        tr("󰈈 Live tracking mode started:", "󰈈 Запущен режим отслеживания:"),
        target_path.display()
    );
    println!(
        "{}",
        tr(
            "  Press Ctrl+C to exit. Changes are applied automatically on save.",
            "  Нажмите Ctrl+C для выхода. Изменения применяются сразу при сохранении файла."
        )
    );

    let read_and_apply = |path: &Path| -> bool {
        match fs::read_to_string(path) {
            Ok(content) => match parse_theme_toml(&content) {
                Ok(theme) => {
                    apply_theme(&theme);
                    let now = simple_time();
                    println!(
                        "  [✓ {}] {} '{}' {}",
                        now,
                        tr("Theme", "Тема"),
                        theme.theme.name,
                        tr("successfully updated and applied!", "успешно обновлена и применена!")
                    );
                    true
                }
                Err(e) => {
                    let now = simple_time();
                    eprintln!(
                        "  [✗ {}] {}: {}",
                        now,
                        tr("TOML syntax error", "Ошибка синтаксиса TOML"),
                        e
                    );
                    false
                }
            },
            Err(e) => {
                eprintln!(
                    "  [✗] {} {}: {}",
                    tr("Failed to read file", "Ошибка чтения файла"),
                    path.display(),
                    e
                );
                false
            }
        }
    };

    // Initial apply
    read_and_apply(&target_path);

    let mut last_mtime = fs::metadata(&target_path).and_then(|m| m.modified()).ok();

    loop {
        thread::sleep(Duration::from_millis(250));
        let cur_mtime = fs::metadata(&target_path).and_then(|m| m.modified()).ok();
        if cur_mtime.is_some() && cur_mtime != last_mtime {
            last_mtime = cur_mtime;
            thread::sleep(Duration::from_millis(50));
            read_and_apply(&target_path);
        }
    }
}

fn print_help() {
    let help_text = tr(
        r#"inerium-theme — Theme manager and compiler for Inerium Desktop

USAGE:
    inerium-theme <COMMAND>

COMMANDS:
    list [--all] [--json] Show active themes (or all including extra palettes with --all)
    set <id>              Apply palette by ID (if theme is in extra, installs to persistent)
    preview <id>          Quick preview palette without copying files
    install <id>          Install theme from extra into persistent and apply
    new <id>              Create new theme template in ~/.config/inerium/themes/<id>.toml
    watch [id]            Watch theme file for live changes (hot reload)
    reload                Reload and reapply active theme from current configuration
    toggle                Quick switch between dark and light variants
    status                Output currently active theme and metadata in JSON format
    current               Print currently active theme ID
    help, --help          Show this help message
"#,
        r#"inerium-theme — Менеджер и компилятор тем оформления Inerium Desktop

ИСПОЛЬЗОВАНИЕ:
    inerium-theme <КОМАНДА>

КОМАНДЫ:
    list [--all] [--json] Показать активные темы (или все включая дополнительные с --all)
    set <id>              Применить палитру по ID (если тема в extra, переносит её в постоянные)
    preview <id>          Быстрый показ палитры (только применение оформления без переноса файла)
    install <id>          Перенести тему из дополнительных в постоянные и применить
    new <id>              Создать шаблон новой темы в ~/.config/inerium/themes/<id>.toml
    watch [id]            Запустить отслеживание изменений темы (live reload на лету)
    reload                Перезагрузить и повторно применить активную тему
    toggle                Быстрое переключение (темный <-> светлый вариант)
    status                Вывести текущую активную тему и авторство в формате JSON
    current               Вывести ID текущей активной темы
    help, --help          Показать эту справку
"#,
    );
    println!("{help_text}");
}

pub fn run(args: &[String]) {
    let args: Vec<String> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };
    if args.is_empty() {
        print_help();
        return;
    }

    let active_themes = load_available_themes();

    match args[0].as_str() {
        "list" => {
            let cur_id = get_current_theme_id();
            let show_all = args.iter().any(|a| a == "--all" || a == "-a");
            let themes_map = if show_all {
                load_all_themes()
            } else {
                active_themes.clone()
            };

            if args.iter().any(|a| a == "--json") {
                let list: Vec<_> = themes_map
                    .iter()
                    .map(|(id, t)| {
                        let is_extra = !active_themes.contains_key(id);
                        serde_json::json!({
                            "id": id,
                            "name": t.theme.name,
                            "author": t.theme.author,
                            "variant": t.theme.variant,
                            "description": t.theme.description.clone().unwrap_or_default(),
                            "is_active": id == &cur_id,
                            "is_extra": is_extra,
                            "colors": {
                                "bg": t.palette.bg_canvas,
                                "surface": t.palette.bg_surface,
                                "accent": t.palette.get_primary(),
                                "fg": t.palette.text_primary,
                            }
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&list).unwrap_or_default());
                return;
            }

            println!("{}", tr("Available Inerium Desktop palettes:\n", "Доступные палитры Inerium Desktop:\n"));
            for (id, t) in &themes_map {
                let active_mark = if id == &cur_id { "* " } else { "  " };
                let extra_mark = if !active_themes.contains_key(id) { " (extra)" } else { "" };
                let author_str = format!("({}: {}){}", tr("author", "автор"), t.theme.author, extra_mark);
                println!(
                    "{}{:<18} [{:<5}] {:<22} {}",
                    active_mark, id, t.theme.variant, t.theme.name, author_str
                );
                if let Some(desc) = &t.theme.description {
                    println!("     └─ {}", desc);
                }
            }
            println!("\n{}", tr("Apply: inerium-theme set <id>", "Применить: inerium-theme set <id>"));
        }
        "status" => {
            let cur_id = get_current_theme_id();
            let theme = active_themes.get(&cur_id).or_else(|| active_themes.get("kanagawa-dragon"));
            if let Some(t) = theme {
                if let Ok(s) = serde_json::to_string(&t.theme) {
                    println!("{}", s);
                }
            } else {
                println!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",\"author\":\"rebelot\",\"variant\":\"dark\"}}",
                    cur_id, cur_id
                );
            }
        }
        "toggle" => {
            let cur_id = get_current_theme_id();
            let cur_theme = active_themes.get(&cur_id).or_else(|| active_themes.get("kanagawa-dragon"));
            let is_dark = cur_theme.map(|t| t.theme.variant == "dark").unwrap_or(true);

            let target_id = if is_dark {
                if active_themes.contains_key("inerium-light") {
                    "inerium-light"
                } else {
                    "kanagawa-lotus"
                }
            } else {
                "kanagawa-dragon"
            };

            if let Some(target) = active_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "{} {} ({}: {})",
                    tr("Theme switched:", "Тема переключена:"),
                    target.theme.name,
                    tr("author", "автор"),
                    target.theme.author
                );
            } else {
                eprintln!(
                    "{}: '{}' {}",
                    tr("Error", "Ошибка"),
                    target_id,
                    tr("target palette not found.", "целевая палитра не найдена.")
                );
                std::process::exit(1);
            }
        }
        "install" | "add" => {
            if args.len() < 2 {
                eprintln!(
                    "{}",
                    tr(
                        "Error: specify theme ID. Run 'inerium-theme list --all'.",
                        "Ошибка: укажите идентификатор темы. Выполните 'inerium-theme list --all'."
                    )
                );
                std::process::exit(1);
            }
            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);
            match install_extra_theme(target_id) {
                Ok(theme) => {
                    apply_theme(&theme);
                    println!(
                        "{} '{}' ({}: {}) {}",
                        tr("Theme", "Тема"),
                        theme.theme.name,
                        tr("author", "автор"),
                        theme.theme.author,
                        tr("successfully installed to persistent and applied.", "успешно перенесена в постоянные и применена.")
                    );
                }
                Err(e) => {
                    eprintln!("{}: {e}", tr("Error", "Ошибка"));
                    std::process::exit(1);
                }
            }
        }
        "preview" => {
            if args.len() < 2 {
                eprintln!(
                    "{}",
                    tr(
                        "Error: specify theme ID for preview. Run 'inerium-theme list --all'.",
                        "Ошибка: укажите идентификатор темы для превью. Выполните 'inerium-theme list --all'."
                    )
                );
                std::process::exit(1);
            }
            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);
            let all_themes = load_all_themes();
            if let Some(target) = all_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "{} {} ({}: {})",
                    tr("Palette preview:", "Быстрый показ палитры:"),
                    target.theme.name,
                    tr("author", "автор"),
                    target.theme.author
                );
            } else {
                eprintln!(
                    "{}: {} '{}' {}",
                    tr("Error", "Ошибка"),
                    tr("theme", "тема"),
                    target_id,
                    tr("not found.", "не найдена.")
                );
                println!(
                    "{}",
                    tr(
                        "Run 'inerium-theme list --all' to see all available palettes.",
                        "Запустите 'inerium-theme list --all' для просмотра всех доступных палитр."
                    )
                );
                std::process::exit(1);
            }
        }
        "set" => {
            if args.len() < 2 {
                eprintln!(
                    "{}",
                    tr(
                        "Error: specify theme ID. Run 'inerium-theme list'.",
                        "Ошибка: укажите идентификатор темы. Выполните 'inerium-theme list'."
                    )
                );
                std::process::exit(1);
            }

            let input_id = args[1].to_lowercase();
            let target_id = resolve_theme_alias(&input_id);

            if let Some(target) = active_themes.get(target_id) {
                apply_theme(target);
                println!(
                    "{} {} ({}: {})",
                    tr("Palette applied:", "Установлена палитра:"),
                    target.theme.name,
                    tr("author", "автор"),
                    target.theme.author
                );
                if let Some(upstream) = &target.theme.upstream {
                    println!("{}: {}", tr("Original upstream repository", "Оригинальный репозиторий"), upstream);
                }
            } else if let Ok(target) = install_extra_theme(target_id) {
                apply_theme(&target);
                println!(
                    "{} '{}' {} ({}: {}).",
                    tr("Palette", "Палитра"),
                    target.theme.name,
                    tr("installed from extra into persistent and applied", "перенесена из дополнительных в постоянные и применена"),
                    tr("author", "автор"),
                    target.theme.author
                );
                if let Some(upstream) = &target.theme.upstream {
                    println!("{}: {}", tr("Original upstream repository", "Оригинальный репозиторий"), upstream);
                }
            } else {
                eprintln!(
                    "{}: {} '{}' {}",
                    tr("Error", "Ошибка"),
                    tr("theme", "тема"),
                    target_id,
                    tr("not found.", "не найдена.")
                );
                println!(
                    "{}",
                    tr(
                        "Run 'inerium-theme list --all' to see all available palettes.",
                        "Запустите 'inerium-theme list --all' для просмотра всех доступных палитр."
                    )
                );
                std::process::exit(1);
            }
        }
        "new" => {
            if args.len() < 2 {
                eprintln!(
                    "{}",
                    tr(
                        "Error: specify new theme ID. Example: inerium-theme new my-theme",
                        "Ошибка: укажите идентификатор новой темы. Пример: inerium-theme new my-theme"
                    )
                );
                std::process::exit(1);
            }
            cmd_new(&args[1]);
        }
        "watch" => {
            let id = if args.len() >= 2 {
                args[1].clone()
            } else {
                get_current_theme_id()
            };
            cmd_watch(&id);
        }
        "reload" => {
            let cur_id = get_current_theme_id();
            let resolved = resolve_theme_alias(&cur_id);
            if let Some(target) = active_themes.get(resolved) {
                apply_theme(target);
                println!(
                    "{} {} ({}: {})",
                    tr("Theme reloaded and applied:", "Тема перезагружена и применена:"),
                    target.theme.name,
                    tr("author", "автор"),
                    target.theme.author
                );
            } else if let Ok(target) = load_theme_by_id(resolved) {
                apply_theme(&target);
                println!(
                    "{} {} ({}: {})",
                    tr("Theme reloaded and applied:", "Тема перезагружена и применена:"),
                    target.theme.name,
                    tr("author", "автор"),
                    target.theme.author
                );
            } else {
                eprintln!(
                    "{}: {} '{}' {}",
                    tr("Error", "Ошибка"),
                    tr("current theme", "текущая тема"),
                    cur_id,
                    tr("could not be loaded.", "не может быть загружена.")
                );
                std::process::exit(1);
            }
        }
        "current" => {
            println!("{}", get_current_theme_id());
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!(
                "{} '{}'. {}",
                tr("Unknown command:", "Неизвестная команда:"),
                other,
                tr("Run 'inerium-theme --help'.", "Запустите 'inerium-theme --help'.")
            );
            std::process::exit(1);
        }
    }
}
