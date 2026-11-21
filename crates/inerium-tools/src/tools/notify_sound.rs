//! Notification sound playback tool with per-sound cooldown debounce and XDG asset resolution.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const DEFAULT_SOUND_ALIAS: &str = "message";
const DEFAULT_COOLDOWN_MS: u64 = 1200;

/// Play notification sound with debounce protection and XDG audio resolution
pub fn run(args: &[String]) {
    let mut sound_target = String::from(DEFAULT_SOUND_ALIAS);
    let mut cooldown_ms = DEFAULT_COOLDOWN_MS;
    let mut volume: Option<f32> = None;

    let is_bin_name = |s: &str| {
        s == "inerium-notify-sound"
            || s == "notify-sound"
            || s == "inerium-ctl"
            || s.ends_with("/inerium-notify-sound")
            || s.ends_with("/inerium-ctl")
    };

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--sound" {
            if let Some(val) = iter.next() {
                sound_target = val.clone();
            }
        } else if let Some(val) = arg.strip_prefix("--sound=") {
            sound_target = val.to_string();
        } else if arg == "--cooldown-ms" {
            if let Some(val) = iter.next()
                && let Ok(ms) = val.parse::<u64>()
            {
                cooldown_ms = ms;
            }
        } else if let Some(val) = arg.strip_prefix("--cooldown-ms=")
            && let Ok(ms) = val.parse::<u64>()
        {
            cooldown_ms = ms;
        } else if arg == "--volume" {
            if let Some(val) = iter.next()
                && let Ok(v) = val.parse::<f32>()
            {
                volume = Some(v.clamp(0.0, 1.5));
            }
        } else if let Some(val) = arg.strip_prefix("--volume=")
            && let Ok(v) = val.parse::<f32>()
        {
            volume = Some(v.clamp(0.0, 1.5));
        } else if !arg.starts_with('-') && !is_bin_name(arg) && sound_target == DEFAULT_SOUND_ALIAS {
            sound_target = arg.clone();
        }
    }

    // Resolve audio file path
    let sound_file = match resolve_sound_path(&sound_target) {
        Some(p) => p,
        None => return,
    };

    // Sanitize alias/filename for isolated per-sound stamp debounce
    let sanitized_name = Path::new(&sound_target)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            s.chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                .collect::<String>()
        })
        .unwrap_or_else(|| "default".to_string());

    let stamp_filename = format!("inerium-notify-sound-{sanitized_name}.stamp");
    let stamp_path = match env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join(&stamp_filename),
        _ => PathBuf::from("/tmp").join(&stamp_filename),
    };

    let now = SystemTime::now();

    // Debounce check based on stamp mtime
    if let Ok(metadata) = fs::metadata(&stamp_path)
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = now.duration_since(modified)
        && elapsed < Duration::from_millis(cooldown_ms)
    {
        // Still in cooldown period for this specific sound, exit quietly
        return;
    }

    // Update stamp mtime
    let _ = fs::write(&stamp_path, b"");

    // Spawn pw-play asynchronously without blocking the caller
    let mut cmd = Command::new("pw-play");
    cmd.arg("--media-role=Notification");
    if let Some(vol) = volume {
        cmd.arg("--volume").arg(format!("{vol:.2}"));
    }
    cmd.arg(&sound_file);
    let _ = cmd.spawn();
}

fn resolve_sound_path(target: &str) -> Option<PathBuf> {
    // 1. Explicit path with tilde or $HOME
    if let Some(sub) = target.strip_prefix("~/")
        && let Ok(home) = env::var("HOME")
    {
        let p = PathBuf::from(home).join(sub);
        if p.exists() {
            return Some(p);
        }
    } else if let Some(sub) = target.strip_prefix("$HOME/")
        && let Ok(home) = env::var("HOME")
    {
        let p = PathBuf::from(home).join(sub);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Direct absolute or relative path
    let raw_path = PathBuf::from(target);
    if raw_path.is_absolute() && raw_path.exists() {
        return Some(raw_path);
    }

    // 3. Search by alias / filename across standard XDG and system sound paths
    let extensions = ["ogg", "oga", "mp3", "wav"];
    let mut search_dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        if !data_home.is_empty() {
            search_dirs.push(PathBuf::from(data_home).join("inerium/sounds"));
        }
    } else if let Ok(home) = env::var("HOME") {
        search_dirs.push(PathBuf::from(home).join(".local/share/inerium/sounds"));
    }

    search_dirs.push(PathBuf::from("/usr/share/sounds/inerium"));
    search_dirs.push(PathBuf::from("/usr/share/sounds/freedesktop/stereo"));

    for dir in &search_dirs {
        let direct = dir.join(target);
        if direct.exists() {
            return Some(direct);
        }
        for ext in &extensions {
            let candidate = dir.join(format!("{target}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 4. Fallback to standard freedesktop message sound
    let freedesktop_fallback = PathBuf::from("/usr/share/sounds/freedesktop/stereo/message.oga");
    if freedesktop_fallback.exists() {
        return Some(freedesktop_fallback);
    }

    None
}
