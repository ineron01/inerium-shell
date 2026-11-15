use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::Command;

const SERVER_NAME: &str = "inerium-system-mcp";
const SERVER_VERSION: &str = "1.1.0";
const PROTOCOL_VERSION: &str = "2024-11-05";

fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir())
}

fn run_cmd(cmd: &str, args: &[&str]) -> (String, String, i32) {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let code = output.status.code().unwrap_or(0);
            (stdout, stderr, code)
        }
        Err(e) => (String::new(), e.to_string(), 1),
    }
}

fn tool_get_system_health() -> String {
    let mut out = Vec::new();

    let (uptime_out, _, _) = run_cmd("uptime", &[]);
    out.push(format!("=== Uptime & Load ===\n{}", uptime_out));

    let (free_out, _, _) = run_cmd("free", &["-h"]);
    out.push(format!("\n=== Memory (RAM) ===\n{}", free_out));

    let (zram_out, _, _) = run_cmd("zramctl", &[]);
    if !zram_out.is_empty() {
        out.push(format!("\n=== ZRAM ===\n{}", zram_out));
    }

    let (sys_failed, _, _) = run_cmd("systemctl", &["--failed", "--no-legend"]);
    out.push(format!(
        "\n=== Failed System Units ===\n{}",
        if sys_failed.is_empty() { "None (All OK)" } else { &sys_failed }
    ));

    let (user_failed, _, _) = run_cmd("systemctl", &["--user", "--failed", "--no-legend"]);
    out.push(format!(
        "\n=== Failed User Units ===\n{}",
        if user_failed.is_empty() { "None (All OK)" } else { &user_failed }
    ));

    out.join("")
}

fn tool_get_gpu_and_power() -> String {
    let mut out = Vec::new();

    let (gpu_mode, _, _) = run_cmd("envycontrol", &["--query"]);
    out.push(format!("Envycontrol Graphics Mode: {}", gpu_mode));

    let dgpu_path = "/sys/bus/pci/devices/0000:01:00.0/power/runtime_status";
    if let Ok(status) = fs::read_to_string(dgpu_path) {
        out.push(format!("RTX 4050 PCI Runtime Status: {}", status.trim()));
    }

    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let path = entry.path();
            let ptype = fs::read_to_string(path.join("type")).unwrap_or_default();
            if ptype.trim() == "Battery" {
                if let (Ok(cap), Ok(stat)) = (
                    fs::read_to_string(path.join("capacity")),
                    fs::read_to_string(path.join("status")),
                ) {
                    out.push(format!("Battery: {}% ({})", cap.trim(), stat.trim()));
                    break;
                }
            }
        }
    }

    out.join("\n")
}

fn tool_get_workspace_git_status() -> String {
    let home = get_home_dir();
    let pub_repo = if home.join("Repos/inerium-shell/.git").exists() {
        home.join("Repos/inerium-shell")
    } else {
        home.join("Repos/inerium-desktop")
    };
    let mut search_dirs: Vec<PathBuf> = vec![pub_repo];
    if home.join(".local/src/inerium/.git").exists() {
        search_dirs.insert(0, home.join(".local/src/inerium"));
    }

    let mut results = Vec::new();
    for d in search_dirs {
        let rel_path = d.to_string_lossy().replace(&home.to_string_lossy().to_string(), "~");
        if !d.join(".git").exists() {
            results.push(format!("? {}: Not a git repository", rel_path));
            continue;
        }
        let (branch, _, _) = run_cmd("git", &["-C", d.to_str().unwrap_or(""), "branch", "--show-current"]);
        let (status, _, _) = run_cmd("git", &["-C", d.to_str().unwrap_or(""), "status", "--porcelain"]);

        if status.is_empty() {
            results.push(format!("✓ {} [{}]: Clean", rel_path, branch));
        } else {
            let lines: Vec<&str> = status.lines().collect();
            let mut summary = format!("✗ {} [{}]: {} uncommitted change(s)\n  ", rel_path, branch, lines.len());
            let preview: Vec<&str> = lines.iter().take(10).copied().collect();
            summary.push_str(&preview.join("\n  "));
            if lines.len() > 10 {
                summary.push_str(&format!("\n  ... and {} more", lines.len() - 10));
            }
            results.push(summary);
        }
    }

    results.join("\n")
}

fn tool_get_hardware_thermals() -> String {
    let (sensors_out, _, rc) = run_cmd("sensors", &[]);
    if rc != 0 || sensors_out.is_empty() {
        return "Sensors utility not available or empty.".to_string();
    }

    let mut out = vec!["=== Hardware Sensors ===".to_string()];
    let mut current_chip = String::new();

    for line in sensors_out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !trimmed.contains(':') {
            current_chip = trimmed.to_string();
            continue;
        }

        if trimmed.starts_with("Adapter:") {
            continue;
        }

        if trimmed.starts_with("Tctl:")
            || trimmed.starts_with("Composite:")
            || trimmed.starts_with("edge:")
            || trimmed.starts_with("temp1:")
            || trimmed.starts_with("in0:")
        {
            let chip_prefix = if !current_chip.is_empty() {
                format!("[{}] ", current_chip)
            } else {
                String::new()
            };
            out.push(format!("{}{}", chip_prefix, trimmed));
        }
    }
    out.join("\n")
}

fn tool_check_hyprland_lua_syntax() -> String {
    let home = get_home_dir();
    let hypr_dir = home.join(".config/hypr");
    let mut lua_files = vec![hypr_dir.join("hyprland.lua")];

    let config_dir = hypr_dir.join("config");
    if let Ok(entries) = fs::read_dir(config_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|ext| ext == "lua") {
                lua_files.push(p);
            }
        }
    }
    lua_files.sort();

    let mut errors = Vec::new();
    let mut checked = 0;
    for f in &lua_files {
        if !f.is_file() {
            continue;
        }
        checked += 1;
        let (_, err, rc) = run_cmd("luac", &["-p", f.to_str().unwrap_or("")]);
        if rc != 0 {
            let rel = f.to_string_lossy().replace(&home.to_string_lossy().to_string(), "~");
            errors.push(format!("Error in {}:\n{}", rel, err));
        }
    }

    if errors.is_empty() {
        format!("✓ All {} Hyprland Lua files passed syntax validation (luac -p).", checked)
    } else {
        format!("✗ Syntax check failed in {} file(s):\n{}", errors.len(), errors.join("\n"))
    }
}

fn tool_get_service_logs(args: &Value) -> String {
    let unit = args["unit"].as_str().unwrap_or("").trim();
    if unit.is_empty() {
        return "Error: 'unit' argument is required (e.g. 'inerium-shell' or 'novpn').".to_string();
    }
    let unit_name = if unit.ends_with(".service") {
        unit.to_string()
    } else {
        format!("{}.service", unit)
    };

    let lines = args["lines"].as_i64().unwrap_or(20).clamp(1, 50);
    let is_user = args["user"].as_bool().unwrap_or(true);

    let lines_str = lines.to_string();
    let cmd_args = if is_user {
        vec!["--no-pager", "-n", &lines_str, "--user", "-u", &unit_name]
    } else {
        vec!["--no-pager", "-n", &lines_str, "-u", &unit_name]
    };

    let (out, err, rc) = run_cmd("journalctl", &cmd_args);
    if rc != 0 {
        format!("Error retrieving logs for {}: {}", unit_name, err)
    } else if out.is_empty() {
        format!("No logs found for unit {}.", unit_name)
    } else {
        format!("=== Journal logs for {} (last {} lines) ===\n{}", unit_name, lines, out)
    }
}

fn tool_get_hyprland_state() -> String {
    let mut out = Vec::new();

    // Active window
    let (win_raw, _, rc_w) = run_cmd("hyprctl", &["-j", "activewindow"]);
    if rc_w == 0 && !win_raw.is_empty() {
        if let Ok(w) = serde_json::from_str::<Value>(&win_raw) {
            let title = w["title"].as_str().unwrap_or("N/A");
            let cls = w["class"].as_str().unwrap_or("N/A");
            let ws = w["workspace"]["name"].as_str().unwrap_or("N/A");
            let flt = if w["floating"].as_bool().unwrap_or(false) { "floating" } else { "tiled" };
            let sz_w = w["size"][0].as_i64().unwrap_or(0);
            let sz_h = w["size"][1].as_i64().unwrap_or(0);
            out.push(format!("Active Window: [{}] \"{}\" (workspace: {}, {}, size: {}x{})", cls, title, ws, flt, sz_w, sz_h));
        }
    }

    // Monitors
    let (mon_raw, _, rc_m) = run_cmd("hyprctl", &["-j", "monitors"]);
    if rc_m == 0 && !mon_raw.is_empty() {
        if let Ok(Value::Array(mons)) = serde_json::from_str::<Value>(&mon_raw) {
            out.push("Monitors:".to_string());
            for m in mons {
                let name = m["name"].as_str().unwrap_or("N/A");
                let w = m["width"].as_i64().unwrap_or(0);
                let h = m["height"].as_i64().unwrap_or(0);
                let hz = m["refreshRate"].as_f64().unwrap_or(0.0);
                let scale = m["scale"].as_f64().unwrap_or(1.0);
                let ws = m["activeWorkspace"]["name"].as_str().unwrap_or("N/A");
                let focused = if m["focused"].as_bool().unwrap_or(false) { " [FOCUSED]" } else { "" };
                out.push(format!("  - {}: {}x{}@{:.1}Hz (scale {:.1}, ws {}){}", name, w, h, hz, scale, ws, focused));
            }
        }
    }

    // Open Windows / Clients
    let (clients_raw, _, rc_c) = run_cmd("hyprctl", &["-j", "clients"]);
    if rc_c == 0 && !clients_raw.is_empty() {
        if let Ok(Value::Array(clients)) = serde_json::from_str::<Value>(&clients_raw) {
            let valid_clients: Vec<&Value> = clients
                .iter()
                .filter(|c| {
                    let mapped = c["mapped"].as_bool().unwrap_or(true);
                    let title = c["title"].as_str().unwrap_or("");
                    mapped && !title.is_empty()
                })
                .collect();

            if !valid_clients.is_empty() {
                out.push(format!("Open Windows ({} total, top 10):", valid_clients.len()));
                for c in valid_clients.iter().take(10) {
                    let cls = c["class"].as_str().unwrap_or("N/A");
                    let title = c["title"].as_str().unwrap_or("N/A");
                    let ws = c["workspace"]["name"].as_str().unwrap_or("N/A");
                    let flt = if c["floating"].as_bool().unwrap_or(false) {
                        "floating"
                    } else {
                        "tiled"
                    };
                    out.push(format!("  - [{}] \"{}\" (ws: {}, {})", cls, title, ws, flt));
                }
            }
        }
    }

    if out.is_empty() {
        "Hyprland session not reachable.".to_string()
    } else {
        out.join("\n")
    }
}

fn tool_get_git_diff(args: &Value) -> String {
    let home = get_home_dir();
    let repo_str = args["path"].as_str().unwrap_or("").trim();
    let repo_dir = if repo_str.is_empty() {
        home.join(".gemini/system_docs")
    } else if repo_str.starts_with("~/") {
        home.join(repo_str.trim_start_matches("~/"))
    } else {
        PathBuf::from(repo_str)
    };

    if !repo_dir.exists() {
        return format!("Error: Repository path does not exist: {:?}", repo_dir);
    }

    let repo_path_str = repo_dir.to_str().unwrap_or("");
    let lines = args["lines"].as_i64().unwrap_or(35).clamp(5, 70);
    let staged = args["staged"].as_bool().unwrap_or(false);

    let stat_args = if staged {
        vec!["-C", repo_path_str, "diff", "--cached", "--stat"]
    } else {
        vec!["-C", repo_path_str, "diff", "--stat"]
    };

    let (stat_out, _, rc_stat) = run_cmd("git", &stat_args);
    if rc_stat != 0 {
        return format!("Error running git diff --stat on {:?}", repo_dir);
    }

    let diff_args = if staged {
        vec!["-C", repo_path_str, "diff", "--cached", "-U2"]
    } else {
        vec!["-C", repo_path_str, "diff", "-U2"]
    };

    let (diff_out, _, _) = run_cmd("git", &diff_args);
    let diff_lines: Vec<&str> = diff_out.lines().take(lines as usize).collect();

    let diff_type = if staged { "staged (--cached)" } else { "unstaged" };
    let mut out = Vec::new();
    out.push(format!("=== Git Diff Stat ({}) for {:?} ===", diff_type, repo_dir));
    if stat_out.is_empty() {
        out.push(format!("No {} changes.", diff_type));
    } else {
        out.push(stat_out);
    }

    if !diff_lines.is_empty() {
        out.push(format!(
            "\n=== Bounded Diff ({}, first {} lines) ===\n{}",
            diff_type,
            diff_lines.len(),
            diff_lines.join("\n")
        ));
        let total_lines = diff_out.lines().count();
        if total_lines > lines as usize {
            out.push(format!("\n... truncated (total lines: {})", total_lines));
        }
    }

    out.join("\n")
}

fn tool_get_msi_ec_telemetry() -> String {
    let ec_path = Path::new("/sys/devices/platform/msi-ec");
    if !ec_path.exists() {
        return "MSI EC sysfs interface not found (/sys/devices/platform/msi-ec).".to_string();
    }

    let read_node = |rel: &str| -> String {
        fs::read_to_string(ec_path.join(rel))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "N/A".to_string())
    };

    let cpu_temp = read_node("cpu/realtime_temperature");
    let cpu_fan = read_node("cpu/realtime_fan_speed");
    let gpu_temp = read_node("gpu/realtime_temperature");
    let gpu_fan = read_node("gpu/realtime_fan_speed");
    let fan_mode = read_node("fan_mode");
    let cooler_boost = read_node("cooler_boost");
    let super_bat = read_node("super_battery");

    [
        "=== MSI EC Hardware Telemetry ===".to_string(),
        format!("CPU: {}°C | Fan: {}%", cpu_temp, cpu_fan),
        format!("GPU: {}°C | Fan: {}%", gpu_temp, gpu_fan),
        format!("Fan Mode: {} (CoolerBoost: {})", fan_mode, cooler_boost),
        format!("Super Battery Mode: {}", super_bat),
    ]
    .join("\n")
}

fn tool_restart_user_service(args: &Value) -> String {
    let mut service = args["service"].as_str().unwrap_or("").trim().to_string();
    if service.ends_with(".service") {
        service = service.trim_end_matches(".service").to_string();
    }

    let safe_services: HashSet<&str> = [
        "inerium-shell", "inerium-popups", "inerium-polkit", "hypr-ipc-bridge",
        "waybar", "mako", "swayosd-server", "inerium-desktop-indexer", "gdrive",
        "cliphist", "cliphist-images", "inerium-updates",
    ]
    .iter()
    .cloned()
    .collect();

    if !safe_services.contains(service.as_str()) {
        let mut sorted: Vec<&str> = safe_services.into_iter().collect();
        sorted.sort();
        return format!("Error: '{}' is not in allowed safe user services: {:?}", service, sorted);
    }

    let service_unit = format!("{}.service", service);
    let (_, err, rc) = run_cmd("systemctl", &["--user", "restart", &service_unit]);
    if rc == 0 {
        format!("✓ Successfully restarted {}.", service_unit)
    } else {
        format!("✗ Failed to restart {}: {}", service_unit, err)
    }
}

fn tool_get_package_info(args: &Value) -> String {
    let pkg = args["package"].as_str().unwrap_or("").trim();
    if pkg.is_empty() {
        return "Error: 'package' parameter is required.".to_string();
    }
    if !pkg.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+') {
        return "Error: Invalid package name characters.".to_string();
    }

    let action = args["action"].as_str().unwrap_or("info");
    if action == "search" {
        let (out, err, rc) = run_cmd("pacman", &["-Ss", pkg]);
        if rc != 0 && out.is_empty() {
            return format!("Package search failed: {}", err);
        }
        let lines: Vec<&str> = out.lines().take(20).collect();
        format!("=== Pacman Search for '{}' ===\n{}", pkg, lines.join("\n"))
    } else {
        let (out, _, rc) = run_cmd("pacman", &["-Qi", pkg]);
        if rc == 0 {
            format!("=== Installed Package Info ({}) ===\n{}", pkg, out)
        } else {
            let (sync_out, sync_err, sync_rc) = run_cmd("pacman", &["-Si", pkg]);
            if sync_rc == 0 {
                format!("=== Repo Package Info ({}) ===\n{}", pkg, sync_out)
            } else {
                format!("Package '{}' not found in local db or repos: {}", pkg, sync_err)
            }
        }
    }
}

fn tool_get_disk_usage() -> String {
    let mut out = Vec::new();
    let home = get_home_dir();
    let home_str = home.to_string_lossy();
    let (df_out, _, _) = run_cmd("df", &["-h", "/", &home_str]);
    out.push(format!("=== Filesystem Disk Space (df -h) ===\n{}", df_out));

    let (btrfs_out, _, rc) = run_cmd("btrfs", &["filesystem", "df", "/"]);
    if rc == 0 && !btrfs_out.is_empty() {
        out.push(format!("\n=== Btrfs Allocation (btrfs fi df /) ===\n{}", btrfs_out));
    }

    out.join("")
}

fn tool_get_journal_errors(args: &Value) -> String {
    let lines = args["lines"].as_i64().unwrap_or(25).clamp(1, 50);
    let lines_str = lines.to_string();
    let boot = args["boot"].as_str().unwrap_or("current");
    let boot_flag = if boot == "previous" { "-1" } else { "0" };

    let (out, err, rc) = run_cmd(
        "journalctl",
        &["-p", "err", "-b", boot_flag, "-n", &lines_str, "--no-pager"],
    );
    if rc != 0 {
        format!("Error querying journal errors: {}", err)
    } else if out.is_empty() {
        format!("No system errors found in journal (priority: err, boot: {}).", boot)
    } else {
        const MAX_TOTAL_CHARS: usize = 4000;
        const MAX_LINE_CHARS: usize = 160;

        let mut processed_lines = Vec::new();
        let mut total_chars = 0;
        let mut truncated_count = 0;

        for line in out.lines() {
            let truncated_line = if line.chars().count() > MAX_LINE_CHARS {
                let prefix: String = line.chars().take(MAX_LINE_CHARS).collect();
                format!("{}... [line truncated]", prefix)
            } else {
                line.to_string()
            };

            let line_len = truncated_line.len();
            if total_chars + line_len > MAX_TOTAL_CHARS {
                truncated_count += 1;
                continue;
            }

            total_chars += line_len + 1;
            processed_lines.push(truncated_line);
        }

        let mut result = format!(
            "=== System Errors in Journal (last {} entries, boot: {}) ===\n{}",
            lines,
            boot,
            processed_lines.join("\n")
        );

        if truncated_count > 0 {
            result.push_str(&format!(
                "\n... [omitted {} tail entries exceeding context budget]",
                truncated_count
            ));
        }

        result
    }
}

fn tool_get_audio_status() -> String {
    let (wp_out, wp_err, rc) = run_cmd("wpctl", &["status"]);
    if rc != 0 || wp_out.is_empty() {
        return format!("Failed to run wpctl status: {}", wp_err);
    }

    let mut out = vec!["=== PipeWire / Audio Status ===".to_string()];
    let mut in_audio = false;

    for line in wp_out.lines() {
        if line.starts_with("Audio") {
            in_audio = true;
            continue;
        }
        if in_audio && (line.starts_with("Video") || line.starts_with("Settings")) {
            break;
        }
        if in_audio {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("├─ Sinks:")
                || trimmed.starts_with("├─ Sources:")
                || trimmed.starts_with("└─ Streams:")
                || line.contains('*')
                || trimmed.starts_with("[vol:")
            {
                out.push(line.to_string());
            }
        }
    }

    if out.len() <= 1 {
        out.push("No active sinks or sources detected in Audio subsystem.".to_string());
    }

    out.join("\n")
}

fn tool_get_network_status() -> String {
    let mut out = vec!["=== Network Status & Routing ===".to_string()];

    let (ip_out, _, _) = run_cmd("ip", &["-br", "addr"]);
    if !ip_out.is_empty() {
        out.push("\nInterfaces:".to_string());
        for line in ip_out.lines() {
            out.push(format!("  {}", line));
        }
    }

    let (route_out, _, _) = run_cmd("ip", &["route", "show", "default"]);
    if !route_out.is_empty() {
        out.push(format!("\nDefault Route:\n  {}", route_out.replace('\n', "\n  ")));
    }

    let (hotspot_out, _, _) = run_cmd("inerium-hotspot", &["status-plain"]);
    if !hotspot_out.is_empty() {
        out.push(format!("\nWi-Fi Hotspot: {}", hotspot_out));
    }

    let (novpn_test, _, rc_novpn) = run_cmd("novpn", &["ip", "route", "show", "default"]);
    if rc_novpn == 0 && !novpn_test.is_empty() {
        out.push(format!("\nnovpn direct namespace route: {}", novpn_test.trim()));
    } else {
        out.push("\nnovpn direct namespace: inactive or not configured".to_string());
    }

    out.join("\n")
}

fn get_tools_manifest() -> Value {
    json!([
        {
            "name": "get_system_health",
            "description": "Returns CachyOS system load, memory, ZRAM status, and any failed systemd units (both system and user level).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_gpu_and_power",
            "description": "Returns graphics state (integrated/hybrid/discrete via envycontrol), RTX 4050 PCI runtime status, and battery capacity/charging state.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_workspace_git_status",
            "description": "Checks Git status across workspace repositories (~/.local/src/inerium and ~/Repos/inerium-shell) showing uncommitted files and current branches.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_hardware_thermals",
            "description": "Returns key thermal and voltage sensors (CPU Tctl, NVMe Composite, Wi-Fi thermals) from the system sensors subsystem.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_msi_ec_telemetry",
            "description": "Reads hardware telemetry directly from the MSI Embedded Controller (msi-ec): CPU/GPU temperatures, real-time fan RPM %, fan mode, cooler boost, and super battery mode.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_hyprland_state",
            "description": "Returns active Hyprland window details (title, class, workspace, size), connected monitors layout, and a compact list of open windows.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_service_logs",
            "description": "Retrieves recent journalctl logs for a specific systemd unit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "unit": {
                        "type": "string",
                        "description": "Name of the systemd unit (e.g. 'inerium-shell', 'waybar', 'gdrive')"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of journal lines to fetch (default: 20, max: 50)"
                    },
                    "user": {
                        "type": "boolean",
                        "description": "Whether to query user journal (--user) or system journal. Defaults to true."
                    }
                },
                "required": ["unit"],
                "additionalProperties": false
            }
        },
        {
            "name": "restart_user_service",
            "description": "Safely restarts a recognized user desktop service (waybar, inerium-shell, mako, swayosd, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name of the service (e.g. 'inerium-shell', 'waybar', 'mako')"
                    }
                },
                "required": ["service"],
                "additionalProperties": false
            }
        },
        {
            "name": "check_hyprland_lua_syntax",
            "description": "Validates the Lua syntax of all Hyprland configuration files using luac -p before reloading the compositor.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_package_info",
            "description": "Inspects installed or available Arch/CachyOS packages using pacman (query info or search).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "package": {
                        "type": "string",
                        "description": "Name of the package to query or search"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["info", "search"],
                        "description": "Whether to query package details ('info', default) or search repos ('search')"
                    }
                },
                "required": ["package"],
                "additionalProperties": false
            }
        },
        {
            "name": "get_disk_usage",
            "description": "Returns filesystem disk space usage (df -h) and Btrfs allocation status.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_journal_errors",
            "description": "Retrieves recent system error log entries (priority err) from journalctl for the current or previous boot.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "lines": {
                        "type": "integer",
                        "description": "Number of log lines to retrieve (default: 25, max: 50)"
                    },
                    "boot": {
                        "type": "string",
                        "enum": ["current", "previous"],
                        "description": "Boot to query: 'current' (default) or 'previous'"
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "get_git_diff",
            "description": "Returns a bounded git diff (--stat and first N lines of diff) for a repository to inspect changes without risking massive prompt token consumption.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to git repository (defaults to '~/.gemini/system_docs', or specify e.g. '~/.config/hypr')"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Maximum number of diff lines to return (default: 35, max: 70)"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "Whether to inspect staged (--cached) changes instead of unstaged changes (default: false)"
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "get_audio_status",
            "description": "Returns PipeWire/WirePlumber audio status (active sinks, sources, volume levels, mute states).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "get_network_status",
            "description": "Returns network interfaces, default routes, Wi-Fi hotspot status, and novpn direct isolation namespace routing.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }
    ])
}

fn handle_tool_call(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "get_system_health" => Ok(tool_get_system_health()),
        "get_gpu_and_power" => Ok(tool_get_gpu_and_power()),
        "get_workspace_git_status" => Ok(tool_get_workspace_git_status()),
        "get_git_diff" => Ok(tool_get_git_diff(args)),
        "get_hardware_thermals" => Ok(tool_get_hardware_thermals()),
        "get_msi_ec_telemetry" => Ok(tool_get_msi_ec_telemetry()),
        "get_hyprland_state" => Ok(tool_get_hyprland_state()),
        "get_service_logs" => Ok(tool_get_service_logs(args)),
        "restart_user_service" => Ok(tool_restart_user_service(args)),
        "check_hyprland_lua_syntax" => Ok(tool_check_hyprland_lua_syntax()),
        "get_package_info" => Ok(tool_get_package_info(args)),
        "get_disk_usage" => Ok(tool_get_disk_usage()),
        "get_journal_errors" => Ok(tool_get_journal_errors(args)),
        "get_audio_status" => Ok(tool_get_audio_status()),
        "get_network_status" => Ok(tool_get_network_status()),
        _ => Err(format!("Tool not found: {}", name)),
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l.trim().to_string(),
            Err(_) => break,
        };
        if line.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req["method"].as_str().unwrap_or("");
        let msg_id = req.get("id");

        match method {
            "initialize" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": SERVER_NAME,
                            "version": SERVER_VERSION
                        }
                    }
                });
                let _ = serde_json::to_writer(&mut stdout, &resp);
                let _ = io::Write::write_all(&mut stdout, b"\n");
                let _ = io::Write::flush(&mut stdout);
            }
            "notifications/initialized" => {
                // Client ack
            }
            "ping" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": {}
                });
                let _ = serde_json::to_writer(&mut stdout, &resp);
                let _ = io::Write::write_all(&mut stdout, b"\n");
                let _ = io::Write::flush(&mut stdout);
            }
            "tools/list" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": msg_id,
                    "result": {
                        "tools": get_tools_manifest()
                    }
                });
                let _ = serde_json::to_writer(&mut stdout, &resp);
                let _ = io::Write::write_all(&mut stdout, b"\n");
                let _ = io::Write::flush(&mut stdout);
            }
            "tools/call" => {
                let params = &req["params"];
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                let resp = match handle_tool_call(name, args) {
                    Ok(text) => json!({
                        "jsonrpc": "2.0",
                        "id": msg_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ],
                            "isError": false
                        }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": msg_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": e
                                }
                            ],
                            "isError": true
                        }
                    }),
                };
                let _ = serde_json::to_writer(&mut stdout, &resp);
                let _ = io::Write::write_all(&mut stdout, b"\n");
                let _ = io::Write::flush(&mut stdout);
            }
            _ => {
                if msg_id.is_some() {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": msg_id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {}", method)
                        }
                    });
                    let _ = serde_json::to_writer(&mut stdout, &resp);
                    let _ = io::Write::write_all(&mut stdout, b"\n");
                    let _ = io::Write::flush(&mut stdout);
                }
            }
        }
    }
}
