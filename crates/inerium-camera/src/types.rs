use serde::{Deserialize, Serialize};
use std::fmt;
use inerium_i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraSource {
    Builtin,
    ExternalBack,
    ExternalFront,
}

impl CameraSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Builtin => tr("Built-in (laptop)", "Встроенная (ноутбук)"),
            Self::ExternalBack => tr("External (Main)", "Внешняя (Основная)"),
            Self::ExternalFront => tr("External (Front)", "Внешняя (Фронтальная)"),
        }
    }
}

impl fmt::Display for CameraSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Landscape,
    Portrait,
}

impl Orientation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Landscape => tr("16:9 (Landscape)", "16:9 (Альбомная)"),
            Self::Portrait => tr("9:16 (Portrait)", "9:16 (Портретная)"),
        }
    }

    /// Returns scrcpy capture orientation angle (0, 90, 180, 270)
    pub fn scrcpy_angle(&self, source: CameraSource) -> &'static str {
        match (self, source) {
            // Front camera sensor is mounted at 270 deg relative to back camera on typical Android devices
            (Self::Landscape, CameraSource::ExternalFront) => "0",
            (Self::Portrait, CameraSource::ExternalFront) => "270",
            (Self::Landscape, _) => "0",
            (Self::Portrait, _) => "90",
        }
    }

    pub fn resolution(&self) -> &'static str {
        // Android camera sensor capture resolution is landscape (1920x1080).
        // Scrcpy rotates the stream to 1080x1920 via --capture-orientation=90.
        "1920x1080"
    }
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSessionInfo {
    pub is_streaming: bool,
    pub source: CameraSource,
    pub orientation: Orientation,
    pub torch: bool,
    pub audio: bool,
    pub conn_type: Option<String>,
    pub v4l2_device: Option<String>,
    #[serde(default)]
    pub bridge_pid: Option<u32>,
    pub pid: Option<u32>,
    #[serde(default)]
    pub aux_pids: Vec<u32>,
}

impl Default for CameraSessionInfo {
    fn default() -> Self {
        Self {
            is_streaming: false,
            source: CameraSource::ExternalBack,
            orientation: Orientation::Landscape,
            torch: false,
            audio: false,
            conn_type: None,
            v4l2_device: None,
            bridge_pid: None,
            pid: None,
            aux_pids: Vec::new(),
        }
    }
}
