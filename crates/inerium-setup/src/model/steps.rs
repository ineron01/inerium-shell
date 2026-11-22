use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use inerium_i18n::Lang;

use super::{SetupPaths, atomic_write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    Terminal,
    Gui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Configured,
    Deferred,
    Pending,
}

#[derive(Debug, Clone)]
pub struct FirstLoginStep {
    pub id: String,
    pub title: String,
    pub title_ru: Option<String>,
    pub path: PathBuf,
    pub mode: StepMode,
    pub status: StepStatus,
}

impl FirstLoginStep {
    pub fn title(&self, lang: Lang) -> &str {
        if lang == Lang::Ru && let Some(ref ru) = self.title_ru {
            ru
        } else {
            &self.title
        }
    }
}

pub fn parse_step_metadata(path: &Path, content: &str) -> (String, Option<String>, StepMode) {
    let fallback_title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("step")
        .to_string();
    let mut title = None;
    let mut title_ru = None;
    let mut mode = StepMode::Terminal;

    for line in content.lines().take(20) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let rest = rest.trim();
            if let Some(val) = rest.strip_prefix("Title[ru]:") {
                let t = val.trim();
                if !t.is_empty() {
                    title_ru = Some(t.to_string());
                }
            } else if let Some(val) = rest.strip_prefix("Title:") {
                let t = val.trim();
                if !t.is_empty() {
                    title = Some(t.to_string());
                }
            } else if let Some(val) = rest.strip_prefix("Mode:")
                && val.trim().eq_ignore_ascii_case("gui")
            {
                mode = StepMode::Gui;
            }
        }
    }

    (title.unwrap_or(fallback_title), title_ru, mode)
}

pub fn list_executable_steps(steps_dir: &Path) -> Vec<FirstLoginStep> {
    let Ok(entries) = fs::read_dir(steps_dir) else {
        return Vec::new();
    };
    let mut map = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || (meta.permissions().mode() & 0o111) == 0 {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let content = fs::read_to_string(&path).unwrap_or_default();
        let (title, title_ru, mode) = parse_step_metadata(&path, &content);
        map.insert(
            name.to_string(),
            FirstLoginStep {
                id: name.to_string(),
                title,
                title_ru,
                path,
                mode,
                status: StepStatus::Pending,
            },
        );
    }
    map.into_values().collect()
}

pub fn step_done_marker(state_dir: &Path, step_id: &str) -> PathBuf {
    state_dir.join(format!("{step_id}.done"))
}

pub fn mark_step_done(state_dir: &Path, step_id: &str) -> Result<(), String> {
    fs::create_dir_all(state_dir)
        .map_err(|e| format!("Failed creating {}: {e}", state_dir.display()))?;
    let marker = step_done_marker(state_dir, step_id);
    fs::write(&marker, b"")
        .map_err(|e| format!("Failed writing {}: {e}", marker.display()))
}

pub fn mark_setup_done(paths: &SetupPaths) -> Result<(), String> {
    atomic_write(&paths.setup_done_path(), "done\n")
}

pub fn reset_all_markers(paths: &SetupPaths) -> Result<(), String> {
    let setup_done = paths.setup_done_path();
    if setup_done.exists() {
        fs::remove_file(&setup_done)
            .map_err(|e| format!("Failed removing {}: {e}", setup_done.display()))?;
    }
    let fl_dir = paths.first_login_state_dir();
    if fl_dir.exists() {
        fs::remove_dir_all(&fl_dir)
            .map_err(|e| format!("Failed removing {}: {e}", fl_dir.display()))?;
    }
    Ok(())
}
