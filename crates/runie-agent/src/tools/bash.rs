//! Bash execution with timeout and truncation.

use std::process::Command;
use std::time::Duration;

use crate::accumulator::{OutputAccumulator, TruncateStrategy};
use crate::safety::check_bash_safety;
use crate::truncate::TruncationPolicy;

use super::ShellOutput;

/// Default timeout for bash commands in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Run bash and return structured output.
pub fn run_bash(command: &str, policy: &TruncationPolicy) -> ShellOutput {
    if let Some(reason) = check_safety(command) {
        return reason;
    }

    let output = match run_command_with_timeout(
        "bash".to_string(),
        vec!["-c".to_string(), command.to_string()],
        Duration::from_secs(DEFAULT_TIMEOUT_SECS),
    ) {
        Ok(output) => output,
        Err(e) => return handle_exec_error(e),
    };

    run_bash_output(&output.stdout, &output.stderr, output.status.code(), policy)
}

/// Run bash and return the rendered output string and success flag.
/// Used by exec.rs for callers that expect (String, bool).
pub fn run_bash_legacy(command: &str, policy: &TruncationPolicy) -> (String, bool) {
    let out = run_bash(command, policy);
    (out.render(), out.is_success())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn check_safety(command: &str) -> Option<ShellOutput> {
    check_bash_safety(command).map(|reason| ShellOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        timed_out: false,
        truncated: false,
        full_output_path: None,
        rendered: format!("Blocked: {}", reason),
        blocked: Some(reason.to_owned()),
    })
}

fn handle_exec_error(e: std::io::Error) -> ShellOutput {
    let timed_out = e.kind() == std::io::ErrorKind::TimedOut;
    ShellOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        timed_out,
        truncated: false,
        full_output_path: None,
        rendered: if timed_out {
            "[Command timed out]".to_string()
        } else {
            format!("Error: {}", e)
        },
        blocked: if timed_out { None } else { Some(e.to_string()) },
    }
}

fn run_bash_output(
    raw_stdout: &[u8],
    raw_stderr: &[u8],
    exit_code: Option<i32>,
    policy: &TruncationPolicy,
) -> ShellOutput {
    let stdout = String::from_utf8_lossy(raw_stdout).to_string();
    let stderr = String::from_utf8_lossy(raw_stderr).to_string();

    let combined = combine_output(&stdout, &stderr);
    let (is_truncated, truncated_content) = truncate_combined(&combined, policy);
    let full_output_path = save_temp_if_truncated(&combined, is_truncated);

    let rendered = super::build_rendered(
        stdout.clone(),
        stderr.clone(),
        exit_code,
        false,
        is_truncated,
        full_output_path.as_ref(),
        if is_truncated { &truncated_content } else { &combined },
    );

    ShellOutput {
        stdout,
        stderr,
        rendered,
        exit_code,
        timed_out: false,
        truncated: is_truncated,
        full_output_path,
        blocked: None,
    }
}

fn combine_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() {
        return stderr.trim_end().to_string();
    }
    if stderr.is_empty() {
        return stdout.trim_end().to_string();
    }
    format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
}

fn truncate_combined(
    combined: &str,
    policy: &TruncationPolicy,
) -> (bool, String) {
    let mut acc = OutputAccumulator::new(policy, TruncateStrategy::Tail);
    acc.append(combined.as_bytes());
    let snap = acc.snapshot();
    (snap.was_truncated, snap.content)
}

fn save_temp_if_truncated(combined: &str, is_truncated: bool) -> Option<std::path::PathBuf> {
    if !is_truncated {
        return None;
    }
    let dir = std::env::temp_dir();
    let path = dir.join("runie_shell_output.txt");
    std::fs::write(&path, combined).ok()?;
    Some(path)
}

fn run_command_with_timeout(
    program: String,
    args: Vec<String>,
    timeout: Duration,
) -> Result<std::process::Output, std::io::Error> {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || match Command::new(&program).args(&args).output() {
        Ok(output) => {
            let _ = tx.send(Ok(output));
        }
        Err(e) => {
            let _ = tx.send(Err(e));
        }
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Command timed out"))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(std::io::Error::other("Channel disconnected unexpectedly"))
        }
    }
}
