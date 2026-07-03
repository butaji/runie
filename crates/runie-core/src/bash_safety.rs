//! Bash command safety checks.
//!
//! Uses `shell-words` for proper shell tokenization, then applies a small
//! regex deny-list of known destructive patterns. Handles interpreter
//! bypasses (e.g. `bash -c 'rm -rf /'`) by checking the full joined string.

use shell_words;

/// Result of a safety check — `None` means safe, `Some(&str)` is the reason.
pub type SafetyResult = Option<&'static str>;

/// Check whether a bash command is unsafe to run automatically.
///
/// Returns `Some(reason)` when the command matches a known destructive
/// pattern; returns `None` when no pattern matched.
///
/// # Bypass resistance
/// Commands passed through interpreters (`bash -c`, `python -c`, etc.) are
/// caught by checking the full command string, not just the parsed tokens.
pub fn check_bash_safety(command: &str) -> SafetyResult {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Check the full joined command for patterns that may appear inside
    // interpreter strings (e.g. bash -c 'exec rm -rf /').
    let joined = match shell_words::split(trimmed) {
        Ok(tokens) => tokens.join(" "),
        Err(_) => return None,
    };

    DENY_LIST_CHECKS
        .iter()
        .find(|check| (check.pattern)(&joined))
        .map(|check| check.reason)
}

// ── Deny-list ────────────────────────────────────────────────────────────────

type CheckFn = fn(&str) -> bool;

struct DenyEntry {
    pattern: CheckFn,
    reason: &'static str,
}

/// Check whether `text` contains a recursive rm on a system path.
fn has_recursive_rm(text: &str) -> bool {
    let has_rm_flag = text.contains(" -rf ")
        || text.contains(" -fr ")
        || text.contains(" -r ")
        || text.contains(" -R ")
        || text.ends_with(" -rf")
        || text.ends_with(" -fr")
        || text.ends_with(" -r")
        || text.ends_with(" -R");
    if !has_rm_flag {
        return false;
    }
    let lower = text.to_lowercase();
    // Block rm -rf on system/home directories or glob patterns.
    let blocked = [
        " /", "/boot", "/dev", "/etc", "/home", "/lib", "/opt", "/proc", "/root", "/run", "/sbin",
        "/sys", "/tmp", "/usr", "/var", " ~", "/~", "\"~", "'~", // home directory variants
    ];
    blocked.iter().any(|p| lower.contains(p)) || lower.contains("$home") || lower.contains("${home")
}

/// Check whether `text` writes to a block device.
///
/// Note: `shell_words` inserts spaces around `>`, so we check ` > /dev/` as well
/// as `>/dev/` (direct mode).
fn has_block_device_write(text: &str) -> bool {
    // Direct mode patterns (no space around >)
    text.contains(">/dev/sd")
        || text.contains(">/dev/nvme")
        || text.contains(">/dev/hd")
        || text.contains(">/dev/vd")
        || text.contains(">/dev/mmc")
        || text.contains("of=/dev/sd")
        || text.contains("of=/dev/nvme")
        || text.contains("if=/dev/zero")
        || text.contains("if=/dev/urandom")
        // Tokenized mode (shell_words inserts spaces around >)
        || text.contains(" > /dev/sd")
        || text.contains(" > /dev/nvme")
        || text.contains(" > /dev/hd")
        || text.contains(" > /dev/vd")
        || text.contains(" > /dev/mmc")
}

/// Check whether `text` escalates permissions.
fn has_permission_escalation(text: &str) -> bool {
    // chmod 777/000 on root paths
    let chmod_on_root = text.contains("chmod")
        && text.contains(" 777 ")
        && (text.contains(" /") || text.contains("/root"));
    // sudo without path restriction
    let sudo_nopasswd = text.contains("sudo") && text.contains("-n ") && text.contains("rm");
    chmod_on_root || sudo_nopasswd
}

/// Check whether `text` contains a fork bomb pattern.
fn has_fork_bomb(text: &str) -> bool {
    // :(){:|:&};: and variants
    let t = text.replace(' ', "");
    (t.contains(":(){:|:") && (t.contains("};") || t.contains("};&")))
        || t.contains(":(){ :|: & };:")
}

/// Check whether `text` contains partition/filesystem tools.
fn has_partition_tools(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("mkfs")
        || lower.starts_with("dd ")
        || lower.starts_with("fdisk ")
        || lower.starts_with("sfdisk ")
        || lower.starts_with("parted ")
        || lower.contains("shred ") && lower.contains("/dev/")
}

/// Check whether `text` contains a find + exec + rm pattern.
fn has_find_exec_rm(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("find ")
        && (lower.contains("-exec ") || lower.contains("-exec {}"))
        && (lower.contains(" rm ") || lower.ends_with("rm"))
}

const DENY_LIST_CHECKS: &[DenyEntry] = &[
    DenyEntry {
        pattern: has_recursive_rm,
        reason: "recursive rm on system/home directory is blocked",
    },
    DenyEntry {
        pattern: has_block_device_write,
        reason: "writing to block devices is blocked",
    },
    DenyEntry {
        pattern: has_permission_escalation,
        reason: "permission escalation pattern is blocked",
    },
    DenyEntry {
        pattern: has_fork_bomb,
        reason: "fork bombs are blocked",
    },
    DenyEntry {
        pattern: has_partition_tools,
        reason: "partition/filesystem tools are blocked",
    },
    DenyEntry {
        pattern: has_find_exec_rm,
        reason: "find with rm exec is blocked",
    },
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Directly destructive ────────────────────────────────────────────────

    #[test]
    fn blocks_direct_destructive_commands() {
        assert!(check_bash_safety("rm -rf /").is_some());
        assert!(check_bash_safety("rm -rf /*").is_some());
        assert!(check_bash_safety("rm -rf ~").is_some());
        assert!(check_bash_safety("rm -rf /tmp").is_some());
        assert!(check_bash_safety("dd if=/dev/zero of=/dev/sda").is_some());
        assert!(check_bash_safety("mkfs.ext4 /dev/sda1").is_some());
        assert!(check_bash_safety(":(){ :|:& };:").is_some());
        assert!(check_bash_safety("sudo rm -rf /").is_some());
        assert!(check_bash_safety("sudo -n rm -rf /home").is_some());
    }

    #[test]
    fn blocks_recursive_chmod() {
        assert!(check_bash_safety("chmod -R 777 /").is_some());
        assert!(check_bash_safety("chmod -R 000 /etc").is_some());
        assert!(check_bash_safety("chmod 777 /root").is_some());
    }

    // ── Evasive variants ───────────────────────────────────────────────────

    #[test]
    fn blocks_evasive_variants() {
        // BASH-253: Interpreter bypass — bash -c embeds the destructive pattern
        // in a string argument. The joined tokens preserve the pattern, so
        // has_recursive_rm finds it.
        assert!(check_bash_safety("bash -c 'exec rm -rf /'").is_some());
        assert!(check_bash_safety("bash -c \"exec rm -rf /\"").is_some());
        assert!(check_bash_safety("bash -c 'rm -rf / --no-preserve-root'").is_some());
        // Quoted paths
        assert!(check_bash_safety("rm -rf \"$HOME\"").is_some());
        assert!(check_bash_safety("rm -rf '$HOME'").is_some());
        assert!(check_bash_safety("cd / && rm -rf *").is_some());
        // Interpreter via python/ruby/perl
        assert!(check_bash_safety("python -c 'import os; os.system(\"rm -rf /\")'").is_some());
        assert!(check_bash_safety("ruby -e 'system(\"rm -rf /\")'").is_some());
        assert!(check_bash_safety("perl -e 'system(\"rm -rf /\")'").is_some());
        assert!(
            check_bash_safety("node -e \"require('child_process').execSync('rm -rf /')\"")
                .is_some()
        );
        // Shred / find exec rm
        assert!(check_bash_safety("shred -n1 /dev/sda").is_some());
        assert!(check_bash_safety("find / -name '*.tmp' -exec rm {} \\;").is_some());
        // Device writes
        assert!(check_bash_safety("dd if=/dev/zero of=/dev/sda bs=1M").is_some());
        assert!(check_bash_safety("cat /dev/zero > /dev/sda").is_some());
    }

    // ── Safe commands ──────────────────────────────────────────────────────

    #[test]
    fn allows_safe_commands() {
        assert!(check_bash_safety("echo hello").is_none());
        assert!(check_bash_safety("ls -la").is_none());
        assert!(check_bash_safety("cat file.txt").is_none());
        assert!(check_bash_safety("git status").is_none());
        assert!(check_bash_safety("rm -rf build/").is_none());
        assert!(check_bash_safety("cargo build --release").is_none());
        assert!(check_bash_safety("npm install").is_none());
        assert!(check_bash_safety("python script.py").is_none());
        assert!(check_bash_safety("echo 'hello world'").is_none());
        assert!(check_bash_safety("ls 'my documents'").is_none());
    }

    #[test]
    fn allows_nested_interpreter_safe_commands() {
        // Safe commands inside interpreter strings are fine.
        assert!(check_bash_safety("bash -c 'echo hello'").is_none());
        assert!(check_bash_safety("python -c 'print(1+1)'").is_none());
        assert!(check_bash_safety("bash -c 'ls -la'").is_none());
    }

    #[test]
    fn empty_command_is_safe() {
        assert!(check_bash_safety("").is_none());
        assert!(check_bash_safety("   ").is_none());
    }
}
