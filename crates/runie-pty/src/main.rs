//! PTY wrapper for TUI using portable-pty.
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::thread;

fn main() {
    let pty_system = native_pty_system();
    
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    let mut cmd = CommandBuilder::new("./target/debug/runie-tui");
    cmd.cwd("/Users/admin/Code/GitHub/runie");
    
    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn TUI");
    
    let mut reader = pair.master.try_clone_reader().expect("Failed to clone reader");
    let mut writer = pair.master.take_writer().expect("Failed to get writer");
    
    // Thread to read stdin and forward to PTY
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(1) => {
                    if writer.write_all(&buf).is_err() { break; }
                    if writer.flush().is_err() { break; }
                }
                Ok(_) => {}
            }
        }
    });
    
    // Main loop - forward PTY output to stdout
    let mut buf = [0u8; 4096];
    loop {
        // Check if child exited
        match child.try_wait() {
            Ok(Some(_)) => break,
            _ => {}
        }
        
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                std::io::stdout().write_all(&buf[..n]).ok();
                std::io::stdout().flush().ok();
            }
            Err(_) => break,
        }
    }
}
