//! Unified and fault-tolerant resolution of Hyprland IPC sockets and JSON queries.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// Returns the base Hyprland runtime directory: `$XDG_RUNTIME_DIR/hypr` or `/run/user/{uid}/hypr`
pub fn get_hyprland_base_dir() -> PathBuf {
    crate::sys::runtime_dir().join("hypr")
}

/// Resolves the directory for the active Hyprland instance.
/// Checks `HYPRLAND_INSTANCE_SIGNATURE` first, falling back to newest modified directory.
pub fn get_socket_dir() -> Option<PathBuf> {
    get_socket_dir_from(&get_hyprland_base_dir(), env::var("HYPRLAND_INSTANCE_SIGNATURE").ok().as_deref())
}

/// Pure resolution helper for socket directory without relying on global environment variables.
pub fn get_socket_dir_from(hypr_dir: &std::path::Path, sig_opt: Option<&str>) -> Option<PathBuf> {
    let sig = sig_opt
        .filter(|s| {
            let p = hypr_dir.join(s);
            p.join(".socket.sock").exists() || p.join(".socket.real.sock").exists() || p.join(".socket2.sock").exists()
        })
        .map(|s| s.to_string())
        .or_else(|| {
            let entries = fs::read_dir(hypr_dir).ok()?;
            let mut subdirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .collect();
            subdirs.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
            subdirs.last().map(|e| e.file_name().to_string_lossy().into_owned())
        })?;

    let target = hypr_dir.join(sig);
    if target.is_dir() {
        Some(target)
    } else {
        None
    }
}

/// Returns path to Hyprland's `.socket.sock` (or proxied socket)
pub fn get_socket1() -> Option<PathBuf> {
    let dir = get_socket_dir()?;
    let p = dir.join(".socket.sock");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Returns path to Hyprland's real backend socket (`.socket.real.sock` if bridge is active, else `.socket.sock`)
pub fn get_real_socket1() -> Option<PathBuf> {
    let dir = get_socket_dir()?;
    let real = dir.join(".socket.real.sock");
    if real.exists() {
        Some(real)
    } else {
        let orig = dir.join(".socket.sock");
        if orig.exists() {
            Some(orig)
        } else {
            None
        }
    }
}

/// Canonical socket lookup for Hyprland command IPC (defaults to real command socket).
pub fn get_hypr_socket() -> Option<PathBuf> {
    get_real_socket1()
}

/// Returns path to Hyprland's event stream socket `.socket2.sock`
pub fn get_socket2() -> Option<PathBuf> {
    let dir = get_socket_dir()?;
    let p = dir.join(".socket2.sock");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Sends a JSON IPC command to Hyprland and deserializes the response.
pub fn query_json<T: serde::de::DeserializeOwned>(cmd: &str) -> Result<T, String> {
    let sock_path = get_hypr_socket()
        .ok_or_else(|| "Hyprland socket not found".to_string())?;
    let mut stream = UnixStream::connect(&sock_path)
        .map_err(|e| format!("Failed to connect to Hyprland socket {}: {e}", sock_path.display()))?;

    let timeout = Some(Duration::from_millis(500));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let full_cmd = if cmd.starts_with("j/") || cmd.starts_with("[[BATCH]]") {
        cmd.to_string()
    } else {
        format!("j/{}", cmd)
    };

    stream.write_all(full_cmd.as_bytes())
        .map_err(|e| format!("Failed to write to Hyprland socket: {e}"))?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read from Hyprland socket: {e}"))?;

    serde_json::from_slice::<T>(&buf)
        .map_err(|e| format!("Failed to parse Hyprland JSON response: {e}"))
}

pub type CursorAndMonitor = (i32, i32, i32, i32, i32, i32, String);

/// Parses cursor position and monitor geometries from Hyprland batch response.
pub fn parse_cursor_and_monitors_from_slice(buf: &[u8]) -> Result<CursorAndMonitor, String> {
    let mut de = serde_json::Deserializer::from_slice(buf).into_iter::<serde_json::Value>();
    let mut cx = 960;
    let mut cy = 540;
    let mut mx = 0;
    let mut my = 0;
    let mut mw = 1920;
    let mut mh = 1080;
    let mut m_name = String::new();
    let mut found_cursor = false;
    let mut found_monitors = false;

    if let Some(Ok(c_val)) = de.next()
        && let (Some(x), Some(y)) = (c_val["x"].as_i64(), c_val["y"].as_i64())
    {
        cx = x as i32;
        cy = y as i32;
        found_cursor = true;
    }

    if let Some(Ok(m_val)) = de.next()
        && let Some(list) = m_val.as_array()
    {
        found_monitors = true;
        let mut matched = None;
        let mut focused = None;
        let mut first = None;

        for mon in list {
            let x = mon["x"].as_i64().unwrap_or(0) as i32;
            let y = mon["y"].as_i64().unwrap_or(0) as i32;
            let scale = mon["scale"].as_f64().unwrap_or(1.0).max(0.1);
            let mut w = (mon["width"].as_f64().unwrap_or(1920.0) / scale).round() as i32;
            let mut h = (mon["height"].as_f64().unwrap_or(1080.0) / scale).round() as i32;
            let transform = mon["transform"].as_i64().unwrap_or(0);
            if transform % 2 != 0 {
                std::mem::swap(&mut w, &mut h);
            }
            let name = mon["name"].as_str().unwrap_or("").to_string();
            let is_focused = mon["focused"].as_bool().unwrap_or(false);

            let mon_info = (x, y, w, h, name);
            if first.is_none() {
                first = Some(mon_info.clone());
            }
            if is_focused && focused.is_none() {
                focused = Some(mon_info.clone());
            }
            if cx >= x && cx < x + w && cy >= y && cy < y + h && matched.is_none() {
                matched = Some(mon_info);
            }
        }

        if let Some((x, y, w, h, name)) = matched.or(focused).or(first) {
            mx = x;
            my = y;
            mw = w;
            mh = h;
            m_name = name;
        }
    }

    if !found_cursor && !found_monitors {
        return Err("Failed to parse cursor and monitor response from Hyprland".to_string());
    }

    Ok((cx, cy, mx, my, mw, mh, m_name))
}

/// Retrieves cursor coordinates and active monitor geometry directly from Hyprland IPC.
/// Returns `(cursor_x, cursor_y, monitor_x, monitor_y, monitor_w, monitor_h, monitor_name)`.
pub fn get_cursor_and_monitor() -> Result<CursorAndMonitor, String> {
    let sock_path = get_hypr_socket()
        .ok_or_else(|| "Hyprland socket not found".to_string())?;
    let mut stream = UnixStream::connect(&sock_path)
        .map_err(|e| format!("Failed to connect to Hyprland socket {}: {e}", sock_path.display()))?;

    let timeout = Some(Duration::from_millis(500));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    stream.write_all(b"[[BATCH]]j/cursorpos ; j/monitors")
        .map_err(|e| format!("Failed to write to Hyprland socket: {e}"))?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read from Hyprland socket: {e}"))?;

    parse_cursor_and_monitors_from_slice(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_dir() {
        if let Some(dir) = get_socket_dir() {
            assert!(dir.is_dir());
        }

        let temp_dir = std::env::temp_dir().join(format!("inerium-test-hypr-{}", std::process::id()));
        let mock_sig = "mock_instance_ci";
        let mock_instance = temp_dir.join(mock_sig);
        let _ = fs::create_dir_all(&mock_instance);
        let mock_sock = mock_instance.join(".socket.sock");
        let _ = fs::write(&mock_sock, b"");

        let resolved = get_socket_dir_from(&temp_dir, Some(mock_sig));
        assert_eq!(resolved, Some(mock_instance));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_rotated_monitor_geometry() {
        // Test transform = 1 (90 degrees, width/height swapped)
        let json_rotated = r#"{"x": 500, "y": 800}
[
    {
        "id": 0,
        "name": "DP-1",
        "x": 0,
        "y": 0,
        "width": 1920,
        "height": 1080,
        "scale": 1.0,
        "transform": 1,
        "focused": true
    }
]"#;
        let res = parse_cursor_and_monitors_from_slice(json_rotated.as_bytes()).unwrap();
        let (cx, cy, mx, my, mw, mh, name) = res;
        assert_eq!(cx, 500);
        assert_eq!(cy, 800);
        assert_eq!(mx, 0);
        assert_eq!(my, 0);
        assert_eq!(mw, 1080);
        assert_eq!(mh, 1920);
        assert_eq!(name, "DP-1");

        // Test transform = 3 (270 degrees), scale = 2.0, cursor outside falling back to focused
        let json_outside = r#"{"x": 5000, "y": 5000}
[
    {
        "id": 0,
        "name": "DP-2",
        "x": 100,
        "y": 200,
        "width": 2560,
        "height": 1440,
        "scale": 2.0,
        "transform": 3,
        "focused": true
    }
]"#;
        let res_outside = parse_cursor_and_monitors_from_slice(json_outside.as_bytes()).unwrap();
        let (cx2, cy2, mx2, my2, mw2, mh2, name2) = res_outside;
        assert_eq!(cx2, 5000);
        assert_eq!(cy2, 5000);
        assert_eq!(mx2, 100);
        assert_eq!(my2, 200);
        // logical width = 1440 / 2 = 720, logical height = 2560 / 2 = 1280
        assert_eq!(mw2, 720);
        assert_eq!(mh2, 1280);
        assert_eq!(name2, "DP-2");
    }
}
