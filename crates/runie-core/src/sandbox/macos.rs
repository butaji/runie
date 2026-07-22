//! macOS Seatbelt sandbox implementation.
//!
//! Uses `sandbox-exec` for kernel-enforced sandboxing on macOS.
//! The sandbox profile is generated based on the sandbox configuration.

use std::path::Path;
use std::process::Command;

use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};

use super::profiles::{Profile, SandboxConfig};

/// Check if sandbox-exec is available on macOS.
#[cfg(target_os = "macos")]
pub fn sandbox_available() -> super::manager::SandboxStatus {
    if Command::new("sandbox-exec").arg("--version").output().is_ok() {
        super::manager::SandboxStatus::Available
    } else {
        super::manager::SandboxStatus::Unavailable {
            reason: "sandbox-exec not available".to_owned(),
        }
    }
}

/// Check if sandbox is available (non-macOS stub).
#[cfg(not(target_os = "macos"))]
pub fn sandbox_available() -> super::manager::SandboxStatus {
    super::manager::SandboxStatus::Unavailable {
        reason: "sandbox-exec only available on macOS".to_owned(),
    }
}

/// Execute a command with OS-level sandboxing if available.
pub fn run_sandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    let status = sandbox_available();

    match status {
        super::manager::SandboxStatus::Unavailable { reason } => {
            warn!("Sandbox unavailable, running unsandboxed: {}", reason);
            run_unsandboxed(program, args, working_dir, env)
        }
        super::manager::SandboxStatus::Available => {
            if config.profile == Profile::Off {
                debug!("Sandbox profile is Off, running unsandboxed");
                return run_unsandboxed(program, args, working_dir, env);
            }

            #[cfg(target_os = "macos")]
            {
                run_mac_sandboxed(program, args, working_dir, env, config)
            }

            #[cfg(not(target_os = "macos"))]
            {
                warn!("No sandbox implementation for this platform, running unsandboxed");
                run_unsandboxed(program, args, working_dir, env)
            }
        }
    }
}

/// Run command without sandboxing (fallback).
pub fn run_unsandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
) -> Result<std::process::ExitStatus, String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))
}

/// Run a macOS sandboxed command.
#[cfg(target_os = "macos")]
pub fn run_mac_sandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    let cwd = working_dir
        .to_str()
        .ok_or_else(|| "Invalid working directory".to_owned())?;

    let profile = build_mac_sandbox_profile(cwd, config);

    let mut cmd = Command::new("sandbox-exec");
    cmd.arg("-p")
        .arg(&profile)
        .arg(program)
        .args(args)
        .current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    info!(
        "Running sandboxed: {} {:?} (profile: {:?})",
        program, args, config.profile
    );

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn sandboxed command: {}", e))?
        .wait()
        .map_err(|e| format!("Failed to wait for sandboxed command: {}", e))
}

/// Linux sandbox implementation (stub when not on Linux).
#[cfg(not(target_os = "macos"))]
pub fn run_mac_sandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    warn!(
        "macOS sandbox not available on this platform, running unsandboxed"
    );
    run_unsandboxed(program, args, working_dir, env)
}

/// Run a Linux sandboxed command.
#[cfg(target_os = "linux")]
pub fn run_linux_sandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    if config.profile == Profile::Off {
        return run_unsandboxed(program, args, working_dir, env);
    }

    #[cfg(feature = "landlock")]
    {
        return run_landlock_native(program, args, working_dir, env, config);
    }

    warn!("Landlock not available, running unsandboxed");
    run_unsandboxed(program, args, working_dir, env)
}

/// Linux sandbox stub for non-Linux platforms.
#[cfg(not(target_os = "linux"))]
pub fn run_linux_sandboxed(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    _config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    warn!("Linux landlock not available on this platform");
    run_unsandboxed(program, args, working_dir, env)
}

/// Native Landlock implementation.
#[cfg(all(feature = "landlock", target_os = "linux"))]
pub fn run_landlock_native(
    program: &str,
    args: &[&str],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<std::process::ExitStatus, String> {
    use landlock::{Access, AccessFs, Rule, Ruleset, RulesetStatus};

    let mut ruleset = Ruleset::new();

    // Add read-only paths
    for path in &config.read_only_paths {
        if let Some(path_str) = path.to_str() {
            let _ = ruleset.add_rule(
                Rule::Directory {
                    path: path_str,
                    access: AccessFs::from_bits(Access::Read.bits()),
                }
                .least_access(),
            );
        }
    }

    // Add read-write paths
    for path in &config.read_write_paths {
        if let Some(path_str) = path.to_str() {
            let _ = ruleset.add_rule(
                Rule::Directory {
                    path: path_str,
                    access: AccessFs::from_bits(Access::Read.bits() | Access::Write.bits()),
                }
                .least_access(),
            );
        }
    }

    // Add workspace if specified
    if let Some(ws) = &config.workspace_root {
        if let Some(ws_str) = ws.to_str() {
            let access = if config.profile == Profile::Strict {
                Access::Read.bits()
            } else {
                Access::Read.bits() | Access::Write.bits()
            };
            let _ = ruleset.add_rule(
                Rule::Directory {
                    path: ws_str,
                    access: AccessFs::from_bits(access),
                }
                .least_access(),
            );
        }
    }

    // Add tmp
    let _ = ruleset.add_rule(
        Rule::Directory {
            path: "/tmp",
            access: AccessFs::from_bits(Access::Read.bits() | Access::Write.bits()),
        }
        .least_access(),
    );

    let ruleset = ruleset
        .create()
        .map_err(|e| format!("Failed to create landlock ruleset: {}", e))?;

    ruleset
        .restrict_self()
        .map_err(|e| format!("Failed to restrict with landlock: {}", e))?;

    match ruleset.status() {
        RulesetStatus::FullyEnforced => info!("Landlock ruleset fully enforced"),
        RulesetStatus::PartiallyEnforced => warn!("Landlock ruleset partially enforced"),
        RulesetStatus::NotEnforced => warn!("Landlock ruleset not enforced"),
    }

    run_unsandboxed(program, args, working_dir, env)
}

/// Build a macOS sandbox profile based on config.
#[cfg(target_os = "macos")]
#[allow(clippy::too_many_lines)]
pub fn build_mac_sandbox_profile(_cwd: &str, config: &SandboxConfig) -> String {
    let mut rules = String::from("(version 1)\n(allow default)\n");

    // Deny patterns
    for pattern in &config.deny_patterns {
        rules.push_str(&format!(
            "(deny file-read* (regex #\"^{}\"))\n",
            regex_escape(pattern)
        ));
        rules.push_str(&format!(
            "(deny file-write* (regex #\"^{}\"))\n",
            regex_escape(pattern)
        ));
    }

    // Profile-specific rules
    match config.profile {
        Profile::Strict => {
            // Read-only system paths
            for path in &["/usr", "/System", "/bin", "/lib", "/etc"] {
                rules.push_str(&format!(
                    "(allow file-read* (regex #\"^{}/.*\"))\n",
                    regex_escape(path)
                ));
            }
            // Workspace write
            if let Some(ws) = &config.workspace_root {
                if let Some(ws_str) = ws.to_str() {
                    rules.push_str(&format!(
                        "(allow file-write* (regex #\"^{}/.*\"))\n",
                        regex_escape(ws_str)
                    ));
                }
            }
            // No network
            rules.push_str("(deny network*)\n");
        }
        Profile::Workspace => {
            // Read all
            rules.push_str("(allow file-read*)\n");
            // Write only to workspace
            if let Some(ws) = &config.workspace_root {
                if let Some(ws_str) = ws.to_str() {
                    rules.push_str(&format!(
                        "(allow file-write* (regex #\"^{}/.*\"))\n",
                        regex_escape(ws_str)
                    ));
                }
            }
        }
        Profile::Devbox => {
            // Read all
            rules.push_str("(allow file-read*)\n");
            // Write to allowed paths
            for path in &config.read_write_paths {
                if let Some(path_str) = path.to_str() {
                    rules.push_str(&format!(
                        "(allow file-write* (regex #\"^{}/.*\"))\n",
                        regex_escape(path_str)
                    ));
                }
            }
        }
        _ => {}
    }

    rules
}

/// Build a macOS sandbox profile (stub for non-macOS).
#[cfg(not(target_os = "macos"))]
pub fn build_mac_sandbox_profile(cwd: &str, config: &SandboxConfig) -> String {
    // Return a minimal profile that allows everything
    // This is a stub for non-macOS platforms
    format!(
        "(version 1)\n(allow default)\n;; cwd: {}\n;; profile: {:?}\n",
        cwd, config.profile
    )
}

/// Escape special regex characters for macOS sandbox profile.
fn regex_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('^', "\\^")
        .replace('$', "\\$")
}

// ── Async wrappers ─────────────────────────────────────────────────────────────

/// Run a sandboxed shell command with async process management.
#[cfg(target_os = "macos")]
pub async fn run_sandboxed_shell_async(
    command: &str,
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<tokio::process::Child, String> {
    let cwd = working_dir
        .to_str()
        .ok_or_else(|| "Invalid working directory".to_owned())?;
    let profile = build_mac_sandbox_profile(cwd, config);

    let mut cmd = AsyncCommand::new("sandbox-exec");
    cmd.arg("-p")
        .arg(&profile)
        .arg("sh")
        .arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn sandboxed command: {}", e))
}

#[cfg(not(target_os = "macos"))]
pub async fn run_sandboxed_shell_async(
    command: &str,
    working_dir: &Path,
    env: &[(String, String)],
    _config: &SandboxConfig,
) -> Result<tokio::process::Child, String> {
    let mut cmd = AsyncCommand::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn shell command: {}", e))
}

/// Run a sandboxed direct command with async process management.
#[cfg(target_os = "macos")]
pub async fn run_sandboxed_direct_async(
    program: &str,
    args: &[String],
    working_dir: &Path,
    env: &[(String, String)],
    config: &SandboxConfig,
) -> Result<tokio::process::Child, String> {
    let cwd = working_dir
        .to_str()
        .ok_or_else(|| "Invalid working directory".to_owned())?;
    let profile = build_mac_sandbox_profile(cwd, config);

    let mut cmd = AsyncCommand::new("sandbox-exec");
    cmd.arg("-p").arg(&profile).arg(program);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn sandboxed direct command: {}", e))
}

#[cfg(not(target_os = "macos"))]
pub async fn run_sandboxed_direct_async(
    program: &str,
    args: &[String],
    working_dir: &Path,
    env: &[(String, String)],
    _config: &SandboxConfig,
) -> Result<tokio::process::Child, String> {
    let mut cmd = AsyncCommand::new(program);
    cmd.args(args);
    cmd.current_dir(working_dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    cmd.spawn()
        .map_err(|e| format!("Failed to spawn direct command: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_escape_works() {
        // *.txt becomes .*\.txt (glob to regex: * → .*)
        assert_eq!(regex_escape("*.txt"), ".*\\.txt".to_string());
        // foo.bar becomes foo\.bar (escape dots)
        assert_eq!(regex_escape("foo.bar"), "foo\\.bar".to_string());
        // /etc/.* becomes /etc/\\..* (escape dot, convert * to .*)
        assert_eq!(regex_escape("/etc/.*"), "/etc/\\..*".to_string());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_sandbox_profile_builds() {
        let config = SandboxConfig::workspace("/tmp/test".into());
        let profile = build_mac_sandbox_profile("/tmp/test", &config);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(allow default)"));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_profile_stub() {
        let config = SandboxConfig::workspace("/tmp/test".into());
        let profile = build_mac_sandbox_profile("/tmp/test", &config);
        assert!(profile.contains("workspace"));
    }
}
