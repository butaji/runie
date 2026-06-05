//! PTY wrapper for TUI.
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use std::thread;

fn main() {
    let (master_fd, slave_fd) = unsafe {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        let mut win: libc::winsize = std::mem::zeroed();
        win.ws_row = 24;
        win.ws_col = 80;
        let mut name: [libc::c_char; 128] = [0; 128];
        if libc::openpty(&mut master, &mut slave, name.as_mut_ptr(), std::ptr::null_mut(), &mut win) < 0 {
            eprintln!("PTY: Failed to create");
            std::process::exit(1);
        }
        (master, slave)
    };

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        unsafe { libc::close(master_fd) };
        unsafe { libc::setsid() };
        let _ = unsafe { libc::ioctl(slave_fd, libc::TIOCSCTTY as libc::c_ulong, 0) };
        unsafe {
            libc::dup2(slave_fd, 0);
            libc::dup2(slave_fd, 1);
            libc::dup2(slave_fd, 2);
            libc::close(slave_fd);
        }
        std::env::set_current_dir("/Users/admin/Code/GitHub/runie").ok();
        Command::new("./target/debug/runie-tui")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .exec();
        unsafe { libc::_exit(1) };
    }

    if pid < 0 {
        eprintln!("PTY: Fork failed");
        std::process::exit(1);
    }

    unsafe { libc::close(slave_fd) };

    // Spawn thread to forward stdin to PTY
    let master_for_input = master_fd;
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 256];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let written = unsafe { libc::write(master_for_input, buf.as_ptr() as *const libc::c_void, n) };
                    if written < 0 { break; }
                }
            }
        }
    });

    // Forward output from PTY to stdout
    let mut buf = [0u8; 4096];
    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        let mut status: libc::c_int = 0;
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result != 0 {
            break;
        }
        let n = unsafe { libc::read(master_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n > 0 {
            std::io::stdout().write_all(&buf[..n as usize]).ok();
            std::io::stdout().flush().ok();
        } else {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    unsafe { libc::close(master_fd) };
    unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
}
