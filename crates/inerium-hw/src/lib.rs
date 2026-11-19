//! inerium-hw — Unified hardware abstraction and sysfs telemetry library.
//! Portable across generic Linux platforms with dynamic vendor discovery and feature gates.

pub mod sysfs;
pub mod battery;
pub mod cooling;
pub mod power;
pub mod wallpaper;
pub mod desktop;

pub use sysfs::*;
pub use battery::{discover_power_supplies, BatteryBackend};
pub use cooling::{CoolingSubsystem, FanSensor, FanTelemetry, PlatformControl};
pub use power::PowerBackend;
pub use desktop::open_folder;
