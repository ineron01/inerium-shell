//! inerium-tools — Shared system core for Inerium Waybar providers and desktop daemons.

extern crate self as inerium_tools;

pub mod sys;
pub mod ipc;
pub mod format;
pub mod desktop;
pub mod tools;

pub use inerium_hw as hw;
pub use sys::*;
pub use ipc::*;
pub use format::*;
pub use desktop::{open_folder, wallpaper};
