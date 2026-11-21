//! Notification sound playback tool with cooldown debounce

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

const DEFAULT_SOUND: &str = "/usr/share/sounds/freedesktop/stereo/message.oga";
const DEFAULT_COOLDOWN_MS: u64 = 1200;
const STAMP_FILENAME: &str = "inerium-notify-sound.stamp";

/// Play notification sound with debounce protection
pub fn run(args: &[String]) {
    let mut sound_path = String::from(DEFAULT_SOUND);
    let mut cooldown_ms = DEFAULT_COOLDOWN_MS;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--sound" {
            if let Some(val) = iter.next() {
                sound_path = val.clone();
            }
        } else if let Some(val) = arg.strip_prefix("--sound=") {
            sound_path = val.to_string();
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
        }
    }

    let stamp_path = match env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join(STAMP_FILENAME),
        _ => PathBuf::from("/tmp").join(STAMP_FILENAME),
    };

    let now = SystemTime::now();

    // Debounce check based on stamp mtime
    if let Ok(metadata) = fs::metadata(&stamp_path)
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = now.duration_since(modified)
        && elapsed < Duration::from_millis(cooldown_ms)
    {
        // Still in cooldown period, exit quietly
        return;
    }

    // Update stamp mtime
    let _ = fs::write(&stamp_path, b"");

    let sound = resolve_sound_path(&sound_path);
    if !sound.exists() {
        return;
    }

    // Spawn pw-play asynchronously without blocking the caller
    let _ = Command::new("pw-play")
        .arg("--media-role=Notification")
        .arg(&sound)
        .spawn();
}

fn resolve_sound_path(path: &str) -> PathBuf {
    if let Some(sub) = path.strip_prefix("~/")
        && let Ok(home) = env::var("HOME")
    {
        return PathBuf::from(home).join(sub);
    } else if let Some(sub) = path.strip_prefix("$HOME/")
        && let Ok(home) = env::var("HOME")
    {
        return PathBuf::from(home).join(sub);
    }

    let p = PathBuf::from(path);
    if p.exists() {
        return p;
    }

    // Fallbacks if only filename was specified
    if let Ok(home) = env::var("HOME") {
        let repo_asset = PathBuf::from(&home)
            .join("Repos/inerium-shell/assets/sounds")
            .join(path);
        if repo_asset.exists() {
            return repo_asset;
        }

        let local_asset = PathBuf::from(&home)
            .join(".local/share/inerium/sounds")
            .join(path);
        if local_asset.exists() {
            return local_asset;
        }
    }

    p
}
