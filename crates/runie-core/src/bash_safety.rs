//! Bash command safety checks.
//!
//! Uses `shell-words` for proper shell tokenization, then applies a static
//! deny-list of known destructive command patterns.

use shell_words;

/// Check whether a bash command is unsafe to run automatically.
///
/// Returns `Some(reason)` when the command matches a known destructive
/// pattern; returns `None` when no pattern matched.
pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let tokens = match shell_words::split(command) {
        Ok(t) => t,
        Err(_) => return None,
    };
    check_destructive_tokens(&tokens)
}

/// Check tokens for destructive patterns.
fn check_destructive_tokens(tokens: &[String]) -> Option<&'static str> {
    // Look for dangerous commands anywhere in the line (handles &&, ; etc).
    if let Some(idx) = tokens.iter().position(|t| t.to_lowercase() == "rm") {
        if let Some(reason) = check_rm_at_idx(tokens, idx) {
            return Some(reason);
        }
    }
    if let Some(idx) = tokens.iter().position(|t| t.to_lowercase() == "dd") {
        if let Some(reason) = check_dd_at_idx(tokens, idx) {
            return Some(reason);
        }
    }
    if let Some(idx) = tokens.iter().position(|t| t.to_lowercase() == "chmod") {
        if let Some(reason) = check_chmod_at_idx(tokens, idx) {
            return Some(reason);
        }
    }

    // Generic checks for commands that don't need positional analysis.
    check_generic_dangerous(tokens)
}

fn check_rm_at_idx(tokens: &[String], idx: usize) -> Option<&'static str> {
    let has_rf = tokens[idx..]
        .iter()
        .any(|t| t == "-rf" || t == "-fr" || t == "-r" || t == "-R");
    if !has_rf {
        return None;
    }
    for token in &tokens[idx..] {
        if is_blocked_rm_target(token) {
            return Some("rm -rf on system/home directories is blocked");
        }
    }
    None
}

fn check_dd_at_idx(tokens: &[String], idx: usize) -> Option<&'static str> {
    for token in &tokens[idx..] {
        if token.starts_with("of=/dev/") {
            return Some("dd writing to block devices is blocked");
        }
    }
    None
}

fn check_chmod_at_idx(tokens: &[String], idx: usize) -> Option<&'static str> {
    let has_recursive = tokens[idx..].iter().any(|t| t == "-r" || t == "-R");
    if !has_recursive {
        return None;
    }
    for (i, token) in tokens.iter().enumerate().skip(idx + 1) {
        if matches!(token.as_str(), "777" | "000") && i + 1 < tokens.len() {
            let next = &tokens[i + 1];
            if next.starts_with('/') {
                return Some("recursive chmod on root is blocked");
            }
        }
    }
    None
}

fn check_generic_dangerous(tokens: &[String]) -> Option<&'static str> {
    // Block dangerous commands anywhere in line.
    if let Some(reason) = check_block_tools(tokens) {
        return Some(reason);
    }
    if let Some(reason) = check_interpreter_attack(tokens) {
        return Some(reason);
    }
    if let Some(reason) = check_fork_bomb(tokens) {
        return Some(reason);
    }
    if has_device_redirect(tokens) {
        return Some("writing directly to block devices is blocked");
    }
    check_find_exec_rm(tokens)
}

fn check_block_tools(tokens: &[String]) -> Option<&'static str> {
    for token in tokens {
        let t = token.to_lowercase();
        if t.starts_with("mkfs") {
            return Some("mkfs is blocked");
        }
        if t == "shred" && tokens.iter().any(|t| t.starts_with("/dev/")) {
            return Some("shred on block devices is blocked");
        }
        if matches!(t.as_str(), "fdisk" | "sfdisk" | "parted") {
            return Some("partitioning tools are blocked");
        }
    }
    None
}

fn check_interpreter_attack(tokens: &[String]) -> Option<&'static str> {
    let interpreters = ["python", "python3", "ruby", "node", "perl", "bash", "sh"];
    if let Some(idx) = tokens.iter().position(|t| {
        let lower = t.to_lowercase();
        interpreters.iter().any(|i| lower == *i)
    }) {
        let joined: String = tokens
            .iter()
            .skip(idx)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if contains_destructive(&joined) {
            return Some("interpreter executing destructive code is blocked");
        }
    }
    None
}

fn check_fork_bomb(tokens: &[String]) -> Option<&'static str> {
    let joined: String = tokens.join("");
    if joined.contains(":|:") && (joined.contains("};") || joined.contains(":|&")) {
        return Some("fork bombs are blocked");
    }
    None
}

fn check_find_exec_rm(tokens: &[String]) -> Option<&'static str> {
    if let Some(idx) = tokens.iter().position(|t| t.to_lowercase() == "find") {
        let rest = &tokens[idx..];
        if rest.iter().any(|t| t.contains("-exec")) && rest.iter().any(|t| t.contains("rm")) {
            return Some("find with rm exec is blocked");
        }
    }
    None
}

fn has_device_redirect(tokens: &[String]) -> bool {
    tokens.iter().any(|t| {
        t.starts_with("> /dev/sd")
            || t.starts_with("> /dev/nvme")
            || t.starts_with("> /dev/hd")
            || t.starts_with("> /dev/vd")
            || t.starts_with("> /dev/mmc")
    })
}

fn is_blocked_rm_target(token: &str) -> bool {
    let lower = token.to_lowercase();
    matches!(
        token,
        "~" | "/" | "/boot" | "/etc" | "/home" | "/root" | "/usr" | "/var" | "/dev"
    ) || token.starts_with("/dev/")
        || token.starts_with("~$")
        || token == "--no-preserve-root"
        || token == "--no-preserve-root=all"
        || token.contains('*')
        || token.contains('?')
        || lower == "$home"
        || lower.starts_with("~${")
        || lower.starts_with("$home")
}

fn contains_destructive(text: &str) -> bool {
    text.contains("rm -rf")
        || text.contains("rm ")
        || text.contains(" dd ")
        || text.contains("mkfs")
        || text.contains("shred")
        || text.contains("fdisk")
        || text.contains("sfdisk")
        || text.contains("parted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_direct_destructive_commands() {
        assert!(check_bash_safety("rm -rf /").is_some());
        assert!(check_bash_safety("rm -rf /*").is_some());
        assert!(check_bash_safety("rm -rf ~").is_some());
        assert!(check_bash_safety("dd if=/dev/zero of=/dev/sda").is_some());
        assert!(check_bash_safety("mkfs.ext4 /dev/sda1").is_some());
        assert!(check_bash_safety(":(){ :|:& };:").is_some());
        assert!(check_bash_safety("chmod -R 777 /").is_some());
        assert!(check_bash_safety("sudo rm -rf / important").is_some());
    }

    #[test]
    fn blocks_evasive_variants() {
        assert!(check_bash_safety("rm -rf / --no-preserve-root").is_some());
        assert!(check_bash_safety("cd / && rm -rf *").is_some());
        assert!(check_bash_safety("rm -rf \"$HOME\"").is_some());
        assert!(check_bash_safety("shred -n1 /dev/sda").is_some());
        assert!(check_bash_safety("python -c 'import os; os.system(\"rm -rf /\")'").is_some());
        assert!(check_bash_safety("find / -name '*.tmp' -exec rm {} \\;").is_some());
    }

    #[test]
    fn allows_safe_commands() {
        assert!(check_bash_safety("echo hello").is_none());
        assert!(check_bash_safety("ls -la").is_none());
        assert!(check_bash_safety("cat file.txt").is_none());
        assert!(check_bash_safety("git status").is_none());
        assert!(check_bash_safety("rm -rf build/").is_none());
    }

    #[test]
    fn quoted_arguments_parsed() {
        assert!(check_bash_safety("echo \"hello world\"").is_none());
        assert!(check_bash_safety("ls 'my documents'").is_none());
    }
}
