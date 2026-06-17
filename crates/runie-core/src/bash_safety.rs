//! Bash command safety checks.
//!
//! These checks are heuristic — they catch common destructive patterns and
//! obvious evasions, but they are not a sandbox. Destructive commands that
//! cannot be proven safe should require explicit user approval.

/// Check whether a bash command is unsafe to run automatically.
///
/// Returns `Some(reason)` when the command matches a known destructive
/// pattern; returns `None` when no pattern matched.
pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let normalized = normalize_command(command);
    if let Some(reason) = check_fork_bomb(&normalized) {
        return Some(reason);
    }
    for segment in split_segments(&normalized) {
        if let Some(reason) = check_segment(&segment) {
            return Some(reason);
        }
    }
    None
}

fn normalize_command(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                // Drop quotes so "rm -rf /" is treated the same as rm -rf /.
            }
            '$' => {
                // Expand simple variable names to common values for matching.
                let mut name = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' {
                        name.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push_str(&expand_var(&name));
            }
            _ => out.push(c),
        }
    }
    out.to_lowercase()
}

fn expand_var(name: &str) -> &str {
    if name.eq_ignore_ascii_case("home") {
        "~"
    } else {
        "$"
    }
}

fn split_segments(cmd: &str) -> Vec<&str> {
    cmd.split(&[';', '&', '|', '\n'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

fn check_segment(seg: &str) -> Option<&'static str> {
    check_rm_rf(seg)
        .or_else(|| check_dd(seg))
        .or_else(|| check_block_write(seg))
        .or_else(|| check_mkfs(seg))
        .or_else(|| check_fork_bomb(seg))
        .or_else(|| check_chmod_root(seg))
        .or_else(|| check_sudo_destructive(seg))
        .or_else(|| check_shred(seg))
        .or_else(|| check_interpreter_destructive(seg))
        .or_else(|| check_find_exec_rm(seg))
}

fn check_rm_rf(seg: &str) -> Option<&'static str> {
    if !seg.contains("rm") || !seg.contains(" -rf ") && !seg.contains(" -fr ") {
        return None;
    }
    let after = seg.rsplit_once("rm").map(|(_, rest)| rest).unwrap_or(seg);
    if has_system_path(after) || after.contains("--no-preserve-root") {
        Some("rm -rf on system directories or home is blocked")
    } else {
        None
    }
}

fn has_system_path(seg: &str) -> bool {
    if seg.contains(" /") || seg.contains("~") || seg.contains("*") {
        return true;
    }
    let system_dirs = [" /boot", " /etc", " /home", " /root", " /usr", " /var"];
    system_dirs.iter().any(|dir| seg.contains(dir))
}

fn check_dd(seg: &str) -> Option<&'static str> {
    if seg.starts_with("dd ") && seg.contains("of=/dev/") {
        Some("dd writing to block devices is blocked")
    } else {
        None
    }
}

fn check_block_write(seg: &str) -> Option<&'static str> {
    if seg.contains("> /dev/sd")
        || seg.contains("> /dev/nvme")
        || seg.contains("> /dev/hd")
        || seg.contains("> /dev/vd")
        || seg.contains("> /dev/mmc")
    {
        Some("writing directly to block devices is blocked")
    } else {
        None
    }
}

fn check_mkfs(seg: &str) -> Option<&'static str> {
    if seg.starts_with("mkfs") || seg.starts_with("mkfs.") {
        Some("mkfs is blocked")
    } else {
        None
    }
}

fn check_fork_bomb(seg: &str) -> Option<&'static str> {
    if seg.contains(":|:") && (seg.contains("};") || seg.contains(":|&")) {
        Some("fork bombs are blocked")
    } else {
        None
    }
}

fn check_chmod_root(seg: &str) -> Option<&'static str> {
    if seg.contains("chmod -r 777 /") || seg.contains("chmod -r 000 /") {
        Some("recursive chmod on root is blocked")
    } else {
        None
    }
}

fn check_sudo_destructive(seg: &str) -> Option<&'static str> {
    if seg.starts_with("sudo ") && contains_destructive(seg) {
        Some("sudo with destructive commands is blocked")
    } else {
        None
    }
}

fn check_shred(seg: &str) -> Option<&'static str> {
    if seg.starts_with("shred ") && seg.contains("/dev/") {
        Some("shred on block devices is blocked")
    } else {
        None
    }
}

fn check_interpreter_destructive(seg: &str) -> Option<&'static str> {
    let interpreters = ["python", "python3", "ruby", "node", "perl"];
    for interp in interpreters {
        if seg.starts_with(interp) && contains_destructive(seg) {
            return Some("interpreter executing destructive code is blocked");
        }
    }
    None
}

fn check_find_exec_rm(seg: &str) -> Option<&'static str> {
    if seg.starts_with("find ") && seg.contains("-exec") && seg.contains("rm") {
        Some("find with rm exec is blocked")
    } else {
        None
    }
}

fn contains_destructive(seg: &str) -> bool {
    seg.contains(" rm ") || seg.contains(" dd ") || seg.contains(" mkfs") || seg.contains(" shred")
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
}
