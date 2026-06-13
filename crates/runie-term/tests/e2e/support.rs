//! End-to-end smoke tests for the runie terminal binary.
//!
//! These tests spawn the real release binary in a PTY and drive it with
//! `rexpect`. They complement the fast Layer 1-3 unit tests by exercising
//! the real async event loop, rendering actor, and command flows.
//!
//! Run with: `cargo test -p runie-term --test e2e -- --ignored`

use rexpect::error::Error;
use rexpect::session::PtySession;
use std::env;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

/// Holds a temporary HOME directory so e2e tests don't pollute the user's
/// real data dir (sessions, clipboard, etc.), while dereferencing to the
/// underlying `rexpect` PTY session.
pub struct IsolatedSession {
    #[allow(dead_code)]
    home: tempfile::TempDir,
    pty: PtySession,
}

impl Deref for IsolatedSession {
    type Target = PtySession;

    fn deref(&self) -> &Self::Target {
        &self.pty
    }
}

impl DerefMut for IsolatedSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pty
    }
}

pub fn binary_path() -> PathBuf {
    env::var("CARGO_BIN_EXE_runie")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::current_dir()
                .unwrap()
                .join("target")
                .join("release")
                .join("runie")
        })
}

pub fn spawn_runie() -> Result<IsolatedSession, Error> {
    spawn_with_setup(|_| {})
}

/// Spawn runie, running `setup(home_path)` before the binary starts so
/// tests can pre-populate the isolated home directory (e.g. corrupt
/// config, saved sessions, etc.) without touching the user's real data.
pub fn spawn_with_setup<F: FnOnce(&std::path::Path)>(setup: F) -> Result<IsolatedSession, Error> {
    let bin = binary_path();
    if !bin.exists() {
        panic!(
            "runie binary not found at {}. Build it with: cargo build --release -p runie-term",
            bin.display()
        );
    }

    // Spawn through a shell so we can set the PTY size before the app
    // starts. This avoids a race where the app queries the terminal size
    // before we resize it. We also override HOME so tests don't touch the
    // user's real data directory.
    let home = tempfile::tempdir().expect("create temp home dir");
    setup(home.path());
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = std::process::Command::new(&shell);
    cmd.arg("-c")
        .arg(format!("stty rows 24 cols 80; exec '{}'", bin.display()))
        .env("HOME", home.path())
        // E2E tests rely on the mock provider being available and selected
        // by default. In production (no flags) the app would auto-open the
        // login dialog, which would break these tests.
        .env("RUNIE_MOCK", "1")
        .env("RUNIE_MOCK_DELAY", "1");
    let session = rexpect::session::spawn_command(cmd, Some(30_000))?;
    Ok(IsolatedSession { home, pty: session })
}

/// Wait until the screen contains the given literal text.
pub fn wait_for(p: &mut PtySession, text: &str) -> Result<(), Error> {
    p.exp_string(text).map(|_| ())
}

/// Send text followed by Enter (carriage return, as a real terminal emits).
pub fn send_line(p: &mut PtySession, text: &str) -> Result<(), Error> {
    p.send(text)?;
    p.send("\r")?;
    p.flush()
}

/// Send raw text without Enter.
pub fn send(p: &mut PtySession, text: &str) -> Result<(), Error> {
    p.send(text)?;
    p.flush()
}

/// Send Escape key.
pub fn send_escape(p: &mut PtySession) -> Result<(), Error> {
    p.send("\x1b")?;
    p.flush()
}

/// Send Ctrl+C.
pub fn send_ctrl_c(p: &mut PtySession) -> Result<(), Error> {
    p.send_control('c')?;
    p.flush()
}

/// Send Ctrl+U (clear line).
pub fn send_ctrl_u(p: &mut PtySession) -> Result<(), Error> {
    p.send_control('u')?;
    p.flush()
}

/// Send Tab.
pub fn send_tab(p: &mut PtySession) -> Result<(), Error> {
    p.send("\t")?;
    p.flush()
}

/// Send a raw CSI sequence (ESC [ <seq> ).
pub fn send_csi(p: &mut PtySession, seq: &str) -> Result<(), Error> {
    p.send(&format!("\x1b[{}]", seq))?;
    p.flush()
}

/// Resize the PTY to the given dimensions and notify the child.
pub fn resize(p: &mut PtySession, rows: u16, cols: u16) -> Result<(), Error> {
    use std::os::unix::io::AsRawFd;
    let ws = nix::pty::Winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        let r = libc::ioctl(
            p.process().pty.as_raw_fd(),
            libc::TIOCSWINSZ,
            &ws as *const _,
        );
        if r < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
    }
    // Crossterm listens for SIGWINCH; make sure the child actually receives
    // one so it re-reads the terminal size instead of waiting indefinitely.
    p.process_mut().signal(nix::sys::signal::Signal::SIGWINCH)?;
    Ok(())
}

/// Wait until the app is back at the idle prompt.
pub fn wait_idle(p: &mut PtySession) {
    wait_for(p, "Type a message to start").expect("idle prompt");
}

pub fn open_cancel(p: &mut PtySession, command: &str, expected: &str) {
    run_command(p, command);
    wait_for(p, expected).unwrap_or_else(|_| panic!("{} form", command));
    send_escape(p).expect("cancel");
    wait_idle(p);
}

pub fn run_wait(p: &mut PtySession, command: &str, expected: &str) {
    run_command(p, command);
    wait_for(p, expected).expect(expected);
    wait_idle(p);
}

/// Open the command palette and wait for it to render.
pub fn open_palette(p: &mut PtySession) {
    send(p, "\x10").expect("ctrl-p palette");
    wait_for(p, "Commands").expect("palette title");
}

/// Type a command name in the palette and press Enter to run it.
pub fn run_command(p: &mut PtySession, name: &str) {
    open_palette(p);
    send_line(p, name).unwrap_or_else(|_| panic!("type {name}"));
}

/// Assert that the captured output contains no panic indicators.
pub fn assert_no_panic(output: &str) {
    assert!(
        !output.to_lowercase().contains("panic"),
        "panic detected: {}",
        output
    );
    assert!(
        !output.contains("thread '"),
        "thread panic detected: {}",
        output
    );
}

/// Assert that no status timer has been stuck for 1000+ seconds.
pub fn assert_no_stuck_timer(output: &str) {
    use regex::Regex;
    let re = Regex::new(r"\d{4}\.\d+s").expect("valid regex");
    assert!(!re.is_match(output), "stuck timer detected: {}", output);
}

/// Kill the process gracefully and return the full captured PTY output.
pub fn capture_output(p: &mut PtySession) -> String {
    p.process_mut().set_kill_timeout(Some(2_000));
    p.process_mut().exit().ok();
    p.exp_eof().unwrap_or_default()
}
