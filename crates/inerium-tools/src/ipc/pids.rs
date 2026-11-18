//! Universal PID registry and IPC notification for Waybar status modules.

use std::fs;
use std::path::PathBuf;
use crate::sys::posix::*;

pub struct PidRegistry {
    name: String,
    base_dir: PathBuf,
}

impl PidRegistry {
    pub fn new(name: &str) -> Self {
        Self::with_base_dir(name, Self::runtime_base_dir())
    }

    pub fn with_base_dir(name: &str, base_dir: PathBuf) -> Self {
        Self {
            name: name.to_string(),
            base_dir,
        }
    }

    fn runtime_base_dir() -> PathBuf {
        if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(rt);
            if p.exists() {
                return p;
            }
        }
        let uid = posix_getuid();
        let run_user = PathBuf::from(format!("/run/user/{}", uid));
        if run_user.exists() {
            run_user
        } else {
            let fallback = std::env::temp_dir().join(format!("inerium-runtime-{}", uid));
            let _ = fs::create_dir_all(&fallback);
            fallback
        }
    }

    pub fn pids_dir(&self) -> PathBuf {
        self.base_dir.join(format!("inerium-{}-pids", self.name))
    }

    pub fn state_file(&self) -> PathBuf {
        self.base_dir.join(format!("inerium-{}", self.name))
    }

    /// Register current PID in the registry. Returns a guard that removes the file on drop.
    pub fn register(&self) -> PidRegistrationGuard {
        let dir = self.pids_dir();
        let _ = fs::create_dir_all(&dir);
        let pid = posix_getpid();
        let file_path = dir.join(pid.to_string());
        let _ = fs::write(&file_path, b"");
        PidRegistrationGuard { path: file_path }
    }

    /// Notify all registered instances with a signal (e.g., SIGUSR1).
    /// Cleans up stale PID files automatically.
    pub fn notify_all(&self, sig: i32) {
        let dir = self.pids_dir();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string()
                    && let Ok(pid) = file_name.parse::<i32>()
                        && posix_kill(pid, sig).is_err() {
                            // Process is dead, remove stale PID entry
                            let _ = fs::remove_file(entry.path());
                        }
            }
        }
    }

    /// Toggle a boolean state file (creates if absent, removes if present).
    /// Returns the new state (`true` = present, `false` = removed).
    pub fn toggle_state(&self) -> bool {
        let path = self.state_file();
        if path.exists() {
            let _ = fs::remove_file(&path);
            false
        } else {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&path, b"").is_ok()
        }
    }

    /// Check if state file exists.
    pub fn is_state_active(&self) -> bool {
        self.state_file().exists()
    }
}

pub struct PidRegistrationGuard {
    path: PathBuf,
}

impl Drop for PidRegistrationGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_registry_toggle() {
        let temp_dir = std::env::temp_dir().join(format!("inerium-test-pids-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let reg = PidRegistry::with_base_dir("test-unit-smoke", temp_dir.clone());
        let initial = reg.is_state_active();
        assert!(!initial);
        let toggled = reg.toggle_state();
        assert!(toggled);
        assert!(reg.is_state_active());
        let reverted = reg.toggle_state();
        assert!(!reverted);
        assert!(!reg.is_state_active());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
