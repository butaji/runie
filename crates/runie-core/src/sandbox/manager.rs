//! Sandbox manager and status tracking.
//!
//! Provides the `SandboxManager` which tracks the active sandbox profile
//! and logs violations for review. Actual kernel enforcement is handled
//! by platform-specific implementations (macOS Seatbelt, Linux Landlock).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, info, warn};

use super::deny::{ReadDenyList, WriteDenyList};
use super::profiles::{Profile, SandboxConfig};

/// Global sandbox manager instance.
static SANDBOX_ACTIVE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static CURRENT_PROFILE: std::cell::RefCell<Option<Arc<SandboxManager>>> = const { std::cell::RefCell::new(None) };
}

/// Result of sandbox initialization or availability check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxStatus {
    /// Sandbox is available and ready to use.
    Available,
    /// Sandbox is not available on this platform.
    Unavailable { reason: String },
}

impl Default for SandboxStatus {
    fn default() -> Self {
        sandbox_available()
    }
}

/// Check if OS-level sandboxing is available on this platform.
pub fn sandbox_available() -> SandboxStatus {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if Command::new("sandbox-exec").arg("--version").output().is_ok() {
            SandboxStatus::Available
        } else {
            SandboxStatus::Unavailable {
                reason: "sandbox-exec not available".to_owned(),
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        SandboxStatus::Available
    }

    #[cfg(target_os = "windows")]
    {
        SandboxStatus::Available
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        SandboxStatus::Unavailable {
            reason: format!("Unsupported platform: {}", std::env::consts::OS),
        }
    }
}

/// Sandbox manager for tracking violations and applying profiles.
///
/// Even without kernel enforcement, the manager can:
/// - Track which profile is active
/// - Log sandbox violations for review
/// - Provide violation callbacks for testing
pub struct SandboxManager {
    /// Active profile name.
    pub profile: Profile,
    /// Sandbox configuration.
    pub config: SandboxConfig,
    /// Read deny list.
    read_deny: ReadDenyList,
    /// Write deny list.
    write_deny: WriteDenyList,
    /// Whether sandbox is currently enforced.
    enforced: bool,
    /// Violation log path.
    violation_log: Option<PathBuf>,
}

impl std::fmt::Debug for SandboxManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxManager")
            .field("profile", &self.profile)
            .field("enforced", &self.enforced)
            .field("violation_log", &self.violation_log)
            .finish()
    }
}

impl SandboxManager {
    /// Create a new sandbox manager with the given profile.
    pub fn new(profile: Profile, config: SandboxConfig) -> Self {
        let read_deny = ReadDenyList::from_profile(profile);
        let write_deny = WriteDenyList::from_profile(profile);

        Self {
            profile,
            config,
            read_deny,
            write_deny,
            enforced: false,
            violation_log: None,
        }
    }

    /// Create a sandbox manager for the workspace profile.
    pub fn workspace(workspace_root: PathBuf) -> Self {
        let config = SandboxConfig::workspace(workspace_root);
        Self::new(Profile::Workspace, config)
    }

    /// Create a sandbox manager for the strict profile.
    pub fn strict(workspace_root: PathBuf) -> Self {
        let config = SandboxConfig::strict(workspace_root);
        Self::new(Profile::Strict, config)
    }

    /// Create a sandbox manager for the devbox profile.
    pub fn devbox(workspace_root: PathBuf) -> Self {
        let config = SandboxConfig::devbox(workspace_root);
        Self::new(Profile::Devbox, config)
    }

    /// Set the violation log path.
    pub fn with_violation_log(mut self, path: PathBuf) -> Self {
        self.violation_log = Some(path);
        self
    }

    /// Apply the sandbox (mark as active globally).
    ///
    /// Returns an error if sandboxing is not available on this platform.
    pub fn apply(&mut self) -> Result<(), String> {
        let status = sandbox_available();

        match status {
            SandboxStatus::Unavailable { reason } => {
                warn!("Sandbox unavailable, cannot apply: {}", reason);
                Err(reason)
            }
            SandboxStatus::Available => {
                // Store globally for violation logging
                CURRENT_PROFILE.with(|p| { *p.borrow_mut() = Some(Arc::new(self.clone())); });
                SANDBOX_ACTIVE.store(true, Ordering::SeqCst);
                self.enforced = true;
                info!("Sandbox applied: profile={:?}", self.profile);
                Ok(())
            }
        }
    }

    /// Check if the sandbox is currently active.
    pub fn is_active() -> bool {
        SANDBOX_ACTIVE.load(Ordering::SeqCst)
    }

    /// Get the current sandbox manager, if any.
    pub fn current() -> Option<Arc<Self>> {
        CURRENT_PROFILE.with(|p| p.borrow().clone())
    }

    /// Check if a path should be denied for reading.
    pub fn should_deny_read(&self, path: &Path) -> bool {
        self.read_deny.is_denied(path)
    }

    /// Check if a path should be denied for writing.
    pub fn should_deny_write(&self, path: &Path) -> bool {
        self.write_deny.is_denied(path)
    }

    /// Log a sandbox violation.
    pub fn log_violation(&self, kind: &str, path: &Path, operation: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = format!(
            "[{}] {} violation: {} on {} (profile={:?})\n",
            timestamp,
            kind,
            operation,
            path.display(),
            self.profile
        );

        debug!("Sandbox violation: {} {} on {}", kind, operation, path.display());

        if let Some(log_path) = &self.violation_log {
            if let Err(e) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(entry.as_bytes())
                })
            {
                warn!("Failed to write sandbox violation log: {}", e);
            }
        }
    }

    /// Check and log a read violation if the path is denied.
    pub fn check_read(&self, path: &Path) -> bool {
        if self.should_deny_read(path) {
            self.log_violation("read", path, "read");
            true
        } else {
            false
        }
    }

    /// Check and log a write violation if the path is denied.
    pub fn check_write(&self, path: &Path) -> bool {
        if self.should_deny_write(path) {
            self.log_violation("write", path, "write");
            true
        } else {
            false
        }
    }

    /// Get the macOS sandbox profile string for this manager.
    #[cfg(target_os = "macos")]
    pub fn macos_profile(&self) -> String {
        let cwd = self
            .config
            .workspace_root
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("/tmp");

        super::macos::build_mac_sandbox_profile(cwd, &self.config)
    }

    /// Reset the global sandbox state (for testing).
    #[cfg(test)]
    pub fn reset() {
        SANDBOX_ACTIVE.store(false, Ordering::SeqCst);
        CURRENT_PROFILE.with(|p| { *p.borrow_mut() = None; });
    }
}

impl Clone for SandboxManager {
    fn clone(&self) -> Self {
        Self {
            profile: self.profile,
            config: self.config.clone(),
            read_deny: self.read_deny.clone(),
            write_deny: self.write_deny.clone(),
            enforced: self.enforced,
            violation_log: self.violation_log.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_status_debug() {
        let status = SandboxStatus::Available;
        assert!(format!("{:?}", status).contains("Available"));

        let unavailable = SandboxStatus::Unavailable {
            reason: "test".to_owned(),
        };
        assert!(format!("{:?}", unavailable).contains("Unavailable"));
    }

    #[test]
    fn manager_workspace() {
        let manager = SandboxManager::workspace("/tmp/test".into());
        assert_eq!(manager.profile, Profile::Workspace);
        assert!(manager.config.workspace_root.is_some());
    }

    #[test]
    fn manager_strict() {
        let manager = SandboxManager::strict("/tmp/test".into());
        assert_eq!(manager.profile, Profile::Strict);
        assert!(manager.config.restrict_network);
    }

    #[test]
    fn manager_check_paths() {
        let manager = SandboxManager::strict("/workspace".into());

        // Workspace should be allowed
        assert!(!manager.should_deny_read(Path::new("/workspace/src/main.rs")));
        assert!(!manager.should_deny_write(Path::new("/workspace/src/main.rs")));

        // /etc should be denied for writing in strict mode
        assert!(manager.should_deny_write(Path::new("/etc/passwd")));
    }

    #[test]
    fn manager_check_read_violation() {
        let manager = SandboxManager::strict("/workspace".into());
        assert!(manager.check_read(Path::new("/etc/shadow")));
    }

    #[test]
    fn manager_check_write_violation() {
        let manager = SandboxManager::devbox("/workspace".into());

        // /data should be denied in devbox mode
        assert!(manager.should_deny_write(Path::new("/data/project")));
    }

    #[test]
    fn manager_clone() {
        let manager = SandboxManager::workspace("/tmp".into());
        let cloned = manager.clone();
        assert_eq!(cloned.profile, manager.profile);
    }
}
