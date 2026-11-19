use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk4::prelude::*;
use gtk4::{Button, glib};
use serde::{Deserialize, Serialize};

use crate::i18n::{Lang, tr};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://github.com/ineron01/inerium-shell/releases/latest";

thread_local! {
    static ACTIVE_UPDATE_BTN: RefCell<Option<Button>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub is_newer: bool,
    pub checked_at: u64,
}

pub fn parse_semver(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v.trim().trim_start_matches(['v', 'V']);
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch_str = parts.next().unwrap_or("0").split('-').next()?;
    let patch = patch_str.parse().ok()?;
    Some((major, minor, patch))
}

pub fn is_candidate_newer(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

pub fn get_cache_path() -> PathBuf {
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR")
        && !rt.is_empty()
    {
        return PathBuf::from(rt).join("inerium-setup-update.json");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/state/inerium/setup-update.json")
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn read_cached_update(max_age_secs: u64) -> Option<UpdateInfo> {
    let path = get_cache_path();
    let data = fs::read_to_string(path).ok()?;
    let info: UpdateInfo = serde_json::from_str(&data).ok()?;
    let now = current_timestamp();
    if now >= info.checked_at && (now - info.checked_at) <= max_age_secs {
        Some(info)
    } else {
        None
    }
}

pub fn save_cached_update(info: &UpdateInfo) {
    let path = get_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(info) {
        let _ = fs::write(path, json);
    }
}

pub fn check_remote_update() -> Option<UpdateInfo> {
    // 1. Simulation override for manual & automated testing
    if let Ok(sim_tag) = std::env::var("INERIUM_SIMULATE_UPDATE") {
        let clean = sim_tag.trim().to_string();
        if !clean.is_empty() {
            let is_newer = is_candidate_newer(CURRENT_VERSION, &clean);
            return Some(UpdateInfo {
                latest_version: clean,
                is_newer,
                checked_at: current_timestamp(),
            });
        }
    }

    // 2. Query GitHub Releases /latest redirect via curl HEAD request
    let output = Command::new("curl")
        .args([
            "-sI",
            "-A",
            "Inerium-Setup",
            "--max-time",
            "3",
            RELEASES_URL,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let headers = String::from_utf8_lossy(&output.stdout);
    for line in headers.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.to_lowercase().find("location:") {
            let loc = trimmed[idx + 9..].trim();
            if let Some(tag_idx) = loc.find("/releases/tag/") {
                let tag = loc[tag_idx + "/releases/tag/".len()..]
                    .trim_matches(['/', ' ', '\r', '\n'])
                    .to_string();
                if !tag.is_empty() {
                    let is_newer = is_candidate_newer(CURRENT_VERSION, &tag);
                    return Some(UpdateInfo {
                        latest_version: tag,
                        is_newer,
                        checked_at: current_timestamp(),
                    });
                }
            }
        }
    }

    None
}

pub fn attach_update_checker(btn: &Button, lang: Lang) {
    // Register active button on main thread TLS
    ACTIVE_UPDATE_BTN.with(|c| {
        *c.borrow_mut() = Some(btn.clone());
    });

    // 1. Check if we already have a fresh result in cache (< 1 hour)
    if let Some(cached) = read_cached_update(3600) {
        if cached.is_newer {
            let tag = &cached.latest_version;
            btn.set_label(&format!("󰚰 {} ({tag})", tr(lang, "Update", "Обновить")));
            btn.set_tooltip_text(Some(&format!(
                "{}: {tag}",
                tr(lang, "Update available", "Доступно обновление")
            )));
            btn.set_visible(true);
        } else {
            btn.set_visible(false);
        }
        return;
    }

    // 2. Otherwise spawn a background thread to check without blocking UI
    std::thread::spawn(move || {
        if let Some(info) = check_remote_update() {
            save_cached_update(&info);
            if info.is_newer {
                glib::idle_add_once(move || {
                    ACTIVE_UPDATE_BTN.with(|c| {
                        if let Some(b) = c.borrow().as_ref() {
                            let tag = &info.latest_version;
                            b.set_label(&format!("󰚰 {} ({tag})", tr(lang, "Update", "Обновить")));
                            b.set_tooltip_text(Some(&format!(
                                "{}: {tag}",
                                tr(lang, "Update available", "Доступно обновление")
                            )));
                            b.set_visible(true);
                        }
                    });
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parsing() {
        assert_eq!(parse_semver("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_semver("0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_semver("v1.4.12-beta1"), Some((1, 4, 12)));
        assert_eq!(parse_semver("invalid"), None);
    }

    #[test]
    fn test_is_candidate_newer() {
        assert!(!is_candidate_newer("0.2.0", "v0.2.0"));
        assert!(!is_candidate_newer("0.2.0", "0.1.9"));
        assert!(is_candidate_newer("0.2.0", "v0.2.1"));
        assert!(is_candidate_newer("0.2.0", "v0.3.0"));
        assert!(is_candidate_newer("0.2.0", "v1.0.0"));
        assert!(!is_candidate_newer("1.0.0", "v0.9.9"));
    }

    #[test]
    fn test_simulate_update_and_cache() {
        let temp_dir = std::env::temp_dir().join(format!("inerium-test-{}", current_timestamp()));
        let _ = fs::create_dir_all(&temp_dir);

        let info = UpdateInfo {
            latest_version: "v0.3.0".to_string(),
            is_newer: true,
            checked_at: current_timestamp(),
        };

        let cache_file = temp_dir.join("test-cache.json");
        let json = serde_json::to_string(&info).unwrap();
        fs::write(&cache_file, json).unwrap();

        let read_info: UpdateInfo = serde_json::from_str(&fs::read_to_string(&cache_file).unwrap()).unwrap();
        assert_eq!(read_info.latest_version, "v0.3.0");
        assert!(read_info.is_newer);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
