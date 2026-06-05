//! Runie TUI - main binary with state persistence.
use std::process;

fn main() {
    if let Err(e) = runie_tui_lib::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
