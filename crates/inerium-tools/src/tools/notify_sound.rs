//! Notification sound playback tool with cooldown debounce

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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

    let sound = Path::new(&sound_path);
    if !sound.exists() {
        return;
    }

    // Spawn pw-play asynchronously without blocking the caller
    let _ = Command::new("pw-play")
        .arg("--media-role=Notification")
        .arg(&sound_path)
        .spawn();
}
