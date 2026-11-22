//! Inerium Dragon — Native Rust Bluetooth Provider for Waybar
//! - Fast sysfs & rfkill checking
//! - Watch mode: Streams bluetooth status, reacts to SIGUSR1
//! - Tooltip: Rich Pango markup with adapter status and connected devices

use std::fs;
use std::io::{self, Write};
use std::os::unix::io::IntoRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use inerium_i18n::tr;
use inerium_tools::format::{escape_json, escape_pango};
use inerium_tools::ipc::{PidRegistry, SelfPipe};
use inerium_tools::sys::posix::{posix_poll, PollFd, POLLERR, POLLHUP, POLLIN};

enum BtState {
    PoweredOff,
    PoweredOnNoDev,
    Connected(String),
}

fn has_bluetooth() -> bool {
    fs::read_dir("/sys/class/bluetooth")
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false)
}

fn get_bt_state() -> BtState {
    if !has_bluetooth() {
        return BtState::PoweredOff;
    }

    // Check rfkill
    let mut rfkill_on = true;
    if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
        for entry in entries.flatten() {
            let tpath = entry.path().join("type");
            if let Ok(t) = fs::read_to_string(&tpath)
                && t.trim() == "bluetooth" {
                    let spath = entry.path().join("state");
                    if let Ok(s) = fs::read_to_string(&spath)
                        && s.trim() == "0" {
                            rfkill_on = false;
                        }
                }
        }
    }

    if !rfkill_on {
        return BtState::PoweredOff;
    }

    // Check if controller is powered
    let is_powered = Command::new("timeout")
        .args(["2", "bluetoothctl", "show"])
        .stdin(Stdio::null())
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("Powered: yes"))
        .unwrap_or(false);

    if !is_powered {
        return BtState::PoweredOff;
    }

    // Check connected devices
    if let Ok(output) = Command::new("timeout")
        .args(["2", "bluetoothctl", "devices", "Connected"])
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let first_line = text.lines().next().unwrap_or("").trim();
        if !first_line.is_empty() {
            let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return BtState::Connected(parts[2].to_string());
            }
        }
    }

    BtState::PoweredOnNoDev
}

fn format_status() -> String {
    match get_bt_state() {
        BtState::PoweredOff => "󰂲".to_string(),
        BtState::PoweredOnNoDev => "󰂯".to_string(),
        BtState::Connected(name) => {
            let short_name = if name.chars().count() > 12 {
                format!("{}…", name.chars().take(11).collect::<String>())
            } else {
                name
            };
            format!("󰂱 {}", short_name)
        }
    }
}

fn format_tooltip_for(state: &BtState) -> String {
    match state {
        BtState::PoweredOff => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#c4746e' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰂲 Bluetooth is disabled", "󰂲 Bluetooth выключен"),
                tr("Adapter is turned off", "Адаптер отключен"),
                tr("LMB: Devices · RMB: Toggle · MMB: Bluetooth Manager", "ЛКМ: Устройства · ПКМ: Вкл/выкл · СКМ: Диспетчер Bluetooth")
            )
        }
        BtState::PoweredOnNoDev => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#8ba4b0' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰂯 Bluetooth active", "󰂯 Bluetooth активен"),
                tr("No connected devices", "Нет подключенных устройств"),
                tr("LMB: Devices · RMB: Toggle · MMB: Bluetooth Manager", "ЛКМ: Устройства · ПКМ: Вкл/выкл · СКМ: Диспетчер Bluetooth")
            )
        }
        BtState::Connected(name) => {
            format!(
                "<span font_desc='JetBrainsMono Nerd Font 10.5'>\
                <span color='#7fb4ca' weight='bold'>{}</span>\n\
                <span color='#727169'>{}</span> <span color='#e6e2cc'>{}</span>\n\
                <span color='#727169' size='small'>{}</span>\
                </span>",
                tr("󰂱 Connected", "󰂱 Подключено"),
                tr("Device:", "Устройство:"),
                escape_pango(name),
                tr("LMB: Devices · RMB: Toggle · MMB: Bluetooth Manager", "ЛКМ: Устройства · ПКМ: Вкл/выкл · СКМ: Диспетчер Bluetooth")
            )
        }
    }
}

fn format_tooltip() -> String {
    format_tooltip_for(&get_bt_state())
}

fn format_json() -> String {
    let state = get_bt_state();
    let (class, text) = match &state {
        BtState::PoweredOff => ("off", "󰂲".to_string()),
        BtState::PoweredOnNoDev => ("on", "󰂯".to_string()),
        BtState::Connected(name) => {
            let short_name = if name.chars().count() > 12 {
                format!("{}…", name.chars().take(11).collect::<String>())
            } else {
                name.clone()
            };
            ("connected", format!("󰂱 {}", short_name))
        }
    };
    let tooltip = format_tooltip_for(&state);
    format!(
        "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":\"{}\"}}",
        escape_json(&text),
        escape_json(&tooltip),
        class
    )
}

fn spawn_dbus_monitor() -> Option<(Child, i32)> {
    let mut cmd = Command::new("dbus-monitor");
    cmd.args([
        "--system",
        "type='signal',sender='org.bluez',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'",
    ]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(stdout) = child.stdout.take() {
                let fd = stdout.into_raw_fd();
                unsafe {
                    let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                Some((child, fd))
            } else {
                let _ = child.kill();
                None
            }
        }
        Err(_) => None,
    }
}

fn drain_fd(fd: i32) -> bool {
    let mut buf = [0u8; 1024];
    loop {
        let ret = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if ret == 0 {
            return true;
        }
        if ret < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return false;
        }
    }
}

fn watch_mode(waybar: bool) {
    let reg = PidRegistry::new("bluetooth");
    let _pid_guard = reg.register();

    let self_pipe = match SelfPipe::new() {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("Failed to setup self-pipe: {e}");
            return;
        }
    };

    let mut dbus_proc: Option<(Child, i32)> = spawn_dbus_monitor();
    let mut last_output = String::new();

    loop {
        let current = if waybar { format_json() } else { format_status() };
        if current != last_output {
            println!("{}", current);
            io::stdout().flush().ok();
            last_output = current;
        }

        let mut fds = Vec::with_capacity(2);
        fds.push(PollFd {
            fd: self_pipe.read_fd(),
            events: POLLIN,
            revents: 0,
        });

        let dbus_fd_opt = dbus_proc.as_ref().map(|(_, fd)| *fd);
        if let Some(fd) = dbus_fd_opt {
            fds.push(PollFd {
                fd,
                events: POLLIN,
                revents: 0,
            });
        }

        let timeout_ms = if dbus_proc.is_some() { -1 } else { 5000 };
        match posix_poll(&mut fds, timeout_ms) {
            Ok(n) if n > 0 => {
                // Check self_pipe for SIGUSR1 / Lang reinit
                if (fds[0].revents & POLLIN) != 0 {
                    self_pipe.drain();
                    inerium_i18n::Lang::reinit();
                }

                // Check dbus-monitor fd
                if fds.len() > 1 {
                    let rev = fds[1].revents;
                    if (rev & (POLLERR | POLLHUP)) != 0 {
                        // Monitor process exited or closed
                        if let Some((mut child, fd)) = dbus_proc.take() {
                            unsafe {
                                libc::close(fd);
                            }
                            let _ = child.wait();
                        }
                    } else if (rev & POLLIN) != 0
                        && let Some((_, fd)) = dbus_proc.as_ref()
                    {
                        let fd_val = *fd;
                        let eof = drain_fd(fd_val) || {
                            // Debounce 150ms to coalesce burst of DBus signals
                            std::thread::sleep(Duration::from_millis(150));
                            drain_fd(fd_val)
                        };
                        if eof
                            && let Some((mut child, fd)) = dbus_proc.take()
                        {
                            unsafe {
                                libc::close(fd);
                            }
                            let _ = child.wait();
                        }
                    }
                }
            }
            Ok(_) => {
                // Heartbeat timeout: if dbus_proc died, attempt respawn
                if dbus_proc.is_none() {
                    dbus_proc = spawn_dbus_monitor();
                }
            }
            Err(e) if e.raw_os_error() == Some(libc::EINTR) => {
                if self_pipe.drain_if_ready() {
                    inerium_i18n::Lang::reinit();
                }
                continue;
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

pub fn run(args: &[String]) {
    if args.iter().any(|a| a == "--tooltip") {
        println!("{}", format_tooltip());
        return;
    }
    if args.iter().any(|a| a == "--waybar") {
        watch_mode(true);
        return;
    }
    if args.iter().any(|a| a == "--watch") {
        let waybar = args.iter().any(|a| a == "--json" || a == "--waybar");
        watch_mode(waybar);
        return;
    }

    println!("{}", format_status());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tooltip_escaping() {
        let state = BtState::Connected("Device <A & B> \"Pro\" 'Special'".to_string());
        let tt = format_tooltip_for(&state);
        assert!(tt.contains("&lt;A &amp; B&gt; &quot;Pro&quot; &apos;Special&apos;"));
    }
}
