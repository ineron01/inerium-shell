use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::model::{
    FirstLoginStep, HyprMonitor, IdleConfig, InputOverrides, MonitorConfig, SetupPaths, StepStatus,
    atomic_write, extract_gestures_lua, generate_hypridle_conf, generate_hypridle_dropin,
    generate_inputs_lua, generate_inputs_lua_with_gestures, generate_monitors_lua,
    list_executable_steps, mark_step_done, parse_hypr_monitors_json, replace_launcher_bind,
    step_done_marker,
};

pub fn query_hypr_monitors() -> Result<Vec<HyprMonitor>, String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|e| format!("Failed executing hyprctl: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "hyprctl monitors -j failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    parse_hypr_monitors_json(&raw)
}

pub fn hyprctl_reload() -> Result<(), String> {
    let output = Command::new("hyprctl")
        .arg("reload")
        .output()
        .map_err(|e| format!("Failed executing hyprctl reload: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "hyprctl reload failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

pub fn query_live_inputs() -> InputOverrides {
    let mut cfg = InputOverrides::default();
    let batch_cmd = "getoption input:kb_layout; \
                     getoption input:kb_options; \
                     getoption input:sensitivity; \
                     getoption input:accel_profile; \
                     getoption input:scroll_factor; \
                     getoption input:touchpad:natural_scroll; \
                     getoption input:touchpad:tap_to_click; \
                     getoption input:touchpad:disable_while_typing";
    let Ok(output) = Command::new("hyprctl")
        .args(["-j", "--batch", batch_cmd])
        .output()
    else {
        return cfg;
    };
    if !output.status.success() {
        return cfg;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let Some(opt) = val.get("option").and_then(|v| v.as_str()) else {
                continue;
            };
            match opt {
                "input:kb_layout" => {
                    if let Some(s) = val.get("str").and_then(|v| v.as_str())
                        && !s.trim().is_empty()
                    {
                        cfg.kb_layout = s.trim().to_string();
                    }
                }
                "input:kb_options" => {
                    if let Some(s) = val.get("str").and_then(|v| v.as_str())
                        && !s.trim().is_empty()
                    {
                        cfg.kb_options = s.trim().to_string();
                    }
                }
                "input:sensitivity" => {
                    if let Some(f) = val.get("float").and_then(|v| v.as_f64()) {
                        cfg.sensitivity = f.clamp(-1.0, 1.0);
                    }
                }
                "input:accel_profile" => {
                    if let Some(s) = val.get("str").and_then(|v| v.as_str())
                        && !s.trim().is_empty()
                    {
                        cfg.accel_profile = s.trim().to_string();
                    }
                }
                "input:scroll_factor" => {
                    if let Some(f) = val.get("float").and_then(|v| v.as_f64()) {
                        cfg.scroll_factor = f.clamp(0.1, 5.0);
                    }
                }
                "input:touchpad:natural_scroll" => {
                    if let Some(b) = val.get("bool").and_then(|v| v.as_bool()) {
                        cfg.natural_scroll = b;
                    }
                }
                "input:touchpad:tap_to_click" | "input:touchpad:tap-to-click" => {
                    if let Some(b) = val.get("bool").and_then(|v| v.as_bool()) {
                        cfg.tap_to_click = b;
                    }
                }
                "input:touchpad:disable_while_typing" => {
                    if let Some(b) = val.get("bool").and_then(|v| v.as_bool()) {
                        cfg.disable_while_typing = b;
                    }
                }
                _ => {}
            }
        }
    }
    cfg
}

pub fn validate_lua_syntax(content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("inerium_setup_check_{}.lua", std::process::id()));
    fs::write(&tmp, content).map_err(|e| format!("Failed writing temp Lua file: {e}"))?;
    let output = match Command::new("luac").arg("-p").arg(&tmp).output() {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("Warning: `luac` not found in PATH; skipping Lua syntax validation");
            let _ = fs::remove_file(&tmp);
            return Ok(());
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Failed executing luac -p: {e}"));
        }
    };
    let _ = fs::remove_file(&tmp);

    if !output.status.success() {
        return Err(format!(
            "Lua syntax error: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

pub fn is_managed_monitors_overlay(paths: &SetupPaths) -> bool {
    let check_path = |p: std::path::PathBuf| {
        let is_sym = fs::symlink_metadata(&p)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_sym {
            return true;
        }
        fs::read_to_string(&p)
            .map(|s| s.contains("INERIUM_WORKSPACES_OVERRIDDEN"))
            .unwrap_or(false)
    };
    check_path(paths.monitors_lua_path()) || check_path(paths.monitors_local_path())
}

pub fn save_and_reload_monitors(
    paths: &SetupPaths,
    monitors: &[MonitorConfig],
) -> Result<(), String> {
    if is_managed_monitors_overlay(paths) {
        return Err(inerium_i18n::tr(
            "monitors.lua is managed by an overlay (symlink / workspace rules); GUI editing is disabled",
            "monitors.lua управляется оверлеем (симлинк / правила воркспейсов); правка из GUI отключена",
        )
        .to_string());
    }
    let lua = generate_monitors_lua(monitors);
    validate_lua_syntax(&lua)?;
    atomic_write(&paths.monitors_lua_path(), &lua)?;

    // Clean up legacy monitors_local.lua if present
    let local = paths.monitors_local_path();
    if fs::symlink_metadata(&local).is_ok() {
        let _ = fs::remove_file(&local);
    }

    hyprctl_reload()
}

pub fn reset_monitors_to_auto(paths: &SetupPaths) -> Result<(), String> {
    if is_managed_monitors_overlay(paths) {
        return Err(inerium_i18n::tr(
            "monitors.lua is managed by an overlay (symlink / workspace rules); GUI reset is disabled",
            "monitors.lua управляется оверлеем (симлинк / правила воркспейсов); сброс из GUI отключен",
        )
        .to_string());
    }
    let lua = "-- Universal monitor configuration\n-- Wiki: https://wiki.hypr.land/Configuring/Monitors/\n\nhl.monitor({\n    output   = \"\",\n    mode     = \"preferred\",\n    position = \"auto\",\n    scale    = 1.0,\n})\n";
    validate_lua_syntax(lua)?;
    atomic_write(&paths.monitors_lua_path(), lua)?;

    let local = paths.monitors_local_path();
    if fs::symlink_metadata(&local).is_ok() {
        let _ = fs::remove_file(&local);
    }

    hyprctl_reload()
}

pub fn apply_wallpaper(path: &Path) -> Result<(), String> {
    let out = Command::new("inerium-wallpaper")
        .arg(path)
        .arg("all")
        .arg("--no-notify")
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

pub fn save_inputs_and_reload(paths: &SetupPaths, cfg: &InputOverrides) -> Result<(), String> {
    let existing_gestures = fs::read_to_string(paths.input_lua_path())
        .ok()
        .and_then(|c| extract_gestures_lua(&c));
    let lua = match existing_gestures {
        Some(gestures) => generate_inputs_lua_with_gestures(cfg, &gestures),
        None => generate_inputs_lua(cfg),
    };
    validate_lua_syntax(&lua)?;
    atomic_write(&paths.input_lua_path(), &lua)?;

    // Clean up legacy inputs_local.lua
    let legacy_in = paths.inputs_local_path();
    if fs::symlink_metadata(&legacy_in).is_ok() {
        let _ = fs::remove_file(&legacy_in);
    }

    // Direct keybinds.lua update (only if launcher_bind was explicitly configured)
    if let Some(ref launcher_key) = cfg.launcher_bind {
        let keybinds_path = paths.keybinds_lua_path();
        if let Ok(content) = fs::read_to_string(&keybinds_path) {
            let updated_keybinds = replace_launcher_bind(&content, launcher_key);
            validate_lua_syntax(&updated_keybinds)?;
            atomic_write(&keybinds_path, &updated_keybinds)?;
        }
    }

    // Clean up legacy binds_local.lua
    let legacy_binds = paths.binds_local_path();
    if fs::symlink_metadata(&legacy_binds).is_ok() {
        let _ = fs::remove_file(&legacy_binds);
    }

    hyprctl_reload()
}

fn run_systemctl_user(args: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("Failed executing systemctl --user: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "systemctl --user {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

pub fn apply_hypridle(paths: &SetupPaths, cfg: &IdleConfig) -> Result<(), String> {
    let conf_path = paths.hypridle_local_conf_path();
    let dropin_path = paths.hypridle_dropin_path();
    let conf_content = generate_hypridle_conf(cfg);
    atomic_write(&conf_path, &conf_content)?;

    if cfg.enabled {
        let dropin_content = generate_hypridle_dropin(&conf_path);
        atomic_write(&dropin_path, &dropin_content)?;
        run_systemctl_user(&["daemon-reload"])?;
        run_systemctl_user(&["enable", "--now", "hypridle.service"])?;
        run_systemctl_user(&["restart", "hypridle.service"])?;
    } else {
        if fs::symlink_metadata(&dropin_path).is_ok() {
            let _ = fs::remove_file(&dropin_path);
        }
        let _ = run_systemctl_user(&["daemon-reload"]);
        let _ = run_systemctl_user(&["disable", "--now", "hypridle.service"]);
    }
    Ok(())
}

pub fn reset_interface_overrides(paths: &SetupPaths) -> Result<(), String> {
    for p in [
        paths.inputs_local_path(),
        paths.binds_local_path(),
        paths.hypridle_local_conf_path(),
        paths.hypridle_dropin_path(),
    ] {
        if fs::symlink_metadata(&p).is_ok() {
            fs::remove_file(&p).map_err(|e| format!("Failed removing {}: {e}", p.display()))?;
        }
    }

    // Reset input.lua to defaults if present
    let in_lua = generate_inputs_lua(&InputOverrides::default());
    let _ = atomic_write(&paths.input_lua_path(), &in_lua);

    // Reset launcher in keybinds.lua to SUPER + Space
    let kb_path = paths.keybinds_lua_path();
    if let Ok(content) = fs::read_to_string(&kb_path) {
        let reset_kb = replace_launcher_bind(&content, "SUPER + Space");
        let _ = atomic_write(&kb_path, &reset_kb);
    }

    let _ = run_systemctl_user(&["daemon-reload"]);
    let _ = run_systemctl_user(&["enable", "--now", "hypridle.service"]);
    let _ = run_systemctl_user(&["restart", "hypridle.service"]);
    hyprctl_reload()
}

pub fn run_step_check(step_path: &Path) -> bool {
    Command::new(step_path)
        .arg("--check")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn evaluate_steps(paths: &SetupPaths) -> Vec<FirstLoginStep> {
    let steps_dir = paths.first_login_steps_dir();
    let state_dir = paths.first_login_state_dir();
    let mut steps = list_executable_steps(&steps_dir);

    for step in &mut steps {
        let marker = step_done_marker(&state_dir, &step.id);
        if marker.exists() {
            step.status = StepStatus::Configured;
        } else if run_step_check(&step.path) {
            let _ = mark_step_done(&state_dir, &step.id);
            step.status = StepStatus::Configured;
        } else {
            step.status = StepStatus::Pending;
        }
    }

    steps
}

pub fn should_open_autostart(paths: &SetupPaths) -> (bool, bool) {
    if paths.setup_done_path().exists() {
        return (false, false);
    }
    (true, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::mark_setup_done;

    #[test]
    fn autostart_opens_only_before_setup_done() {
        let base = std::env::temp_dir().join(format!("inerium_autostart_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let paths = SetupPaths {
            home: base.clone(),
            config_home: base.join(".config"),
            state_home: base.join(".local/state"),
        };

        // Even with a pending step in first-login.d, once setup.done exists, autostart stays closed
        let step_dir = paths.first_login_steps_dir();
        std::fs::create_dir_all(&step_dir).unwrap();
        let step = step_dir.join("99-test-step");
        std::fs::write(&step, "#!/bin/sh\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&step, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert_eq!(should_open_autostart(&paths), (true, false));

        mark_setup_done(&paths).unwrap();
        assert_eq!(should_open_autostart(&paths), (false, false));

        let _ = std::fs::remove_dir_all(&base);
    }
}
