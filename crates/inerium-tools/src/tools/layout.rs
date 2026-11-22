//! Inerium Dragon — Native Rust Keyboard Layout Watcher for Waybar

use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::process::Command;

use inerium_i18n::tr;
use crate::format::escape_json;
use crate::ipc::hyprland::get_socket2;
use crate::ipc::{PidRegistry, SelfPipe};
use crate::sys::posix::{posix_poll, PollFd, POLLERR, POLLHUP, POLLIN};

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
    "EN"
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

pub fn run(args: &[String]) {
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

    let reg = PidRegistry::new("layout");
    let _pid_guard = reg.register();
    let self_pipe = match SelfPipe::new() {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("Failed to setup self-pipe: {e}");
            return;
        }
    };

    let mut read_buf = Vec::with_capacity(1024);

    loop {
        if let Some(s2) = get_socket2()
            && let Ok(mut stream) = UnixStream::connect(&s2)
        {
            let sock_fd = stream.as_raw_fd();
            let _ = stream.set_nonblocking(true);
            read_buf.clear();
            let refreshed = get_current_layout();
            if refreshed != current {
                current = refreshed;
                println!("{}", format_output(current, waybar));
                std::io::stdout().flush().ok();
            }

            'inner: loop {
                let mut fds = [
                    PollFd {
                        fd: self_pipe.read_fd(),
                        events: POLLIN,
                        revents: 0,
                    },
                    PollFd {
                        fd: sock_fd,
                        events: POLLIN,
                        revents: 0,
                    },
                ];

                match posix_poll(&mut fds, -1) {
                    Ok(n) if n > 0 => {
                        // Check for SIGUSR1 / locale reload
                        if (fds[0].revents & POLLIN) != 0 {
                            self_pipe.drain();
                            inerium_i18n::Lang::reinit();
                            current = get_current_layout();
                            println!("{}", format_output(current, waybar));
                            std::io::stdout().flush().ok();
                        }

                        // Check for socket events
                        let rev = fds[1].revents;
                        if (rev & (POLLERR | POLLHUP)) != 0 {
                            break 'inner;
                        }

                        if (rev & POLLIN) != 0 {
                            let mut chunk = [0u8; 512];
                            match stream.read(&mut chunk) {
                                Ok(0) => break 'inner,
                                Ok(bytes_read) => {
                                    read_buf.extend_from_slice(&chunk[..bytes_read]);
                                    while let Some(pos) = read_buf.iter().position(|&b| b == b'\n') {
                                        let line = String::from_utf8_lossy(&read_buf[..pos]);
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
                                        read_buf.drain(..=pos);
                                    }
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                Err(_) => break 'inner,
                            }
                        }
                    }
                    Ok(_) => continue,
                    Err(e) if e.raw_os_error() == Some(libc::EINTR) => {
                        if self_pipe.drain_if_ready() {
                            inerium_i18n::Lang::reinit();
                            current = get_current_layout();
                            println!("{}", format_output(current, waybar));
                            std::io::stdout().flush().ok();
                        }
                    }
                    Err(_) => break 'inner,
                }
            }
        }

        // Wait before reconnecting to Hyprland socket
        if self_pipe.wait_event(2000) {
            inerium_i18n::Lang::reinit();
            current = get_current_layout();
            println!("{}", format_output(current, waybar));
            std::io::stdout().flush().ok();
        }
    }
}
