//! Universal PID registry and IPC notification for Waybar status modules.

use std::fs;
use std::path::{Path, PathBuf};
use crate::sys::posix::*;

pub fn is_inerium_comm(comm: &str) -> bool {
    comm.starts_with("inerium-") || comm == "hypr-ipc-bridge" || comm == "inerium-ctl"
}

fn is_valid_process(pid: i32) -> bool {
    let is_alive = unsafe { libc::kill(pid, 0) == 0 };
    if !is_alive {
        return false;
    }
    match fs::read_to_string(format!("/proc/{pid}/comm")) {
        Ok(comm) => is_inerium_comm(comm.trim()),
        Err(_) => false,
    }
}

pub struct PidRegistry {
    name: String,
    base_dir: PathBuf,
}

impl PidRegistry {
    pub fn new(name: &str) -> Self {
        let clean = name.strip_prefix("inerium-").unwrap_or(name);
        Self::with_base_dir(clean, Self::runtime_base_dir())
    }

    pub fn with_base_dir(name: &str, base_dir: PathBuf) -> Self {
        Self {
            name: name.to_string(),
            base_dir,
        }
    }

    fn runtime_base_dir() -> PathBuf {
        let base = if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(rt);
            if p.exists() {
                p
            } else {
                Self::fallback_base_dir()
            }
        } else {
            Self::fallback_base_dir()
        };
        base.join("inerium")
    }

    fn fallback_base_dir() -> PathBuf {
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
        self.base_dir.join(format!("{}-pids", self.name))
    }

    pub fn state_file(&self) -> PathBuf {
        self.base_dir.join(&self.name)
    }

    /// Register current PID in the registry. Returns a guard that removes the file on drop.
    pub fn register(&self) -> PidRegistrationGuard {
        let dir = self.pids_dir();
        let _ = fs::create_dir_all(&dir);
        let pid = unsafe { libc::getpid() };
        let file_path = dir.join(pid.to_string());
        let _ = fs::write(&file_path, b"");
        PidRegistrationGuard { path: file_path }
    }

    /// Clean up stale PID files whose processes are no longer alive.
    /// Returns the number of removed stale PID files.
    pub fn cleanup_stale(&self) -> usize {
        Self::cleanup_stale_dir(&self.pids_dir())
    }

    fn cleanup_stale_dir(dir: &Path) -> usize {
        if !dir.exists() {
            return 0;
        }
        let mut cleaned = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string()
                    && let Ok(pid) = file_name.parse::<i32>()
                    && !is_valid_process(pid)
                    && fs::remove_file(entry.path()).is_ok()
                {
                    cleaned += 1;
                }
            }
        }
        cleaned
    }

    /// Notify all registered instances in this registry with a signal (e.g., SIGUSR1).
    /// Cleans up stale PID files automatically.
    /// Returns the number of processes successfully notified.
    pub fn notify(&self, sig: i32) -> Result<usize, std::io::Error> {
        let mut dirs = vec![self.pids_dir()];
        // Check legacy flat runtime directory (/run/user/{uid}/inerium-{name}-pids)
        if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
            let legacy = PathBuf::from(rt).join(format!("inerium-{}-pids", self.name));
            if legacy != self.pids_dir() && legacy.exists() {
                dirs.push(legacy);
            }
        }

        let mut notified = 0;
        for dir in dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string()
                        && let Ok(pid) = file_name.parse::<i32>()
                    {
                        if !is_valid_process(pid) {
                            let _ = fs::remove_file(entry.path());
                            continue;
                        }
                        let ret = unsafe { libc::kill(pid, sig) };
                        if ret == 0 {
                            notified += 1;
                        } else {
                            let err = std::io::Error::last_os_error();
                            if err.raw_os_error() == Some(libc::ESRCH) {
                                // Process is dead, remove stale PID entry
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
        Ok(notified)
    }

    /// Notify all registered instances for a given application with a signal (e.g., SIGUSR1).
    /// Cleans up stale PID files automatically.
    /// Returns the number of processes successfully notified.
    pub fn notify_all(app: &str, sig: i32) -> Result<usize, std::io::Error> {
        let reg = Self::new(app);
        reg.notify(sig)
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

    #[test]
    fn test_pid_registry_register_and_drop() {
        let temp_dir = std::env::temp_dir().join(format!("inerium-test-reg-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let reg = PidRegistry::with_base_dir("test-reg", temp_dir.clone());
        let pid = unsafe { libc::getpid() };
        let pid_file = reg.pids_dir().join(pid.to_string());

        {
            let _guard = reg.register();
            assert!(pid_file.exists());
        }
        assert!(!pid_file.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_inerium_comm() {
        assert!(is_inerium_comm("inerium-battery"));
        assert!(is_inerium_comm("inerium-wallpaper"));
        assert!(is_inerium_comm("inerium-ctl"));
        assert!(is_inerium_comm("hypr-ipc-bridge"));
        assert!(!is_inerium_comm("systemd"));
        assert!(!is_inerium_comm("bash"));
        assert!(!is_inerium_comm("cat"));
        assert!(!is_inerium_comm(""));
    }

    #[test]
    fn test_pid_reuse_cleanup() {
        let temp_dir = std::env::temp_dir().join(format!("inerium-test-reuse-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let reg = PidRegistry::with_base_dir("test-reuse", temp_dir.clone());
        let pids_dir = reg.pids_dir();
        let _ = fs::create_dir_all(&pids_dir);

        // PID 1 is systemd (alive, but comm != inerium-*)
        let pid1_file = pids_dir.join("1");
        let _ = fs::write(&pid1_file, b"");

        // Stale non-existent PID
        let dead_file = pids_dir.join("99999999");
        let _ = fs::write(&dead_file, b"");

        assert!(pid1_file.exists());
        assert!(dead_file.exists());

        // Notify should not signal PID 1 and should remove both stale/reused PID files
        let notified = reg.notify(libc::SIGUSR1).unwrap();
        assert_eq!(notified, 0);
        assert!(!pid1_file.exists());
        assert!(!dead_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
