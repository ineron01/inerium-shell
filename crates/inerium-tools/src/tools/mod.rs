//! Inerium Tools — Individual tool implementations unified under inerium-ctl

pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod clock;
pub mod dnd;
pub mod hotspot;
pub mod hwmon;
pub mod hypr_ipc_bridge;
pub mod layout;
pub mod network;
pub mod osd_watcher;
pub mod popup;
pub mod theme;
pub mod updates;
pub mod wallpaper;

/// Run a tool by its name or alias.
pub fn run_tool(name: &str, args: &[String]) -> Result<(), String> {
    match name {
        "inerium-battery" | "battery" => battery::run(args),
        "inerium-bluetooth" | "bluetooth" => bluetooth::run(args),
        "inerium-clock" | "clock" => clock::run(args),
        "inerium-dnd" | "dnd" => dnd::run(args),
        "inerium-hotspot" | "hotspot" => hotspot::run(args),
        "inerium-hwmon" | "hwmon" => hwmon::run(args),
        "inerium-layout" | "layout" => layout::run(args),
        "inerium-network" | "network" => network::run(args),
        "inerium-updates" | "updates" => updates::run(args),
        "inerium-clipboard" | "clipboard" => clipboard::run(args),
        "inerium-popup" | "popup" => popup::run(args),
        "hypr-ipc-bridge" | "ipc-bridge" => hypr_ipc_bridge::run(args),
        "inerium-osd-watcher" | "osd-watcher" => osd_watcher::run(args),
        "inerium-theme" | "theme" => theme::run(args),
        "inerium-wallpaper" | "wallpaper" => wallpaper::run(args),
        _ => return Err(format!("Unknown tool: {}", name)),
    }
    Ok(())
}
