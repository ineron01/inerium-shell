use std::path::Path;
use std::process;

fn print_usage() {
    eprintln!(
        "inerium-wallpaper v{} — Dynamic wallpaper setter for hyprpaper in Hyprland",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    inerium-wallpaper <IMAGE_PATH> [MONITOR_NAME|all] [--no-notify]");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    inerium-wallpaper ~/.local/share/inerium/wallpapers/01_nordic_mountains.webp");
    eprintln!("    inerium-wallpaper ~/.local/share/inerium/wallpapers/04_abstract_silk_waves.webp DP-1");
    eprintln!("    inerium-wallpaper ~/.local/share/inerium/wallpapers/01_nordic_mountains.webp all --no-notify");
}

pub fn run(args: &[String]) {
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

    if let Err(err) = inerium_tools::desktop::wallpaper::apply_wallpaper(image_path, target_monitor, notify) {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
