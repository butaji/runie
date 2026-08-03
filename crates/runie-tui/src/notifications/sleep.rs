//! Best-effort idle-sleep inhibition while an agent turn is active.

use std::cell::{Cell, RefCell};

pub struct SleepInhibitor {
    enabled: bool,
    active: Cell<bool>,
    unavailable: Cell<bool>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    child: RefCell<Option<std::process::Child>>,
}

impl SleepInhibitor {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            active: Cell::new(false),
            unavailable: Cell::new(false),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            child: RefCell::new(None),
        }
    }

    pub fn inhibit(&self) {
        if !self.enabled || self.active.get() || self.unavailable.get() {
            return;
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let (program, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
                ("caffeinate", &["-dimsu"])
            } else {
                (
                    "systemd-inhibit",
                    &["--what=idle", "--who=runie", "--why=agent turn in progress", "sleep", "infinity"],
                )
            };
            let child = std::process::Command::new(program)
                .args(args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            match child {
                Ok(child) => {
                    *self.child.borrow_mut() = Some(child);
                    self.active.set(true);
                }
                Err(error) => {
                    tracing::debug!(%error, "platform sleep inhibitor unavailable");
                    self.unavailable.set(true);
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        self.unavailable.set(true);
    }

    pub fn release(&self) {
        if !self.active.replace(false) {
            return;
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Some(mut child) = self.child.borrow_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    #[cfg(test)]
    fn is_active(&self) -> bool {
        self.active.get()
    }
}

impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::SleepInhibitor;
    #[test]
    fn disabled_inhibitor_is_idempotent_and_inactive() {
        let inhibitor = SleepInhibitor::new(false);
        inhibitor.inhibit();
        inhibitor.inhibit();
        inhibitor.release();
        assert!(!inhibitor.is_active());
    }
}
