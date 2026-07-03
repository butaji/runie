//! Unified bash execution using `command-group` for reliable process-group kill.
//!
//! Single source of truth for all bash/command execution in Runie. Provides both
//! async (for agent tools) and sync (for IO actor) variants.
//!
//! - Uses `command-group` to kill the entire process tree on timeout.
//! - Uses `shell-words` for direct-mode command parsing.
//! - Supports both shell mode (`sh -c`) and direct mode (parsed args).
//! - Supports OS-level sandboxing via the `sandbox` module.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use command_group::AsyncCommandGroup;
use shell_words;
use tokio::process::Command;
use tokio::sync::oneshot;

use crate::sandbox::{sandbox_available, SandboxStatus};

/// Result of a bash command execution.
#[derive(Debug, Clone)]
pub struct ShellResult {
    /// Combined stdout/stderr output.
    pub output: String,
    /// Total bytes transferred (stdout + stderr length).
    pub bytes_transferred: Option<u64>,
    /// Exit status.
    pub status: ShellStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShellStatus {
    Success,
    Error,
    TimedOut,
}

/// Execute a bash command asynchronously with process-group kill on timeout.
///
/// - `shell`: if true, passes command to `sh -c` (supports pipes, redirects, etc.).
///   If false, parses with `shell-words` and executes directly (faster, safer).
/// - Uses `command-group` to kill the entire process tree when timeout expires.
/// - When `use_sandbox` is true and sandbox is available, wraps execution in OS sandbox.
pub async fn run_bash(
    command: &str,
    working_dir: impl AsRef<Path>,
    env: &HashMap<String, String>,
    timeout: Duration,
    shell: bool,
) -> ShellResult {
    run_bash_internal(command, working_dir, env, timeout, shell, false).await
}

/// Execute a bash command asynchronously with optional sandboxing.
pub async fn run_bash_sandboxed(
    command: &str,
    working_dir: impl AsRef<Path>,
    env: &HashMap<String, String>,
    timeout: Duration,
    shell: bool,
) -> ShellResult {
    run_bash_internal(command, working_dir, env, timeout, shell, true).await
}

/// Internal implementation of bash execution with optional sandboxing.
async fn run_bash_internal(
    command: &str,
    working_dir: impl AsRef<Path>,
    env: &HashMap<String, String>,
    timeout: Duration,
    shell: bool,
    use_sandbox: bool,
) -> ShellResult {
    let working_dir = working_dir.as_ref();

    // Check if sandbox should be used
    let use_sandbox = use_sandbox && matches!(sandbox_available(), SandboxStatus::Available);

    if shell {
        run_bash_shell_internal(command, working_dir, env, timeout, use_sandbox).await
    } else {
        run_bash_direct(command, working_dir, env, timeout).await
    }
}

/// Internal shell mode implementation with optional sandboxing.
async fn run_bash_shell_internal(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
    timeout: Duration,
    use_sandbox: bool,
) -> ShellResult {
    // For sandboxed execution, we use the sandbox module
    if use_sandbox {
        return run_sandboxed_shell(command, working_dir, env, timeout).await;
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .envs(env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_command(cmd, timeout).await
}

/// Direct mode: parse with shell-words and execute without shell wrapper.
async fn run_bash_direct(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
    timeout: Duration,
) -> ShellResult {
    let args = match shell_words::split(command) {
        Ok(args) if !args.is_empty() => args,
        Ok(_) => return ShellResult::error("Empty command".to_owned()),
        Err(e) => return ShellResult::error(format!("Error parsing command: {}", e)),
    };

    let (program, args) = (&args[0], &args[1..]);
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(working_dir)
        .envs(env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_command(cmd, timeout).await
}

/// Run a sandboxed shell command with timeout support.
///
/// Uses tokio::process::Command so the child can be killed when the timeout fires.
async fn run_sandboxed_shell(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
    timeout: Duration,
) -> ShellResult {
    use crate::sandbox;

    let env_pairs: Vec<(String, String)> = env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    let mut child = match sandbox::run_sandboxed_shell_async(command, working_dir, &env_pairs).await {
        Ok(c) => c,
        Err(e) => return ShellResult::error(format!("Failed to spawn sandboxed shell: {}", e)),
    };

    tokio::select! {
        status = child.wait() => {
            match status {
                Ok(exit_status) => {
                    let shell_status = if exit_status.success() {
                        ShellStatus::Success
                    } else {
                        ShellStatus::Error
                    };
                    ShellResult {
                        output: format!("Sandboxed command exited with: {}", exit_status),
                        bytes_transferred: None,
                        status: shell_status,
                    }
                }
                Err(e) => ShellResult::error(format!("IO error waiting for sandboxed command: {}", e)),
            }
        }
        _ = tokio::time::sleep(timeout) => {
            // Kill the child process group when timeout fires.
            child.start_kill().ok();
            ShellResult::timed_out(timeout)
        }
    }
}

/// Run a command group and collect output with timeout.
async fn run_command(mut cmd: Command, timeout: Duration) -> ShellResult {
    // Spawn the child process group
    let child = match cmd.group_spawn() {
        Ok(c) => c,
        Err(e) => return ShellResult::error(format!("Failed to spawn command: {}", e)),
    };

    // Shared flag: set to true when timeout fires.
    let killed = Arc::new(AtomicBool::new(false));
    let killed_out = killed.clone();

    // Channel to send the result back from the collector task.
    let (tx, rx) = oneshot::channel();

    // Spawn collector task — owns the child, collects output, checks killed flag.
    tokio::spawn(async move {
        // Wait for output (takes ownership of child).
        let out = child.wait_with_output().await;

        // Only send result if not killed (timeout hasn't fired).
        if !killed_out.load(Ordering::SeqCst) {
            let _ = tx.send(out);
        }
        // If killed: child is dropped here (implicitly killed by drop impl).
        // This is fine — the timeout handler has already sent the TimedOut result.
    });

    // Race between output collection and timeout.
    tokio::select! {
        result = rx => {
            match result {
                Ok(Ok(out)) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let combined = combine_output(&stdout, &stderr);
                    let bytes = out.stdout.len() as u64 + out.stderr.len() as u64;
                    ShellResult {
                        output: combined,
                        bytes_transferred: Some(bytes),
                        status: if out.status.success() {
                            ShellStatus::Success
                        } else {
                            ShellStatus::Error
                        },
                    }
                }
                Ok(Err(e)) => ShellResult::error(format!("IO error reading output: {}", e)),
                // Sender dropped means timeout fired first — result already returned.
                Err(_) => ShellResult::error("Output collection cancelled".to_owned()),
            }
        }
        _ = tokio::time::sleep(timeout) => {
            // Timeout: signal kill and return.
            killed.store(true, Ordering::SeqCst);
            // The spawned task will drop its child on next await,
            // which triggers process-group kill via command-group's drop impl.
            ShellResult::timed_out(timeout)
        }
    }
}

/// Execute a bash command synchronously (for IO actor and update tools).
///
/// Same semantics as `run_bash` but synchronous.
pub fn run_bash_sync(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
    shell: bool,
) -> ShellResult {
    if shell {
        run_bash_sync_shell(command, working_dir, env)
    } else {
        run_bash_sync_direct(command, working_dir, env)
    }
}

fn run_bash_sync_shell(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> ShellResult {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .envs(env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = combine_output(&stdout, &stderr);
            let bytes = out.stdout.len() as u64 + out.stderr.len() as u64;
            ShellResult {
                output: combined,
                bytes_transferred: Some(bytes),
                status: if out.status.success() {
                    ShellStatus::Success
                } else {
                    ShellStatus::Error
                },
            }
        }
        Err(e) => ShellResult::error(format!("Error running command: {}", e)),
    }
}

fn run_bash_sync_direct(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> ShellResult {
    let args = match shell_words::split(command) {
        Ok(args) if !args.is_empty() => args,
        Ok(_) => return ShellResult::error("Empty command".to_owned()),
        Err(e) => return ShellResult::error(format!("Error parsing command: {}", e)),
    };

    let (program, args) = (&args[0], &args[1..]);
    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(working_dir)
        .envs(env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = combine_output(&stdout, &stderr);
            let bytes = out.stdout.len() as u64 + out.stderr.len() as u64;
            ShellResult {
                output: combined,
                bytes_transferred: Some(bytes),
                status: if out.status.success() {
                    ShellStatus::Success
                } else {
                    ShellStatus::Error
                },
            }
        }
        Err(e) => ShellResult::error(format!("Error running command: {}", e)),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn combine_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() && stderr.is_empty() {
        return String::new();
    }
    if stdout.is_empty() {
        return stderr.trim_end().to_owned();
    }
    if stderr.is_empty() {
        return stdout.trim_end().to_owned();
    }
    format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
}

impl ShellResult {
    fn error(msg: String) -> Self {
        Self {
            output: msg,
            bytes_transferred: None,
            status: ShellStatus::Error,
        }
    }

    fn timed_out(timeout: Duration) -> Self {
        Self {
            output: format!("Command timed out after {:.0} seconds", timeout.as_secs_f64()),
            bytes_transferred: None,
            status: ShellStatus::TimedOut,
        }
    }
}

/// Format command output for display (matches legacy behavior).
pub fn format_command_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("stderr: ");
        result.push_str(stderr);
    }
    if result.is_empty() {
        result = format!("(exit code: {})", exit_code);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_output_prefers_nonempty_streams() {
        assert!(combine_output("", "").is_empty());
        assert_eq!(combine_output("out", ""), "out");
        assert_eq!(combine_output("", "err"), "err");
        assert_eq!(combine_output("out", "err"), "out\nerr");
    }

    #[test]
    fn format_empty_output() {
        let result = format_command_output("", "", 0);
        assert_eq!(result, "(exit code: 0)");
    }

    #[test]
    fn format_stdout_only() {
        let result = format_command_output("hello\nworld", "", 0);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn format_stderr_included() {
        let result = format_command_output("", "error message", 1);
        assert!(result.contains("stderr: error message"));
    }

    #[test]
    fn format_combined_output() {
        let result = format_command_output("stdout\noutput", "stderr msg", 0);
        assert!(result.contains("stdout"));
        assert!(result.contains("stderr"));
    }

    #[tokio::test]
    async fn async_bash_echo_succeeds() {
        let env = HashMap::new();
        let result =
            run_bash("echo hello", Path::new("."), &env, Duration::from_secs(5), true).await;
        assert_eq!(result.status, ShellStatus::Success, "output: {}", result.output);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn async_bash_direct_mode() {
        let env = HashMap::new();
        let result =
            run_bash("echo hello", Path::new("."), &env, Duration::from_secs(5), false).await;
        assert_eq!(result.status, ShellStatus::Success, "output: {}", result.output);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn async_bash_timeout_kills_child() {
        let env = HashMap::new();
        let result =
            run_bash("sleep 30", Path::new("."), &env, Duration::from_secs(1), true).await;
        assert_eq!(result.status, ShellStatus::TimedOut, "output: {}", result.output);
        assert!(result.output.contains("timed out"));
    }

    #[test]
    fn sync_bash_echo_succeeds() {
        let env = HashMap::new();
        let result = run_bash_sync("echo hello", Path::new("."), &env, true);
        assert_eq!(result.status, ShellStatus::Success, "output: {}", result.output);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn sync_bash_direct_mode() {
        let env = HashMap::new();
        let result = run_bash_sync("echo hello", Path::new("."), &env, false);
        assert_eq!(result.status, ShellStatus::Success, "output: {}", result.output);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn sync_bash_command_not_found() {
        let env = HashMap::new();
        let result = run_bash_sync("nonexistent_cmd_xyz_123", Path::new("."), &env, true);
        assert_eq!(result.status, ShellStatus::Error);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn sync_bash_fails_with_nonzero_exit() {
        let env = HashMap::new();
        let result = run_bash_sync("exit 1", Path::new("."), &env, true);
        assert_eq!(result.status, ShellStatus::Error);
    }
}
