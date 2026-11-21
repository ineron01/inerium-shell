use std::path::Path;

use eyre::Result;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    helper_path: String,
    socket_path: String,
}

impl SystemConfig {
    pub fn from_file() -> Result<Self> {
        let mut fig = Figment::new();
        // Prioritize XDG_CONFIG_HOME (user config), defaulting to $HOME/.config
        let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME")
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config").into()));
        let xdg_path = xdg_config_home.as_ref().map(|c| Path::new(&c).join("inerium-polkit/config.toml"));
        let xdg_fallback = xdg_config_home.as_ref().map(|c| Path::new(&c).join("inerium/polkit.toml"));

        if let Some(path) = xdg_path.filter(|p| p.exists()) {
            fig = fig.merge(Toml::file_exact(path.clone()));
            tracing::info!("using configuration file found at {}", path.display());
        } else if let Some(path) = xdg_fallback.filter(|p| p.exists()) {
            fig = fig.merge(Toml::file_exact(path.clone()));
            tracing::info!("using configuration file found at {}", path.display());
        } else if Path::new("/usr/local/etc/inerium-polkit/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/usr/local/etc/inerium-polkit/config.toml"));
            tracing::info!("using configuration file found at /usr/local/etc/inerium-polkit/config.toml");
        } else if Path::new("/etc/inerium-polkit/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/etc/inerium-polkit/config.toml"));
            tracing::info!("using configuration file found at /etc/inerium-polkit/config.toml");
        // Fall back to default
        } else {
            fig = fig.merge(Serialized::defaults(Self::default()));
            tracing::info!("no configuration file found, using default configuration instead");
        }
        Ok(fig.extract()?)
    }

    pub fn get_helper_path(&self) -> &str {
        &self.helper_path
    }

    pub fn get_socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            helper_path: env!("POLKIT_AGENT_HELPER_PATH").into(),
            socket_path: env!("POLKIT_AGENT_SOCKET_PATH").into(),
        }
    }
}
