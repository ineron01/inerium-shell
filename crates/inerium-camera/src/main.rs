#![allow(clippy::collapsible_if)]

mod bridge;
mod cli;
mod device;
mod gui;
mod preview;
mod scrcpy;
mod types;

use cli::{handle_cli, parse_cli_args, Commands};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize tokio runtime");

    let _guard = rt.enter();

    let cmd = parse_cli_args();
    match cmd {
        Commands::Gui => {
            gui::run_gui();
        }
        other => {
            rt.block_on(handle_cli(other));
        }
    }
}
