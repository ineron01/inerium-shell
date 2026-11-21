//! Inerium Dragon — Native Rust Keyboard Layout Watcher for Waybar

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;

use inerium_i18n::tr;
use inerium_tools::format::escape_json;
use inerium_tools::ipc::hyprland::get_socket2;

fn get_current_layout() -> &'static str {
    if let Ok(out) = Command::new("hyprctl").args(["devices", "-j"]).output()
        && let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        && let Some(keyboards) = val.get("keyboards").and_then(|v| v.as_array())
    {
        let mut fallback_km: Option<&str> = None;
        for kb in keyboards {
            let km = kb.get("active_keymap").and_then(|v| v.as_str()).unwrap_or("");
            if km.is_empty() {
                continue;
            }
            if kb.get("main").and_then(|v| v.as_bool()) == Some(true) {
                return if km.contains("Russian") || km.contains("RU") {
                    "RU"
                } else {
                    "EN"
                };
            }
            if fallback_km.is_none() {
                fallback_km = Some(km);
            }
        }
        if let Some(km) = fallback_km {
            return if km.contains("Russian") || km.contains("RU") {
                "RU"
            } else {
                "EN"
            };
        }
    }
    "RU"
}

fn switch_keyboards() {
    let _ = Command::new("hyprctl")
        .args(["switchxkblayout", "all", "next"])
        .output();
}

fn format_output(layout: &str, waybar: bool) -> String {
    if waybar {
        let long_name = if layout == "RU" {
            tr("Russian (RU)", "Русская (RU)")
        } else {
            tr("English (EN)", "English (EN)")
        };
        let tooltip = format!(
            "{}: {}\n{}",
            tr("Keyboard layout", "Раскладка"),
            long_name,
            tr("LMB: Switch (RU/EN)", "ЛКМ: Переключить (RU/EN)")
        );
        format!(
            "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":\"{}\"}}",
            escape_json(layout),
            escape_json(&tooltip),
            layout.to_lowercase()
        )
    } else {
        layout.to_string()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--switch") {
        switch_keyboards();
        println!("{}", get_current_layout());
        return;
    }

    let waybar = args.iter().any(|a| a == "--waybar" || a == "--json");
    let stream_mode = waybar || args.iter().any(|a| a == "--watch" || a == "--stream");

    let mut current = get_current_layout();
    println!("{}", format_output(current, waybar));
    std::io::stdout().flush().ok();

    if !stream_mode {
        return;
    }

    if let Some(s2) = get_socket2()
        && let Ok(stream) = UnixStream::connect(s2) {
            let reader = BufReader::new(stream);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(ev) = line.strip_prefix("activelayout>>") {
                    let new_layout = if ev.contains("Russian") || ev.contains("RU") {
                        "RU"
                    } else {
                        "EN"
                    };
                    if new_layout != current {
                        current = new_layout;
                        println!("{}", format_output(current, waybar));
                        std::io::stdout().flush().ok();
                    }
                }
            }
        }
}
