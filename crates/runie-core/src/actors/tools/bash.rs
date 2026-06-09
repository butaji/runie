//! Bash tool implementation

use std::process::Command;
use std::sync::mpsc;
use std::thread;



/// Check if a bash command is safe to execute
pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let cmd = command.trim().to_lowercase();
    if cmd.contains("rm -rf /") || cmd.contains("rm -rf /*") || cmd.contains("rm -rf ~") {
        return Some("rm -rf on system directories or home is blocked");
    }
    if cmd.starts_with("dd ") && cmd.contains("of=/dev/") {
        return Some("dd writing to block devices is blocked");
    }
    if cmd.contains("> /dev/sda") || cmd.contains("> /dev/nvme") || cmd.contains("> /dev/hd") {
        return Some("writing directly to block devices is blocked");
    }
    if cmd.starts_with("mkfs") || cmd.starts_with("mkfs.") {
        return Some("mkfs is blocked");
    }
    if cmd.contains(":|:") && cmd.contains("};") {
        return Some("fork bombs are blocked");
    }
    if cmd.contains("chmod -r 777 /") || cmd.contains("chmod -r 000 /") {
        return Some("recursive chmod on root is blocked");
    }
    if cmd.starts_with("sudo ") && (cmd.contains(" rm ") || cmd.contains(" dd ")) {
        return Some("sudo with destructive commands is blocked");
    }
    None
}

/// Run bash command with timeout
pub fn run_bash_with_timeout(command: &str, timeout: std::time::Duration) -> std::io::Result<std::process::Output> {
    let (tx, rx) = mpsc::channel();
    let cmd = command.to_string();
    thread::spawn(move || {
        match Command::new("bash").arg("-c").arg(&cmd).output() {
            Ok(output) => { let _ = tx.send(Ok(output)); }
            Err(e) => { let _ = tx.send(Err(e)); }
        }
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Command timed out after {:?}", timeout)
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(std::io::Error::other("Channel disconnected unexpectedly"))
        }
    }
}

/// Execute bash tool
pub fn execute_bash(params: &serde_json::Value) -> super::ToolOutput {
    let command = get_str(params, "command");
    let timeout_secs = get_usize(params, "timeout", 60);

    if let Some(reason) = check_bash_safety(&command) {
        return super::ToolOutput {
            success: false,
            output: format!("Blocked: {}", reason),
        };
    }

    let output = run_bash_with_timeout(&command, std::time::Duration::from_secs(timeout_secs as u64));

    match output {
        Ok(out) => parse_bash_output(out),
        Err(e) => super::ToolOutput {
            success: false,
            output: format!("Error executing '{}': {}", command, e),
        },
    }
}

fn parse_bash_output(out: std::process::Output) -> super::ToolOutput {
    let mut result = String::new();
    if !out.stdout.is_empty() {
        result.push_str(&String::from_utf8_lossy(&out.stdout));
    }
    if !out.stderr.is_empty() {
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    let success = out.status.success();
    if result.is_empty() {
        result = if success { "(no output)".to_string() } else { "(command failed)".to_string() };
    }
    super::ToolOutput { success, output: result }
}

fn get_str(params: &serde_json::Value, key: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn get_usize(params: &serde_json::Value, key: &str, default: usize) -> usize {
    params.get(key).and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_allows_simple_commands() {
        assert!(check_bash_safety("echo hello").is_none());
        assert!(check_bash_safety("ls -la").is_none());
    }

    #[test]
    fn safety_blocks_rm_rf_root() {
        assert!(check_bash_safety("rm -rf /").is_some());
        assert!(check_bash_safety("rm -rf /*").is_some());
    }

    #[test]
    fn bash_tool_echo() {
        let result = execute_bash(&serde_json::json!({"command": "echo hello"}));
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn bash_tool_blocked() {
        let result = execute_bash(&serde_json::json!({"command": "rm -rf /"}));
        assert!(!result.success);
        assert!(result.output.contains("Blocked"));
    }

    #[test]
    fn bash_timeout() {
        let result = run_bash_with_timeout("sleep 10", std::time::Duration::from_millis(100));
        assert!(result.is_err());
        assert!(result.unwrap_err().kind() == std::io::ErrorKind::TimedOut);
    }
}
