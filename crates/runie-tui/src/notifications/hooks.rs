//! Optional, bounded notification hooks.
//!
//! `RUNIE_NOTIFICATION_HOOK` is intentionally opt-in until the full
//! notification TOML schema lands. Hooks run off the render loop and are
//! killed when their timeout expires.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run_hook(event: &str, message: &str) {
    let Ok(command) = std::env::var("RUNIE_NOTIFICATION_HOOK") else { return };
    if command.trim().is_empty() {
        return;
    }
    let event = event.to_owned();
    let message = message.to_owned();
    let timeout = std::env::var("RUNIE_NOTIFICATION_HOOK_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10)
        .max(1);
    std::thread::spawn(move || {
        let mut child = match Command::new("sh")
            .arg("-c")
            .arg(command)
            .env("RUNIE_EVENT", &event)
            .env("RUNIE_MESSAGE", &message)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                tracing::debug!(%error, "notification hook spawn failed");
                return;
            }
        };
        let deadline = Instant::now() + Duration::from_secs(timeout);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                Err(error) => {
                    tracing::debug!(%error, "notification hook wait failed");
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_hook_is_a_noop() {
        super::run_hook("TurnComplete", "done");
    }
}
