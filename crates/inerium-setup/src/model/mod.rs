pub mod hardware;
pub mod locale;
pub mod monitors;
pub mod steps;
pub mod wallpapers;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use hardware::*;
pub use locale::*;
pub use monitors::*;
pub use steps::*;
pub use wallpapers::*;

#[derive(Debug, Clone)]
pub struct SetupPaths {
    pub home: PathBuf,
    pub config_home: PathBuf,
    pub state_home: PathBuf,
}

impl SetupPaths {
    pub fn from_env() -> Self {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let state_home = std::env::var("XDG_STATE_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));
        Self {
            home,
            config_home,
            state_home,
        }
    }

    pub fn hypr_dir(&self) -> PathBuf {
        self.config_home.join("hypr")
    }

    pub fn workspaces_count_path(&self) -> PathBuf {
        self.hypr_dir().join("config/workspaces_count")
    }

    pub fn monitors_local_path(&self) -> PathBuf {
        self.hypr_dir().join("config/monitors_local.lua")
    }

    pub fn monitors_lua_path(&self) -> PathBuf {
        self.hypr_dir().join("config/monitors.lua")
    }

    pub fn inputs_local_path(&self) -> PathBuf {
        self.hypr_dir().join("config/inputs_local.lua")
    }

    pub fn input_lua_path(&self) -> PathBuf {
        self.hypr_dir().join("config/input.lua")
    }

    pub fn keybinds_lua_path(&self) -> PathBuf {
        self.hypr_dir().join("config/keybinds.lua")
    }

    pub fn hyprpaper_conf_path(&self) -> PathBuf {
        self.hypr_dir().join("hyprpaper.conf")
    }

    pub fn hypridle_local_conf_path(&self) -> PathBuf {
        self.hypr_dir().join("hypridle_local.conf")
    }

    pub fn hypridle_dropin_path(&self) -> PathBuf {
        self.config_home
            .join("systemd/user/hypridle.service.d/local.conf")
    }

    pub fn locale_conf_path(&self) -> PathBuf {
        self.config_home.join("locale.conf")
    }

    pub fn hyprlock_i18n_conf_path(&self) -> PathBuf {
        self.hypr_dir().join("hyprlock-i18n.conf")
    }

    pub fn binds_local_path(&self) -> PathBuf {
        self.hypr_dir().join("config/binds_local.lua")
    }

    pub fn user_dirs_path(&self) -> PathBuf {
        self.config_home.join("user-dirs.dirs")
    }

    pub fn user_dirs_locale_path(&self) -> PathBuf {
        self.config_home.join("user-dirs.locale")
    }

    pub fn inerium_wallpapers_dir(&self) -> PathBuf {
        self.home.join(".local/share/inerium/wallpapers")
    }

    pub fn inerium_extra_wallpapers_dir(&self) -> PathBuf {
        self.home.join(".local/share/inerium/wallpapers/extra")
    }

    pub fn first_login_steps_dir(&self) -> PathBuf {
        self.config_home.join("inerium/first-login.d")
    }

    pub fn first_login_state_dir(&self) -> PathBuf {
        self.state_home.join("inerium/first-login")
    }

    pub fn setup_done_path(&self) -> PathBuf {
        self.state_home.join("inerium/setup.done")
    }

    pub fn wallpaper_dirs(&self) -> Vec<PathBuf> {
        vec![
            self.inerium_wallpapers_dir(),
            self.inerium_extra_wallpapers_dir(),
        ]
    }
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let resolved = if let Ok(canon) = path.canonicalize() {
        canon
    } else {
        let mut curr = path.to_path_buf();
        let mut depth = 0;
        while curr.is_symlink() && depth < 16 {
            depth += 1;
            match std::fs::read_link(&curr) {
                Ok(target) => {
                    curr = if target.is_relative() {
                        curr.parent().map(|p| p.join(&target)).unwrap_or(target)
                    } else {
                        target
                    };
                    if let Ok(canon) = curr.canonicalize() {
                        curr = canon;
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        curr
    };
    let path = &resolved;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating {}: {e}", parent.display()))?;
    }
    let tmp_name = format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("inerium"),
        std::process::id()
    );
    let tmp_path = path.with_file_name(tmp_name);
    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed creating {}: {e}", tmp_path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed writing {}: {e}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|e| format!("Failed syncing {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed renaming to {}: {e}", path.display()))
}

pub(crate) fn escape_lua_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

pub(crate) fn extract_lua_field(block: &str, key: &str) -> Option<String> {
    for line in block.lines() {
        let code = line.split("--").next().unwrap_or("").trim();
        let Some((lhs, rhs)) = code.split_once('=') else {
            continue;
        };
        let lhs_clean = lhs.trim().trim_start_matches("[\"").trim_end_matches("\"]");
        if lhs_clean != key {
            continue;
        }
        let val = rhs.trim().trim_end_matches(',').trim();
        if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            return Some(val[1..val.len() - 1].replace("\\\"", "\""));
        }
        return Some(val.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspaces_count_parses_and_clamps() {
        assert_eq!(parse_workspaces_count("5\n"), 5);
        assert_eq!(parse_workspaces_count(" 9 "), 9);
        assert_eq!(parse_workspaces_count("0"), 3);
        assert_eq!(parse_workspaces_count("12"), 3);
        assert_eq!(parse_workspaces_count("abc"), 3);
    }

    #[test]
    fn hypr_monitors_json_parses_and_generates_lua() {
        let json = r#"[
            {
                "name": "eDP-2",
                "description": "Laptop Panel",
                "width": 1920,
                "height": 1080,
                "refreshRate": 144.003,
                "x": 0,
                "y": 216,
                "scale": 1.25,
                "focused": true,
                "vrr": false,
                "disabled": false,
                "availableModes": ["1920x1080@144.00Hz", "1920x1080@60.02Hz"]
            },
            {
                "name": "HDMI-A-1",
                "description": "External",
                "width": 1920,
                "height": 1080,
                "refreshRate": 60.0,
                "x": 1536,
                "y": 0,
                "scale": 1.0,
                "focused": false,
                "vrr": true,
                "disabled": false,
                "availableModes": ["1920x1080@60.00Hz"]
            }
        ]"#;

        let live = parse_hypr_monitors_json(json).expect("valid monitor json");
        assert_eq!(live.len(), 2);

        let mut merged = merge_monitors(&live, &[]);
        assert_eq!(merged[0].output, "eDP-2");
        assert_eq!(merged[0].mode, "1920x1080@144.00Hz");
        assert_eq!(merged[0].position, "0x216");
        assert_eq!(merged[1].output, "HDMI-A-1");
        assert!(merged[1].vrr);

        apply_layout_preset(&mut merged, &live, LayoutPreset::HorizontalBottom);
        assert_eq!(merged[0].position, "0x216");
        assert_eq!(merged[1].position, "1536x0");

        let lua = generate_monitors_lua(&merged);
        assert!(lua.contains("output   = \"eDP-2\""));
        assert!(lua.contains("scale    = 1.25"));
        assert!(lua.contains("vrr      = 1"));
        assert!(lua.contains("output   = \"\""));

        let reparsed = parse_monitors_lua(&lua);
        assert_eq!(reparsed, merged);
    }

    #[test]
    fn inputs_and_hypridle_roundtrip() {
        let inputs = InputOverrides {
            kb_layout: "us,ru,ua".to_string(),
            kb_options: "grp:alt_shift_toggle".to_string(),
            natural_scroll: true,
            tap_to_click: false,
            disable_while_typing: true,
            sensitivity: 0.25,
            accel_profile: "flat".to_string(),
            scroll_factor: 1.2,
            launcher_bind: Some("SUPER_L".to_string()),
        };
        let lua = generate_inputs_lua(&inputs);
        let parsed = parse_inputs_lua(&lua);
        assert_eq!(parsed.kb_layout, inputs.kb_layout);
        assert_eq!(parsed.kb_options, inputs.kb_options);
        assert_eq!(parsed.natural_scroll, inputs.natural_scroll);
        assert_eq!(parsed.tap_to_click, inputs.tap_to_click);
        assert_eq!(parsed.disable_while_typing, inputs.disable_while_typing);
        assert!((parsed.sensitivity - inputs.sensitivity).abs() < 0.01);
        assert_eq!(parsed.accel_profile, inputs.accel_profile);
        assert!((parsed.scroll_factor - inputs.scroll_factor).abs() < 0.01);

        let fallback_base = InputOverrides {
            kb_layout: "de".to_string(),
            kb_options: "grp:caps_toggle".to_string(),
            sensitivity: -0.5,
            ..InputOverrides::default()
        };
        // Partial Lua without sensitivity or kb_options
        let partial_lua = "hl.config({\n    input = {\n        kb_layout = \"fr\",\n    },\n})\n";
        let fallback_parsed = parse_inputs_lua_with_fallback(partial_lua, fallback_base);
        assert_eq!(fallback_parsed.kb_layout, "fr");
        assert_eq!(fallback_parsed.kb_options, "grp:caps_toggle");
        assert_eq!(fallback_parsed.sensitivity, -0.5);

        let test_keybinds = "hl.bind(mainMod .. \" + Space\", hl.dsp.exec_cmd(launchPrefix .. \"rofi -show drun\"))\n";
        assert_eq!(parse_launcher_bind(test_keybinds), Some("SUPER + Space".to_string()));
        let replaced = replace_launcher_bind(test_keybinds, "ALT + Space");
        assert!(replaced.contains("hl.bind(\"ALT + Space\", hl.dsp.exec_cmd(launchPrefix .. \"rofi -show drun\"))"));
        assert_eq!(parse_launcher_bind(&replaced), Some("ALT + Space".to_string()));
        let restored = replace_launcher_bind(&replaced, "SUPER + Space");
        assert_eq!(parse_launcher_bind(&restored), Some("SUPER + Space".to_string()));

        let idle = IdleConfig {
            enabled: true,
            dim_secs: 180,
            lock_secs: 300,
            dpms_secs: 480,
        };
        let conf = generate_hypridle_conf(&idle);
        assert!(conf.contains("timeout = 180"));
        assert!(conf.contains("timeout = 300"));
        assert!(conf.contains("timeout = 480"));
        assert_eq!(parse_hypridle_conf(&conf), idle);

        let disabled_idle = IdleConfig {
            enabled: false,
            ..idle
        };
        let disabled_conf = generate_hypridle_conf(&disabled_idle);
        assert!(!disabled_conf.contains("listener {"));
        assert_eq!(parse_hypridle_conf(&disabled_conf), disabled_idle);

        let raw_laptop = r#"
# Auto-generated by inerium-desktop (laptop chassis, gitignored)
listener {
    timeout = 150
    on-timeout = brightnessctl -s set 20%
}
listener {
    timeout = 300
    on-timeout = loginctl lock-session
}
listener {
    timeout = 330
    on-timeout = hyprctl dispatch dpms off
}
"#;
        assert_eq!(
            parse_hypridle_conf(raw_laptop),
            IdleConfig {
                enabled: true,
                dim_secs: 150,
                lock_secs: 300,
                dpms_secs: 330,
            }
        );

        let raw_desktop = r#"
# Auto-generated by inerium-desktop (desktop chassis, gitignored)
listener {
    timeout = 600
    on-timeout = loginctl lock-session
}
listener {
    timeout = 900
    on-timeout = hyprctl dispatch dpms off
}
"#;
        assert_eq!(
            parse_hypridle_conf(raw_desktop),
            IdleConfig {
                enabled: true,
                dim_secs: 0,
                lock_secs: 600,
                dpms_secs: 900,
            }
        );
    }

    #[test]
    fn wallpaper_parsing_and_step_metadata() {
        let conf = r#"
wallpaper {
    monitor = eDP-2
    path = /home/user/Pictures/Wallpapers/fuji.jpg
    fit_mode = cover
}
wallpaper {
    monitor = 
    path = /home/user/Pictures/Wallpapers/default.jpg
    fit_mode = cover
}
"#;
        assert_eq!(
            parse_active_wallpaper(conf),
            Some(PathBuf::from("/home/user/Pictures/Wallpapers/fuji.jpg"))
        );

        let script = "#!/usr/bin/env bash\n# Title: GitHub — auth\n# Title[ru]: GitHub — вход\n# Mode: gui\necho ok\n";
        let (title, title_ru, mode) = parse_step_metadata(Path::new("10-github"), script);
        assert_eq!(title, "GitHub — auth");
        assert_eq!(title_ru, Some("GitHub — вход".to_string()));
        assert_eq!(mode, StepMode::Gui);

        let test_paths = SetupPaths {
            home: PathBuf::from("/home/testuser"),
            config_home: PathBuf::from("/home/testuser/.config"),
            state_home: PathBuf::from("/home/testuser/.local/state"),
        };
        assert_eq!(
            test_paths.wallpaper_dirs(),
            vec![
                PathBuf::from("/home/testuser/.local/share/inerium/wallpapers"),
                PathBuf::from("/home/testuser/.local/share/inerium/wallpapers/extra"),
            ]
        );
    }

    #[test]
    fn markers_and_atomic_write_in_tempdir() {
        let base = std::env::temp_dir().join(format!("inerium_setup_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let paths = SetupPaths {
            home: base.clone(),
            config_home: base.join(".config"),
            state_home: base.join(".local/state"),
        };

        write_workspaces_count(&paths.workspaces_count_path(), 6).unwrap();
        assert_eq!(read_workspaces_count(&paths.workspaces_count_path()), 6);

        mark_setup_done(&paths).unwrap();
        mark_step_done(&paths.first_login_state_dir(), "10-github").unwrap();
        assert!(paths.setup_done_path().exists());
        assert!(step_done_marker(&paths.first_login_state_dir(), "10-github").exists());

        reset_all_markers(&paths).unwrap();
        assert!(!paths.setup_done_path().exists());
        assert!(!paths.first_login_state_dir().exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_write_preserves_symlinks() {
        let base = std::env::temp_dir().join(format!("inerium_symlink_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let target_file = base.join("real_target.txt");
        fs::write(&target_file, "original").unwrap();

        let symlink_file = base.join("symlink_target.txt");
        std::os::unix::fs::symlink(&target_file, &symlink_file).unwrap();

        atomic_write(&symlink_file, "updated_content").unwrap();

        assert!(symlink_file.is_symlink(), "symlink must still be a symlink");
        assert_eq!(fs::read_to_string(&symlink_file).unwrap(), "updated_content");
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "updated_content");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn generate_inputs_lua_preserves_standard_gestures() {
        let inputs = InputOverrides::default();
        let lua = generate_inputs_lua(&inputs);
        assert!(lua.contains("hl.gesture({\n    fingers   = 4,\n    direction = \"horizontal\",\n    action    = \"workspace\",\n})"));
        assert!(lua.contains("hl.gesture({\n    fingers   = 3,\n    direction = \"down\",\n    action    = \"close\",\n})"));
        assert!(lua.contains("hl.gesture({\n    fingers   = 3,\n    direction = \"up\",\n    action    = \"fullscreen\",\n})"));
        assert!(lua.contains("hl.gesture({\n    fingers   = 3,\n    direction = \"left\",\n    action    = \"float\",\n})"));
    }

    #[test]
    fn extract_and_preserve_custom_gestures() {
        let sample_lua = r#"-- Custom input
hl.config({
    input = {
        kb_layout = "us",
    },
})

hl.gesture({
    fingers   = 3,
    direction = "down",
    action    = "special:custom",
})
"#;
        let extracted = extract_gestures_lua(sample_lua);
        assert!(extracted.is_some());
        let gestures = extracted.unwrap();
        assert!(gestures.contains("action    = \"special:custom\""));

        let inputs = InputOverrides::default();
        let generated = generate_inputs_lua_with_gestures(&inputs, &gestures);
        assert!(generated.contains("action    = \"special:custom\""));
        assert!(!generated.contains("action    = \"workspace\""));
    }
}
