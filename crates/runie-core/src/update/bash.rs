//! Bash command execution for ! prefix

use std::process::{Command, Stdio};

/// Execute a bash command and return output string
pub fn execute_bash(command: &str) -> String {
    let output = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => out,
        Err(e) => return format!("Error running command: {}", e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    format_command_output(&stdout, &stderr, exit_code)
}

/// Format command output for display
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
    fn execute_echo_command() {
        let output = execute_bash("echo hello");
        assert!(output.contains("hello"), "Should contain hello");
    }

    #[test]
    fn execute_pwd_command() {
        let output = execute_bash("pwd");
        assert!(!output.is_empty(), "pwd should return output");
    }

    #[test]
    fn command_not_found() {
        let output = execute_bash("nonexistent_command_xyz");
        assert!(
            output.contains("Error") || output.contains("not found"),
            "Should show error for invalid command"
        );
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
}
