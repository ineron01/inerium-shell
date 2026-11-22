use std::process::Command;
use std::thread;

use relm4::ComponentSender;

use super::{ControlCenterModel, ControlCenterMsg};
use crate::hw;

pub fn get_theme_info() -> (bool, String, String, usize) {
    let home = hw::home_dir();

    let mut theme_ids = std::collections::HashSet::new();
    let user_themes = home.join(".config/inerium/themes");
    if let Ok(entries) = std::fs::read_dir(&user_themes) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                theme_ids.insert(stem.to_string());
            }
        }
    }
    let count = theme_ids.len().max(2);

    let cur_json = home.join(".config/inerium/current.json");
    if let Ok(content) = std::fs::read_to_string(&cur_json)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
    {
        let variant = val.get("variant").and_then(|v| v.as_str()).unwrap_or("dark");
        let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("Kanagawa Dragon");
        let author = val.get("author").and_then(|v| v.as_str()).unwrap_or("rebelot");
        return (variant == "dark", name.to_string(), author.to_string(), count);
    }
    let is_dark = crate::theme::detect_theme() == crate::theme::ThemeMode::Dark;
    if is_dark {
        (true, "Kanagawa Dragon".to_string(), "rebelot".to_string(), count)
    } else {
        (false, "Inerium Light".to_string(), "Inerium Core".to_string(), count)
    }
}

pub fn handle_header_msg(
    model: &mut ControlCenterModel,
    msg: &ControlCenterMsg,
    sender: &ComponentSender<ControlCenterModel>,
) -> bool {
    match msg {
        ControlCenterMsg::CheckUpdates => {
            model.is_updating_check = true;
            let s = sender.clone();
            thread::spawn(move || {
                let _ = Command::new("inerium-updates").arg("--check").output();
                let fresh = hw::get_updates_info();
                s.input(ControlCenterMsg::UpdatesChecked(fresh));
            });
            true
        }
        ControlCenterMsg::UpdatesChecked(fresh) => {
            model.is_updating_check = false;
            model.updates = fresh.clone();
            true
        }
        ControlCenterMsg::LaunchUpdateTerminal => {
            (model.close_cb)();
            let err_prompt = inerium_i18n::tr(
                "\\n[paru exited with code %s] Press Enter to close...",
                "\\n[paru завершился с кодом %s] Нажмите Enter для закрытия...",
            );
            let cmd = format!(
                "paru; rc=$?; inerium-updates --check >/dev/null 2>&1 || true; if [ \"$rc\" -ne 0 ]; then printf '{err_prompt}' \"$rc\"; read -r _; fi"
            );
            thread::spawn(move || {
                let _ = Command::new("uwsm")
                    .args(["app", "--", "kitty", "-e", "sh", "-c", &cmd])
                    .spawn();
            });
            true
        }
        ControlCenterMsg::ToggleHypridle => {
            model.is_idle_active = !model.is_idle_active;
            hw::toggle_hypridle(model.is_idle_active);
            true
        }
        ControlCenterMsg::ToggleTheme => {
            model.is_dark_theme = !model.is_dark_theme;
            let s = sender.clone();
            thread::spawn(move || {
                let _ = Command::new("inerium-theme").arg("toggle").status();
                let (is_dark, name, author, count) = get_theme_info();
                s.input(ControlCenterMsg::ThemeUpdated {
                    is_dark,
                    name,
                    author,
                    count,
                });
            });
            true
        }
        ControlCenterMsg::ThemeUpdated {
            is_dark,
            name,
            author,
            count,
        } => {
            model.is_dark_theme = *is_dark;
            model.theme_name = name.clone();
            model.theme_author = author.clone();
            model.available_themes_count = *count;
            true
        }
        ControlCenterMsg::NavAppearance => {
            (model.nav_cb)("appearance");
            true
        }
        ControlCenterMsg::LockScreen => {
            (model.close_cb)();
            thread::spawn(|| {
                let _ = Command::new("sh")
                    .args(["-c", "hyprlock || loginctl lock-session"])
                    .output();
            });
            true
        }
        ControlCenterMsg::PowerMenu => {
            (model.nav_cb)("power");
            true
        }
        _ => false,
    }
}
