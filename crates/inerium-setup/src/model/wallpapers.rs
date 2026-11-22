use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::SetupPaths;

pub fn parse_active_wallpaper(hyprpaper_conf: &str) -> Option<PathBuf> {
    let mut current_mon: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut fallback: Option<PathBuf> = None;

    for raw_line in hyprpaper_conf.lines() {
        let line = raw_line.trim();
        if line.starts_with("wallpaper") && line.contains('{') {
            current_mon = None;
            current_path = None;
            continue;
        }
        if line == "}" {
            if let Some(p) = current_path.take()
                && !p.is_empty()
            {
                if current_mon.as_deref().unwrap_or("").is_empty() {
                    fallback = Some(PathBuf::from(p));
                } else {
                    return Some(PathBuf::from(p));
                }
            }
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            if key == "monitor" {
                current_mon = Some(val.to_string());
            } else if key == "path" {
                current_path = Some(val.to_string());
            }
        }
    }
    fallback
}

pub fn scan_wallpapers(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                continue;
            }
            let canon = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.insert(canon) {
                items.push(path);
            }
        }
    }
    items.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    items
}

pub fn ensure_default_wallpapers(paths: &SetupPaths) {
    let dest_dir = paths.inerium_wallpapers_dir();
    let marker_file = dest_dir.join(".wallpapers_initialized");
    if marker_file.exists() {
        return;
    }

    let _ = fs::create_dir_all(&dest_dir);

    // If destination already has images, nothing to do, just touch marker
    let has_any = fs::read_dir(&dest_dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let p = e.path();
                p.is_file()
                    && matches!(
                        p.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp"
                    )
            })
        })
        .unwrap_or(false);

    if !has_any {
        let sources = [
            paths.home.join(".local/share/inerium/shell/assets/wallpapers"),
            paths.home.join(".local/share/inerium-shell/assets/wallpapers"),
            paths.home.join("inerium-shell/assets/wallpapers"),
            paths.home.join("Repos/inerium-shell/assets/wallpapers"),
            paths.home.join("Repos/inerium-desktop/assets/wallpapers"),
            PathBuf::from("/usr/share/inerium/wallpapers"),
        ];
        for src_dir in &sources {
            if let Ok(entries) = fs::read_dir(src_dir) {
                let mut copied = false;
                for entry in entries.flatten() {
                    let sp = entry.path();
                    let ext = sp
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if sp.is_file()
                        && matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp")
                        && let Some(file_name) = sp.file_name()
                    {
                        let target = dest_dir.join(file_name);
                        if !target.exists() && fs::copy(&sp, &target).is_ok() {
                            copied = true;
                        }
                    }
                }
                if copied {
                    break;
                }
            }
        }
    }

    let _ = fs::File::create(&marker_file);
}
