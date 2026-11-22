use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::time::Duration;

pub fn run(args: &[String]) {
    let popup_args: Vec<String> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        Vec::new()
    };
    if popup_args.is_empty() {
        eprintln!("Usage: inerium-popup <popup_name> [args...]");
        std::process::exit(1);
    }

    let sock_path = crate::sys::runtime_dir().join("inerium-popups.sock");

    let mut connected = false;
    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
        let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));

        let msg = format!("{}\n", popup_args.join(" "));
        if stream.write_all(msg.as_bytes()).is_ok() {
            let mut buf = [0u8; 16];
            let _ = stream.read(&mut buf);
            connected = true;
        }
    }

    if !connected {
        // Fallback: start daemon via systemd and retry connection
        let _ = Command::new("systemctl")
            .args(["--user", "start", "inerium-popups.service"])
            .status();

        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(50));
            if let Ok(mut stream) = UnixStream::connect(&sock_path) {
                let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
                let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
                let msg = format!("{}\n", popup_args.join(" "));
                if stream.write_all(msg.as_bytes()).is_ok() {
                    let mut buf = [0u8; 16];
                    let _ = stream.read(&mut buf);
                    break;
                }
            }
        }
    }
}
