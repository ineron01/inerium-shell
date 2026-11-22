//! hypr-ipc-bridge — Native Rust Unix socket bridge for Hyprland 0.56+ Lua API compatibility.
//!
//! Intercepts commands sent to Hyprland's IPC socket (.socket.sock) by Waybar
//! and translates legacy dispatch syntax into valid Lua dispatch calls.

use std::ffi::CString;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::thread;
use std::time::Duration;

use inerium_tools::ipc::hyprland::{get_socket_dir, get_socket2};
use inerium_tools::sys::posix::*;

static PUB_SOCK_PTR: AtomicPtr<std::ffi::c_char> = AtomicPtr::new(std::ptr::null_mut());
static REAL_SOCK_PTR: AtomicPtr<std::ffi::c_char> = AtomicPtr::new(std::ptr::null_mut());
static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_: i32) {
    restore_sockets();
    posix_exit(0);
}

fn restore_sockets() {
    let pub_ptr = PUB_SOCK_PTR.load(Ordering::SeqCst);
    let real_ptr = REAL_SOCK_PTR.load(Ordering::SeqCst);
    if !pub_ptr.is_null() && !real_ptr.is_null() {
        let pub_cstr = unsafe { std::ffi::CStr::from_ptr(pub_ptr) };
        let real_cstr = unsafe { std::ffi::CStr::from_ptr(real_ptr) };
        let _ = posix_unlink(pub_cstr);
        let _ = posix_rename(real_cstr, pub_cstr);
    }
}

pub fn escape_lua_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' | '\n' | '\0' => {}
            other => out.push(other),
        }
    }
    out
}

fn strip_cmd<'a>(s: &'a str, cmd: &str) -> Option<&'a str> {
    if s == cmd {
        Some("")
    } else if let Some(rest) = s.strip_prefix(cmd) {
        if rest.starts_with(char::is_whitespace) {
            Some(rest.trim_start())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn translate(cmd: &str) -> Option<String> {
    let s = cmd.trim_matches(|c| c == '\0' || c == '\r' || c == '\n' || c == ' ');
    let s = s.strip_prefix('/').unwrap_or(s);
    let s = s.strip_prefix("dispatch")?;
    let rest = s.trim_start();

    // 1. Toggle special workspace
    if let Some(arg) = strip_cmd(rest, "togglespecialworkspace") {
        let name = arg.trim();
        if name.is_empty() {
            return Some("dispatch hl.dsp.workspace.toggle_special({})".to_string());
        } else {
            return Some(format!(
                "dispatch hl.dsp.workspace.toggle_special({{ name = \"{}\" }})",
                escape_lua_str(name)
            ));
        }
    }

    // 2. Move cursor
    if let Some(coords) = strip_cmd(rest, "movecursor") {
        let mut parts = coords.split_whitespace();
        if let (Some(x), Some(y)) = (parts.next(), parts.next())
            && x.parse::<i32>().is_ok()
            && y.parse::<i32>().is_ok()
        {
            return Some(format!("dispatch hl.dsp.cursor.move({{ x = {}, y = {} }})", x, y));
        }
    }

    // 2b. Focus monitor
    if let Some(arg) = strip_cmd(rest, "focusmonitor") {
        let first_word = arg.split_whitespace().next().unwrap_or("");
        if !first_word.is_empty() {
            return Some(format!(
                "dispatch hl.dsp.focus({{ monitor = \"{}\" }})",
                escape_lua_str(first_word)
            ));
        }
    }

    // 3. Focus workspace (regular, relative, special or on current monitor)
    let (ws_arg, on_current_monitor) = if let Some(arg) = strip_cmd(rest, "focusworkspaceoncurrentmonitor") {
        (Some(arg), true)
    } else if let Some(arg) = strip_cmd(rest, "workspace") {
        (Some(arg), false)
    } else {
        (None, false)
    };

    if let Some(arg) = ws_arg {
        let first_word = arg.split_whitespace().next().unwrap_or("");
        if !first_word.is_empty() {
            let monitor_opt = if on_current_monitor {
                ", on_current_monitor = true"
            } else {
                ""
            };

            // Relative prefixes: e+N, e-N, r+N, r-N, m+N, m-N
            if first_word.starts_with("e+")
                || first_word.starts_with("r+")
                || first_word.starts_with("m+")
                || first_word.starts_with("e-")
                || first_word.starts_with("r-")
                || first_word.starts_with("m-")
            {
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = \"{}\"{} }})",
                    escape_lua_str(first_word),
                    monitor_opt
                ));
            }

            // Direct relative offsets: +N, -N
            if first_word.starts_with('+') || first_word.starts_with('-') {
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = \"{}\"{} }})",
                    escape_lua_str(first_word),
                    monitor_opt
                ));
            }

            // Named special targets: previous, empty, etc.
            if matches!(
                first_word,
                "previous" | "previous_per_monitor" | "empty" | "empty_per_monitor" | "current"
            ) {
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = \"{}\"{} }})",
                    escape_lua_str(first_word),
                    monitor_opt
                ));
            }

            // Absolute numbered workspace
            if first_word.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = {}{} }})",
                    first_word, monitor_opt
                ));
            } else if first_word.starts_with("name:") {
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = \"{}\"{} }})",
                    escape_lua_str(first_word),
                    monitor_opt
                ));
            } else {
                // Named workspace without explicit name: prefix
                return Some(format!(
                    "dispatch hl.dsp.focus({{ workspace = \"name:{}\"{} }})",
                    escape_lua_str(first_word),
                    monitor_opt
                ));
            }
        }
    }

    None
}

fn handle_client(mut client: UnixStream, real_sock: &Path) {
    let _ = client.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(5)));

    let mut buf = [0u8; 8192];
    let mut request_data = Vec::new();

    match client.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => {
            request_data.extend_from_slice(&buf[..n]);
            if n == buf.len() {
                let _ = client.set_read_timeout(Some(Duration::from_millis(10)));
                loop {
                    match client.read(&mut buf) {
                        Ok(0) => break,
                        Ok(m) => {
                            request_data.extend_from_slice(&buf[..m]);
                            if m < buf.len() {
                                break;
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            break;
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }

    if request_data.is_empty() {
        return;
    }

    let mut upstream = match UnixStream::connect(real_sock) {
        Ok(u) => u,
        Err(_) => return,
    };
    let _ = upstream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = upstream.set_write_timeout(Some(Duration::from_secs(5)));

    let data_to_send: Vec<u8> = if let Ok(cmd_str) = std::str::from_utf8(&request_data) {
        if let Some(translated) = translate(cmd_str) {
            translated.into_bytes()
        } else {
            request_data
        }
    } else {
        request_data
    };

    if upstream.write_all(&data_to_send).is_err() {
        return;
    }
    let _ = upstream.shutdown(std::net::Shutdown::Write);

    let mut resp_buf = [0u8; 65536];
    loop {
        match upstream.read(&mut resp_buf) {
            Ok(0) | Err(_) => break,
            Ok(rn) => {
                if client.write_all(&resp_buf[..rn]).is_err() {
                    break;
                }
            }
        }
    }
}

fn watch_hyprland_lifecycle(s2_path: PathBuf) {
    let mut stream = None;
    for _ in 0..10 {
        if let Ok(s) = UnixStream::connect(&s2_path) {
            stream = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let Some(stream) = stream else {
        eprintln!("[hypr-ipc-bridge] Failed to connect to Hyprland socket2 at {}", s2_path.display());
        RUNNING.store(false, Ordering::SeqCst);
        restore_sockets();
        std::process::exit(1);
    };

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if line.is_err() {
            break;
        }
    }

    // Socket2 closed = Hyprland terminated (clean EOF)
    eprintln!("[hypr-ipc-bridge] Hyprland socket2 closed, exiting");
    RUNNING.store(false, Ordering::SeqCst);
    restore_sockets();
    std::process::exit(0);
}

pub fn run(_args: &[String]) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_sockets();
        default_hook(info);
    }));

    let sock_dir = match get_socket_dir() {
        Some(d) => d,
        None => {
            eprintln!("[hypr-ipc-bridge] HYPRLAND_INSTANCE_SIGNATURE could not be determined");
            std::process::exit(1);
        }
    };

    let pub_sock = sock_dir.join(".socket.sock");
    let real_sock = sock_dir.join(".socket.real.sock");

    // Register C strings for signal handler
    let pub_bytes = pub_sock.to_string_lossy().into_owned().into_bytes();
    let real_bytes = real_sock.to_string_lossy().into_owned().into_bytes();
    if let (Ok(pub_c), Ok(real_c)) = (CString::new(pub_bytes), CString::new(real_bytes)) {
        PUB_SOCK_PTR.store(pub_c.into_raw(), Ordering::SeqCst);
        REAL_SOCK_PTR.store(real_c.into_raw(), Ordering::SeqCst);
    }

    posix_signal(SIGHUP, handle_signal);
    posix_signal(SIGINT, handle_signal);
    posix_signal(SIGQUIT, handle_signal);
    posix_signal(SIGTERM, handle_signal);

    if !real_sock.exists() {
        if !pub_sock.exists() {
            eprintln!("[hypr-ipc-bridge] Error: {} does not exist", pub_sock.display());
            restore_sockets();
            std::process::exit(1);
        }
        if let Err(e) = fs::rename(&pub_sock, &real_sock) {
            eprintln!("[hypr-ipc-bridge] Failed to rename socket: {e}");
            restore_sockets();
            std::process::exit(1);
        }
    } else if pub_sock.exists() {
        let _ = fs::remove_file(&pub_sock);
    }

    let listener = match UnixListener::bind(&pub_sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[hypr-ipc-bridge] Failed to bind {}: {e}", pub_sock.display());
            restore_sockets();
            std::process::exit(1);
        }
    };

    // Monitor Hyprland socket2 to cleanly exit when compositor restarts
    if let Some(s2) = get_socket2() {
        thread::spawn(move || {
            watch_hyprland_lifecycle(s2);
        });
    }

    eprintln!(
        "[hypr-ipc-bridge] Listening on {}, forwarding to {}",
        pub_sock.display(),
        real_sock.display()
    );

    for stream in listener.incoming() {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        match stream {
            Ok(client) => {
                let real_path = real_sock.clone();
                thread::spawn(move || {
                    handle_client(client, &real_path);
                });
            }
            Err(e) => {
                if RUNNING.load(Ordering::SeqCst) {
                    eprintln!("[hypr-ipc-bridge] Accept error: {e}");
                }
            }
        }
    }

    restore_sockets();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translations() {
        assert_eq!(
            translate("dispatch workspace e+1"),
            Some("dispatch hl.dsp.focus({ workspace = \"e+1\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace +1"),
            Some("dispatch hl.dsp.focus({ workspace = \"+1\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace -1"),
            Some("dispatch hl.dsp.focus({ workspace = \"-1\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace m+1"),
            Some("dispatch hl.dsp.focus({ workspace = \"m+1\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace previous"),
            Some("dispatch hl.dsp.focus({ workspace = \"previous\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace empty"),
            Some("dispatch hl.dsp.focus({ workspace = \"empty\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace r-1"),
            Some("dispatch hl.dsp.focus({ workspace = \"r-1\" })".to_string())
        );
        assert_eq!(
            translate("/dispatch workspace 2"),
            Some("dispatch hl.dsp.focus({ workspace = 2 })".to_string())
        );
        assert_eq!(
            translate("dispatch focusworkspaceoncurrentmonitor 3"),
            Some("dispatch hl.dsp.focus({ workspace = 3, on_current_monitor = true })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace name:gaming"),
            Some("dispatch hl.dsp.focus({ workspace = \"name:gaming\" })".to_string())
        );
        assert_eq!(
            translate("dispatch workspace gaming"),
            Some("dispatch hl.dsp.focus({ workspace = \"name:gaming\" })".to_string())
        );
        assert_eq!(
            translate("dispatch focusworkspaceoncurrentmonitor name:special"),
            Some("dispatch hl.dsp.focus({ workspace = \"name:special\", on_current_monitor = true })".to_string())
        );
        assert_eq!(
            translate("dispatch focusworkspaceoncurrentmonitor gaming"),
            Some("dispatch hl.dsp.focus({ workspace = \"name:gaming\", on_current_monitor = true })".to_string())
        );
        assert_eq!(
            translate("dispatch togglespecialworkspace"),
            Some("dispatch hl.dsp.workspace.toggle_special({})".to_string())
        );
        assert_eq!(
            translate("dispatch togglespecialworkspace special:magic"),
            Some("dispatch hl.dsp.workspace.toggle_special({ name = \"special:magic\" })".to_string())
        );
        assert_eq!(
            translate("dispatch movecursor 100 200"),
            Some("dispatch hl.dsp.cursor.move({ x = 100, y = 200 })".to_string())
        );
        assert_eq!(
            translate("dispatch movecursor -15 -30"),
            Some("dispatch hl.dsp.cursor.move({ x = -15, y = -30 })".to_string())
        );
        assert_eq!(
            translate("dispatch focusmonitor HDMI-A-1"),
            Some("dispatch hl.dsp.focus({ monitor = \"HDMI-A-1\" })".to_string())
        );
        assert_eq!(translate("monitors"), None);
        assert_eq!(translate("dispatch exec kitty"), None);

        // Lua escaping & command boundary tests
        assert_eq!(
            translate(r#"dispatch workspace name:test"foo\bar"#),
            Some(r#"dispatch hl.dsp.focus({ workspace = "name:test\"foo\\bar" })"#.to_string())
        );
        assert_eq!(
            translate(r#"dispatch togglespecialworkspace special:"magic\"#),
            Some(r#"dispatch hl.dsp.workspace.toggle_special({ name = "special:\"magic\\" })"#.to_string())
        );
        assert_eq!(translate("dispatch workspaceextra 1"), None);
        assert_eq!(translate("dispatch movecursorfast 10 20"), None);
        assert_eq!(translate("dispatch togglespecialworkspacefast"), None);
        assert_eq!(translate("dispatch focusmonitorfast HDMI-1"), None);
    }

    #[test]
    fn test_handle_client_no_newline_no_shutdown_fast() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mock_hypr_sock = std::env::temp_dir().join(format!(
            "mock-hypr-{}-{}.sock",
            std::process::id(),
            unique
        ));
        let _ = std::fs::remove_file(&mock_hypr_sock);
        let mock_listener = UnixListener::bind(&mock_hypr_sock).expect("bind mock listener");

        let mock_thread = thread::spawn(move || {
            if let Ok((mut upstream_conn, _)) = mock_listener.accept() {
                let mut req = Vec::new();
                let mut b = [0u8; 1024];
                if let Ok(n) = upstream_conn.read(&mut b) {
                    req.extend_from_slice(&b[..n]);
                }
                let _ = upstream_conn.write_all(b"ok\n");
                let _ = upstream_conn.flush();
                req
            } else {
                Vec::new()
            }
        });

        let (mut client_end, server_end) = UnixStream::pair().expect("unix stream pair");
        let mock_sock_path = mock_hypr_sock.clone();
        let bridge_thread = thread::spawn(move || {
            handle_client(server_end, &mock_sock_path);
        });

        let start = std::time::Instant::now();
        // Send command without trailing newline and without shutdown(Write)
        client_end
            .write_all(b"dispatch workspace 2")
            .expect("write to bridge");

        let mut resp_buf = [0u8; 128];
        let n = client_end.read(&mut resp_buf).expect("read response");
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "Request took {:?}, expected < 500ms",
            elapsed
        );
        assert_eq!(&resp_buf[..n], b"ok\n");

        bridge_thread.join().expect("bridge thread join");
        let received_upstream = mock_thread.join().expect("mock thread join");
        assert_eq!(
            std::str::from_utf8(&received_upstream).unwrap(),
            "dispatch hl.dsp.focus({ workspace = 2 })"
        );
        let _ = std::fs::remove_file(&mock_hypr_sock);
    }
}
