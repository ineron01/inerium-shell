//! inerium-ctl — Multicall binary and IPC controller for Inerium desktop tools.

use std::env;
use std::path::Path;
use std::process;

use inerium_tools::ipc::PidRegistry;
use inerium_tools::tools::run_tool;

const TOOLS: &[&str] = &[
    "inerium-battery",
    "inerium-bluetooth",
    "inerium-clock",
    "inerium-dnd",
    "inerium-hotspot",
    "inerium-hwmon",
    "inerium-layout",
    "inerium-network",
    "inerium-notify-sound",
    "inerium-updates",
    "inerium-clipboard",
    "inerium-popup",
    "hypr-ipc-bridge",
    "inerium-osd-watcher",
    "inerium-theme",
    "inerium-wallpaper",
];

fn canonical_tool_name(cmd: &str) -> Option<&'static str> {
    if cmd == "set-wallpaper" {
        return Some("inerium-wallpaper");
    }
    for &tool in TOOLS {
        if cmd == tool {
            return Some(tool);
        }
        if let Some(short) = tool.strip_prefix("inerium-")
            && cmd == short
        {
            return Some(tool);
        }
    }
    if cmd == "ipc-bridge" {
        return Some("hypr-ipc-bridge");
    }
    None
}

fn set_process_name(name: &str) {
    let max_len = 15.min(name.len());
    let truncated = &name[..max_len];
    if let Ok(c_name) = std::ffi::CString::new(truncated) {
        unsafe {
            libc::prctl(libc::PR_SET_NAME, c_name.as_ptr() as libc::c_ulong, 0, 0, 0);
        }
    }
}

fn parse_signal(sig_str: Option<&str>) -> i32 {
    match sig_str {
        None => libc::SIGUSR1,
        Some("SIGUSR1") | Some("USR1") | Some("10") => libc::SIGUSR1,
        Some("SIGUSR2") | Some("USR2") | Some("12") => libc::SIGUSR2,
        Some("SIGHUP") | Some("HUP") | Some("1") => libc::SIGHUP,
        Some("SIGTERM") | Some("TERM") | Some("15") => libc::SIGTERM,
        Some("SIGINT") | Some("INT") | Some("2") => libc::SIGINT,
        Some(num) => num.parse::<i32>().unwrap_or(libc::SIGUSR1),
    }
}

fn print_help() {
    println!("inerium-ctl — Multicall dispatcher and IPC manager for Inerium Shell");
    println!();
    println!("USAGE:");
    println!("    inerium-ctl <SUBCOMMAND> [ARGS...]");
    println!("    <tool-symlink> [ARGS...]");
    println!();
    println!("SUBCOMMANDS:");
    println!("    notify <target> [signal]   Send signal to target daemon (default: SIGUSR1)");
    println!("    notify-sound [options]     Play notification sound with debounce (--sound, --cooldown-ms)");
    println!();
    println!("AVAILABLE TOOLS:");
    for tool in TOOLS {
        println!("    {}", tool);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let raw_arg0 = args.first().cloned().unwrap_or_default();
    let bin_name = Path::new(&raw_arg0)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&raw_arg0);

    // 1. Direct symlink dispatch based on argv[0] basename
    if let Some(tool) = canonical_tool_name(bin_name) {
        set_process_name(tool);
        let mut tool_args = args.clone();
        if !tool_args.is_empty() {
            tool_args[0] = tool.to_string();
        } else {
            tool_args.push(tool.to_string());
        }
        if let Err(e) = run_tool(tool, &tool_args) {
            eprintln!("Error running {tool}: {e}");
            process::exit(1);
        }
        return;
    }

    // 2. Multicall invocation as inerium-ctl
    if args.len() < 2 {
        print_help();
        return;
    }

    let subcmd = &args[1];
    match subcmd.as_str() {
        "-h" | "--help" | "help" => {
            print_help();
        }
        "notify" => {
            if args.len() < 3 {
                eprintln!("Usage: inerium-ctl notify <target> [signal]");
                process::exit(1);
            }
            let target = &args[2];
            let sig = parse_signal(args.get(3).map(|s| s.as_str()));

            match PidRegistry::notify_all(target, sig) {
                Ok(0) => {
                    eprintln!("No active process found for target: {target}");
                    process::exit(1);
                }
                Ok(_count) => {
                    // Success (exits with 0)
                }
                Err(e) => {
                    eprintln!("Error notifying {target}: {e}");
                    process::exit(1);
                }
            }
        }
        "notify-sound" => {
            inerium_tools::tools::notify_sound::run(&args[2..]);
        }
        other => {
            if let Some(tool) = canonical_tool_name(other) {
                set_process_name(tool);
                let mut tool_args = vec![tool.to_string()];
                tool_args.extend_from_slice(&args[2..]);
                if let Err(e) = run_tool(tool, &tool_args) {
                    eprintln!("Error running {tool}: {e}");
                    process::exit(1);
                }
            } else {
                eprintln!("Unknown command or tool: {other}");
                eprintln!("Run 'inerium-ctl help' for available tools.");
                process::exit(1);
            }
        }
    }
}
