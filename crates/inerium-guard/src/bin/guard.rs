use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir())
}

fn find_k10temp_input() -> Option<PathBuf> {
    let entries = fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(name) = fs::read_to_string(p.join("name")) {
            if name.trim() == "k10temp" {
                let input = p.join("temp1_input");
                if input.exists() {
                    return Some(input);
                }
            }
        }
    }
    None
}

fn handle_session_start() -> Value {
    let mut parts = Vec::new();

    // 1. GPU mode via envycontrol
    if let Ok(output) = Command::new("envycontrol").arg("--query").output() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            parts.push(format!("GPU: {}", s));
        }
    }

    // 2. CPU temp via MSI EC sysfs
    let ec_temp = Path::new("/sys/devices/platform/msi-ec/cpu/realtime_temperature");
    if let Ok(content) = fs::read_to_string(ec_temp) {
        let t = content.trim();
        if !t.is_empty() {
            parts.push(format!("CPU: {}°C", t));
        }
    } else if let Some(k10) = find_k10temp_input() {
        if let Ok(content) = fs::read_to_string(k10) {
            if let Ok(millis) = content.trim().parse::<i32>() {
                parts.push(format!("CPU: {}°C", millis / 1000));
            }
        }
    }

    // 3. Failed system units (exact unit names)
    let mut failed_units: Vec<String> = Vec::new();
    if let Ok(output) = Command::new("systemctl")
        .args(["--failed", "--no-legend"])
        .output()
    {
        let failed = String::from_utf8_lossy(&output.stdout);
        for line in failed.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(unit) = words.iter().find(|w| w.ends_with(".service") || w.ends_with(".socket") || w.ends_with(".target") || w.ends_with(".timer")) {
                failed_units.push((*unit).to_string());
            }
        }
    }

    if !failed_units.is_empty() {
        parts.push(format!("Warning: Failed unit(s) [{}]", failed_units.join(", ")));
    } else {
        parts.push("System units: All OK".to_string());
    }

    let msg = if parts.is_empty() {
        "Inerium Environment (transient): Ready.".to_string()
    } else {
        format!("Inerium Environment (transient): {}", parts.join(" | "))
    };

    // Use ephemeralMessage so opening state is transient and does not pollute future context turns
    json!({
        "injectSteps": [
            {
                "ephemeralMessage": msg
            }
        ]
    })
}

fn handle_pre_tool(payload: &Value) -> Value {
    let tool_call = &payload["toolCall"];
    let name = tool_call["name"].as_str().unwrap_or("");
    let args = &tool_call["args"];

    let path_str = args["Path"]
        .as_str()
        .or_else(|| args["path"].as_str())
        .unwrap_or("");
    let cmd_str = args["CommandLine"]
        .as_str()
        .or_else(|| args["command"].as_str())
        .unwrap_or("");

    // Check protected file paths on write/edit
    if name == "write_file" || name == "edit_file" {
        let home = get_home_dir();
        let protected_subpaths = [
            ".ssh",
            ".gnupg",
            ".config/hypr/config/autostart_backup.lua",
            ".config/hypr/config/binds_backup.lua",
            ".config/hypr/backup",
        ];

        let target_path = PathBuf::from(path_str);
        let normalized = if let Ok(canon) = target_path.canonicalize() {
            canon
        } else {
            target_path.clone()
        };
        let norm_str = normalized.to_string_lossy();

        for sub in protected_subpaths {
            let full_protected = home.join(sub);
            let full_str = full_protected.to_string_lossy();
            if norm_str.starts_with(&*full_str) || norm_str.contains(sub) {
                return json!({
                    "decision": "deny",
                    "reason": format!("Security Gate: Modifying protected path ({}) is strictly prohibited.", sub)
                });
            }
        }
    }

    // Check command line for dangerous destructive commands
    if name == "run_command" && !cmd_str.is_empty() {
        let trimmed = cmd_str.trim();

        // 1. Root / home full wipe checks
        let is_rm = trimmed.starts_with("rm ") || trimmed.contains(" rm ");
        if is_rm {
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            let is_recursive = tokens.iter().any(|t| t.starts_with('-') && (t.contains('r') || t.contains('R')));
            let is_forced = tokens.iter().any(|t| t.starts_with('-') && t.contains('f'));

            if is_recursive && is_forced {
                let home_str = get_home_dir().to_string_lossy().into_owned();
                let home_slash = format!("{}/", home_str);
                let home_star = format!("{}/*", home_str);
                for token in &tokens {
                    let clean = token.trim_matches(['"', '\'']);
                    if clean == "/" || clean == "/*" || clean == "~" || clean == "~/" || clean == "~/*"
                        || clean == home_str || clean == home_slash || clean == home_star
                        || clean == "$HOME" || clean == "$HOME/" || clean == "$HOME/*"
                    {
                        return json!({
                            "decision": "deny",
                            "reason": format!("Security Gate: Dangerous root/home deletion command blocked: {}", trimmed)
                        });
                    }
                }
            }

            // Destructive removal of fallback session configs
            if (trimmed.contains("autostart_backup") || trimmed.contains("binds_backup"))
                && (trimmed.contains("rm ") || trimmed.contains("mv "))
            {
                return json!({
                    "decision": "deny",
                    "reason": "Security Gate: Deleting or moving fallback session configs is prohibited."
                });
            }
        }

        // 2. Raw disk operations
        if trimmed.starts_with("mkfs") || trimmed.contains(" mkfs") {
            return json!({
                "decision": "deny",
                "reason": "Security Gate: Filesystem formatting (mkfs) is blocked."
            });
        }

        if (trimmed.starts_with("dd ") || trimmed.contains(" dd "))
            && (trimmed.contains("of=/dev/nvme") || trimmed.contains("of=/dev/sd"))
        {
            return json!({
                "decision": "deny",
                "reason": "Security Gate: Raw block device write (dd) is blocked."
            });
        }
    }

    json!({ "decision": "allow" })
}

fn handle_post_tool(_payload: &Value) -> Value {
    // PostToolUse returns empty JSON object per hook contract
    json!({})
}

fn handle_stop(payload: &Value) -> Value {
    let reason = payload["terminationReason"].as_str().unwrap_or("");

    // Validate turn completion when model finishes
    if reason == "NO_TOOL_CALL" {
        let home = get_home_dir();

        // 1. Validate Lua syntax if Hyprland Lua files were modified
        let hypr_dir = home.join(".config/hypr");
        let mut modified_lua_files = Vec::new();
        if let Ok(output) = Command::new("git")
            .args(["-C", hypr_dir.to_str().unwrap_or(""), "status", "--porcelain"])
            .output()
        {
            let status_str = String::from_utf8_lossy(&output.stdout);
            for line in status_str.lines() {
                let trimmed = line.trim();
                if trimmed.ends_with(".lua") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if let Some(rel) = parts.last() {
                        let path = hypr_dir.join(rel);
                        if path.is_file() {
                            modified_lua_files.push(path);
                        }
                    }
                }
            }
        }

        if !modified_lua_files.is_empty() {
            let mut errors = Vec::new();
            for f in modified_lua_files {
                if let Ok(output) = Command::new("luac").arg("-p").arg(&f).output() {
                    if !output.status.success() {
                        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let filename = f.file_name().unwrap_or_default().to_string_lossy();
                        errors.push(format!("{}: {}", filename, err_msg));
                    }
                }
            }

            if !errors.is_empty() {
                return json!({
                    "decision": "continue",
                    "reason": format!(
                        "Linter Gate: Syntax check failed on modified Hyprland configuration:\n{}\nPlease fix these syntax errors before completing the turn.",
                        errors.join("\n")
                    )
                });
            }
        }

        // 2. Enforce Definition of Done: system_docs must be committed cleanly
        let docs_dir = home.join(".gemini/system_docs");
        if let Ok(output) = Command::new("git")
            .args(["-C", docs_dir.to_str().unwrap_or(""), "status", "--porcelain", "."])
            .output()
        {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !status.is_empty() {
                return json!({
                    "decision": "continue",
                    "reason": format!(
                        "Definition of Done Gate: Uncommitted changes detected in ~/.gemini/system_docs:\n{}\nPlease stage and commit system documentation atomically (git -C ~/.gemini/system_docs add -A && git -C ~/.gemini/system_docs commit -m \"docs: sync changes\") before completing.",
                        status
                    )
                });
            }
        }
    }

    json!({ "decision": "allow" })
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let payload: Value = serde_json::from_str(&input).unwrap_or(Value::Null);

    let event = env::args().nth(1).unwrap_or_default();

    let resp = match event.as_str() {
        "pre_tool" => handle_pre_tool(&payload),
        "post_tool" => handle_post_tool(&payload),
        "stop" => handle_stop(&payload),
        "session_start" => handle_session_start(),
        _ => {
            if payload.get("toolCall").is_some() {
                handle_pre_tool(&payload)
            } else if payload.get("terminationReason").is_some() {
                handle_stop(&payload)
            } else if payload.get("conversationId").is_some() {
                handle_session_start()
            } else {
                json!({})
            }
        }
    };

    println!("{}", serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()));
}
