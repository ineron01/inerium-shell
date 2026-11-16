use std::env;
use std::path::Path;
use std::process;

fn print_usage() {
    eprintln!("inerium-wallpaper v0.1.2 — Dynamic wallpaper setter for hyprpaper in Hyprland");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    inerium-wallpaper <IMAGE_PATH> [MONITOR_NAME|all] [--no-notify]");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    inerium-wallpaper ~/Pictures/Wallpapers/wall3.png");
    eprintln!("    inerium-wallpaper ~/Pictures/Wallpapers/wall4.png DP-1");
    eprintln!("    inerium-wallpaper ~/Pictures/Wallpapers/wall3.png all --no-notify");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    let image_path = Path::new(&args[1]);
    let mut target_monitor = None;
    let mut notify = true;

    for arg in &args[2..] {
        if arg == "--no-notify" {
            notify = false;
        } else if !arg.starts_with("--") && target_monitor.is_none() {
            target_monitor = Some(arg.as_str());
        }
    }

    if let Err(err) = inerium_hw::wallpaper::apply_wallpaper(image_path, target_monitor, notify) {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
