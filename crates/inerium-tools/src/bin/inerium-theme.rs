use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Dark => "Inerium Dragon",
            Theme::Light => "Inerium Lotus",
        }
    }

    pub fn display_name_ru(&self) -> &'static str {
        match self {
            Theme::Dark => "Инериум Тёмная (Dragon)",
            Theme::Light => "Инериум Светлая (Lotus)",
        }
    }

    pub fn scheme_name(&self) -> &'static str {
        match self {
            Theme::Dark => "Inerium-Dragon",
            Theme::Light => "Inerium-Lotus",
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

fn get_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ineron"))
}

fn atomic_symlink<P: AsRef<Path>, Q: AsRef<Path>>(target: P, link_path: Q) {
    let link = link_path.as_ref();
    let parent = match link.parent() {
        Some(p) => p,
        None => return,
    };
    let _ = fs::create_dir_all(parent);

    let tmp = parent.join(format!(".tmp_link_{}", std::process::id()));
    let _ = fs::remove_file(&tmp);

    if symlink(target.as_ref(), &tmp).is_ok() {
        let _ = fs::rename(&tmp, link);
    }
}

pub fn detect_current_theme() -> Theme {
    // 1. Try gsettings color-scheme
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout);
        if s.contains("prefer-light") {
            return Theme::Light;
        }
        if s.contains("prefer-dark") {
            return Theme::Dark;
        }
    }

    // 2. Try kdeglobals
    let home = get_home();
    let kdeglobals = home.join(".config/kdeglobals");
    if let Ok(content) = fs::read_to_string(&kdeglobals)
        && content.contains("ColorScheme=Inerium-Lotus") {
            return Theme::Light;
        }

    // Default to Dark
    Theme::Dark
}

pub fn apply_theme(theme: Theme) {
    let home = get_home();
    let scheme = theme.scheme_name();

    // 1. GSettings (Portal, Browsers, VS Code, GTK)
    let (gsettings_scheme, gtk_theme, icon_theme) = match theme {
        Theme::Dark => ("prefer-dark", "adw-gtk3-dark", "Papirus-Dark"),
        Theme::Light => ("prefer-light", "adw-gtk3", "Papirus-Light"),
    };
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "color-scheme", gsettings_scheme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "gtk-theme", gtk_theme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "icon-theme", icon_theme])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 2. GTK 3 & GTK 4 settings.ini
    let prefer_dark = match theme {
        Theme::Dark => "1",
        Theme::Light => "0",
    };
    for gtk_ver in &["gtk-3.0", "gtk-4.0"] {
        let path = home.join(format!(".config/{}/settings.ini", gtk_ver));
        if let Ok(content) = fs::read_to_string(&path) {
            let mut lines = Vec::new();
            for line in content.lines() {
                if line.starts_with("gtk-theme-name=") {
                    lines.push(format!("gtk-theme-name={}", gtk_theme));
                } else if line.starts_with("gtk-application-prefer-dark-theme=") {
                    lines.push(format!("gtk-application-prefer-dark-theme={}", prefer_dark));
                } else if line.starts_with("gtk-icon-theme-name=") {
                    lines.push(format!("gtk-icon-theme-name={}", icon_theme));
                } else {
                    lines.push(line.to_string());
                }
            }
            let _ = fs::write(&path, lines.join("\n") + "\n");
        }
    }

    // 3. KDE Frameworks 6: dolphinrc & kdeglobals
    let dolphinrc_path = home.join(".config/dolphinrc");
    if let Ok(content) = fs::read_to_string(&dolphinrc_path) {
        let mut lines = Vec::new();
        let mut has_ui = false;
        let mut in_ui = false;
        for line in content.lines() {
            if line.trim() == "[UiSettings]" {
                in_ui = true;
                has_ui = true;
                lines.push(line.to_string());
                continue;
            }
            if in_ui && line.starts_with('[') {
                in_ui = false;
            }
            if in_ui && line.starts_with("ColorScheme=") {
                lines.push(format!("ColorScheme={}", scheme));
                continue;
            }
            lines.push(line.to_string());
        }
        if !has_ui {
            lines.push("[UiSettings]".to_string());
            lines.push(format!("ColorScheme={}", scheme));
        }
        let _ = fs::write(&dolphinrc_path, lines.join("\n") + "\n");
    }

    // Read colors from color-schemes/<scheme>.colors
    let color_scheme_path = home.join(format!(".local/share/color-schemes/{}.colors", scheme));
    let color_sections = fs::read_to_string(&color_scheme_path).unwrap_or_default();

    let kdeglobals_path = home.join(".config/kdeglobals");
    let mut kdeglobals_header = format!(
        "[General]\nColorScheme={}\nName={}\nTerminalApplication=kitty\n\n[Icons]\nTheme={}\n\n[KDE]\ncontrast=4\nwidgetStyle=Fusion\n\n[KFileDialog Settings]\nAllow Expansion=false\nAutomatically select filename extension=true\nBreadcrumb Navigation=true\nDecoration position=2\nShow Full Path=false\nShow Inline Previews=true\nShow Preview=false\nShow Speedbar=true\nShow hidden files=false\nSort by=Name\nSort directories first=true\nSort hidden files last=false\nSort reversed=false\nSpeedbar Width=301\nView Style=DetailTree\n\n",
        scheme,
        theme.display_name(),
        icon_theme,
    );

    let mut skip = false;
    for line in color_sections.lines() {
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with("[General]") {
            skip = true;
            continue;
        }
        if skip && line.starts_with('[') {
            skip = false;
        }
        if !skip {
            kdeglobals_header.push_str(line);
            kdeglobals_header.push('\n');
        }
    }
    let _ = fs::write(&kdeglobals_path, kdeglobals_header.trim().to_string() + "\n");

    // Warm KDE cache
    let _ = Command::new("plasma-apply-colorscheme")
        .arg(scheme)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 4. Qt6ct & Qt5ct
    let suffix = match theme {
        Theme::Dark => "Dragon",
        Theme::Light => "Lotus",
    };
    for qt_ct in &["qt6ct", "qt5ct"] {
        let conf_path = home.join(format!(".config/{}/{}.conf", qt_ct, qt_ct));
        if let Ok(content) = fs::read_to_string(&conf_path) {
            let mut lines = Vec::new();
            for line in content.lines() {
                if line.starts_with("color_scheme_path=") {
                    lines.push(format!(
                        "color_scheme_path={}/.config/{}/colors/Inerium-{}.conf",
                        home.display(),
                        qt_ct,
                        suffix
                    ));
                } else if line.starts_with("icon_theme=") {
                    lines.push(format!("icon_theme={}", icon_theme));
                } else {
                    lines.push(line.to_string());
                }
            }
            let _ = fs::write(&conf_path, lines.join("\n") + "\n");
        }
    }

    // 5. Waybar: colors.css symlink + SIGUSR2
    let waybar_target = match theme {
        Theme::Dark => "colors-dragon.css",
        Theme::Light => "colors-lotus.css",
    };
    atomic_symlink(waybar_target, home.join(".config/waybar/colors.css"));
    let _ = Command::new("pkill")
        .args(["-SIGUSR2", "waybar"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 6. Rofi: colors.rasi symlink
    let rofi_target = match theme {
        Theme::Dark => "colors-dragon.rasi",
        Theme::Light => "colors-lotus.rasi",
    };
    atomic_symlink(rofi_target, home.join(".config/rofi/colors.rasi"));

    // 7. Kitty: current-theme.conf symlink + SIGUSR1
    let kitty_target = match theme {
        Theme::Dark => "inerium-dragon.conf",
        Theme::Light => "inerium-lotus.conf",
    };
    atomic_symlink(kitty_target, home.join(".config/kitty/themes/current-theme.conf"));
    let _ = Command::new("killall")
        .args(["-SIGUSR1", "kitty"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 8. Mako: config symlink + reload
    let mako_target = match theme {
        Theme::Dark => "config-dragon",
        Theme::Light => "config-lotus",
    };
    atomic_symlink(mako_target, home.join(".config/mako/config"));
    let _ = Command::new("makoctl")
        .arg("reload")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 9. Hyprland: colors.lua symlink + reload
    let hypr_cfg_dir = home.join(".config/hypr/config");
    let repo_hypr = home.join("Repos/inerium-shell/config/hypr/config");
    for f in &["colors-dragon.lua", "colors-lotus.lua"] {
        let dest = hypr_cfg_dir.join(f);
        if !dest.exists() {
            let src = repo_hypr.join(f);
            if src.exists() {
                let _ = symlink(&src, &dest);
            }
        }
    }

    let hypr_target = match theme {
        Theme::Dark => "colors-dragon.lua",
        Theme::Light => "colors-lotus.lua",
    };
    atomic_symlink(hypr_target, hypr_cfg_dir.join("colors.lua"));
    let _ = Command::new("hyprctl")
        .arg("reload")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 10. Inerium Shell Daemon Hot-Reload
    let uid = inerium_tools::sys::posix::posix_getuid();
    let sock_path = format!("/run/user/{}/inerium-popups.sock", uid);
    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
        let msg = format!("theme {}\n", theme.name());
        let _ = stream.write_all(msg.as_bytes());
    }
}

fn print_help() {
    println!(
        r#"inerium-theme — Менеджер и переключатель темы оформления Inerium Desktop

ИСПОЛЬЗОВАНИЕ:
    inerium-theme <КОМАНДА>

КОМАНДЫ:
    set dark | dragon     Включить темную палитру (Inerium Dragon)
    set light | lotus     Включить светлую палитру (Inerium Lotus)
    toggle                Переключить тему (тёмная <-> светлая)
    status                Вывести текущее состояние темы
    help, --help          Показать эту справку
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "status" => {
            let cur = detect_current_theme();
            println!(
                "{{\"theme\":\"{}\",\"name\":\"{}\",\"description\":\"{}\"}}",
                cur.name(),
                cur.display_name(),
                cur.display_name_ru()
            );
        }
        "toggle" => {
            let cur = detect_current_theme();
            let next = cur.opposite();
            apply_theme(next);
            println!("Тема переключена: {}", next.display_name_ru());
        }
        "set" => {
            if args.len() < 2 {
                eprintln!("Ошибка: укажите тему (dark / light)");
                std::process::exit(1);
            }
            let target = match args[1].to_lowercase().as_str() {
                "dark" | "dragon" | "черная" | "темная" | "тёмная" => Theme::Dark,
                "light" | "lotus" | "белая" | "светлая" => Theme::Light,
                other => {
                    eprintln!("Неизвестная тема: {}. Допустимы: dark (dragon), light (lotus)", other);
                    std::process::exit(1);
                }
            };
            apply_theme(target);
            println!("Установлена тема: {}", target.display_name_ru());
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!("Неизвестная команда: '{}'. Запустите 'inerium-theme --help'.", other);
            std::process::exit(1);
        }
    }
}
