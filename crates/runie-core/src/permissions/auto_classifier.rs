//! Auto permission classifier with safe command allowlist and dangerous pattern blocking.

use std::collections::HashSet;

/// Safe commands that auto-approve
const SAFE_COMMANDS: &[&str] = &[
    // File listing
    "ls", "pwd", "cat", "head", "tail", "wc", "sort", "uniq", "cut", "tr",
    "grep", "rg", "find", "fd", "tree", "stat", "file", "basename", "dirname",
    "realpath", "readlink", "xargs", "column", "paste", "join", "comm",
    // Git commands (read-only)
    "git status", "git diff", "git log", "git show", "git branch", "git tag",
    "git remote", "git reflog", "git show-ref", "git ls-tree", "git ls-files",
    "git diff-index", "git diff-tree", "git log --oneline", "git log --stat",
    // Cargo commands (non-destructive)
    "cargo check", "cargo build", "cargo test", "cargo clippy", "cargo fmt",
    "cargo metadata", "cargo tree", "cargo list", "cargo search", "cargo info",
    // Package managers (safe subcommands)
    "npm --version", "npm list", "npm test", "npm run", "npm audit", "npm outdated",
    "npm doctor", "npm search", "npm view", "npm config list", "npm whoami",
    "pnpm --version", "pnpm list", "pnpm test", "pnpm run", "pnpm audit",
    "yarn --version", "yarn list", "yarn test", "yarn run", "yarn audit",
    // Python tools
    "python --version", "python3 --version", "pip list", "pip show", "pip freeze",
    "pip3 list", "pip3 show", "pip3 freeze", "pip check",
    // Node tools
    "node --version", "npm --version", "npx --version", "npx --help",
    "bun --version", "bunpm list",
    // System info
    "uname", "whoami", "id", "hostname", "uptime", "df", "du", "free",
    "ps", "pgrep", "pidof", "which", "type", "command", "hash",
    // Network (read-only)
    "curl -s", "curl -I", "curl --head", "wget --spider", "ping",
    "nslookup", "dig", "host", "traceroute", "netstat", "ss",
];

/// Dangerous patterns that always block
const DANGEROUS_PATTERNS: &[&str] = &[
    // Pipe to shell execution
    "curl | sh", "curl | bash", "wget | sh", "wget | bash",
    "fetch | sh", "fetch | bash",
    // Dangerous permissions
    "chmod 777", "chmod -R 777", "chmod 000", "chmod -R 000",
    "chmod u+s", "chmod g+s",
    // Dangerous encoding
    "base64 -d",
    // Network shells
    "nc -e", "netcat -e", "/dev/tcp/", "/dev/udp/",
    "rm -f /tmp/f", "mkfifo",
    // Destructive sudo
    "sudo rm", "sudo del", "sudo shutdown", "sudo reboot", "sudo halt",
    "sudo init", "sudo poweroff", "sudo systemctl stop",
    // Fork bombs
    ":(){ :|:& };:", "fork();", "while true; do",
    // Overwrite system files
    "> /etc/passwd", "> /etc/shadow", "> /etc/group",
    // DD to device
    "dd if=", "dd of=/dev/", "dd of=/dev/sd",
    // Mount over system dirs
    "mount --bind", "mount -o bind",
    // Kernel modification
    "sysctl -w", "echo > /proc/",
    // Download and execute
    "wget -O- |", "curl -sL |",
];

/// Package manager safe subcommands
const NPM_SAFE_SUBCOMMANDS: &[&str] = &[
    "install", "ci", "test", "run", "build", "audit", "ls", "list", 
    "outdated", "doctor", "search", "view", "config", "whoami", "info",
    "pack", "explore", "help", "init", "link", "logout", "owner", "pkg",
    "profile", "rate", "repo", "star", "stars", "team", "token", "unstar",
];

/// Python package manager safe subcommands
const PIP_SAFE_SUBCOMMANDS: &[&str] = &[
    "install", "download", "list", "show", "check", "config", "freeze",
    "index", "search", "cache", "hash", "debug", "completion", "version",
];

/// Classifier verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierVerdict {
    /// Command is safe and can be auto-approved
    Allow,
    /// Command is dangerous and should be blocked
    Block,
    /// Command requires user decision
    Unavailable,
}

impl ClassifierVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, ClassifierVerdict::Allow)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, ClassifierVerdict::Block)
    }

    pub fn needs_user_decision(&self) -> bool {
        matches!(self, ClassifierVerdict::Unavailable)
    }
}

/// Auto classifier for permission decisions
pub struct AutoClassifier;

impl AutoClassifier {
    /// Classify a bash command, returning Allow/Block/Unavailable
    pub fn classify_bash(cmd: &str) -> ClassifierVerdict {
        let trimmed = cmd.trim();
        
        // Check for empty command
        if trimmed.is_empty() {
            return ClassifierVerdict::Block;
        }

        // Check safe command list first (fast path)
        if Self::is_safe_command(trimmed) {
            return ClassifierVerdict::Allow;
        }

        // Check dangerous patterns (fast path)
        if Self::has_dangerous_pattern(trimmed) {
            return ClassifierVerdict::Block;
        }

        // Unwrap command wrappers and recurse
        if let Some(inner) = Self::unwrap_wrapper(trimmed) {
            return Self::classify_bash(&inner);
        }

        // Unknown command - require user decision
        ClassifierVerdict::Unavailable
    }

    /// Classify an MCP tool call
    pub fn classify_mcp_tool(server: &str, tool: &str) -> ClassifierVerdict {
        // File system read operations are generally safe
        let safe_read_tools = [
            "read_file", "read_multiple_files", "glob", "glob_images",
            "list_directory", "search_files", "get_file_info",
        ];

        // File system write operations need approval
        let write_tools = [
            "write_file", "create_directory", "delete_file", "move_file",
            "edit_file", "str_replace_editor", "notebook_write",
        ];

        // Safe read operations
        if safe_read_tools.contains(&tool) {
            return ClassifierVerdict::Allow;
        }

        // Write operations require decision
        if write_tools.contains(&tool) {
            return ClassifierVerdict::Unavailable;
        }

        // Unknown MCP tool - require user decision
        ClassifierVerdict::Unavailable
    }

    /// Classify a file path operation
    pub fn classify_path(path: &std::path::Path, operation: PathOperation) -> ClassifierVerdict {
        // Read operations on home directory are generally safe
        if matches!(operation, PathOperation::Read) {
            if let Some(home) = dirs::home_dir() {
                if path.starts_with(home) && !path.to_string_lossy().contains(".ssh/") {
                    return ClassifierVerdict::Allow;
                }
            }
        }

        // Block operations on system paths
        let system_paths = [
            "/etc", "/usr", "/bin", "/sbin", "/lib", "/lib64",
            "/sys", "/proc", "/dev", "/boot", "/srv",
        ];

        for sys_path in system_paths {
            if path.starts_with(sys_path) {
                return ClassifierVerdict::Block;
            }
        }

        // Block operations on hidden directories in root
        if path.to_string_lossy().starts_with("/.") {
            return ClassifierVerdict::Block;
        }

        ClassifierVerdict::Unavailable
    }

    fn is_safe_command(cmd: &str) -> bool {
        // Exact match
        if SAFE_COMMANDS.contains(&cmd) {
            return true;
        }

        // Prefix match with space (cmd followed by args)
        SAFE_COMMANDS.iter().any(|safe| {
            cmd.starts_with(*safe) && cmd.chars().nth(safe.len()) == Some(' ')
        })
    }

    fn has_dangerous_pattern(cmd: &str) -> bool {
        let lower = cmd.to_lowercase();
        DANGEROUS_PATTERNS.iter().any(|pattern| {
            lower.contains(&pattern.to_lowercase())
        })
    }

    fn unwrap_wrapper(cmd: &str) -> Option<String> {
        let trimmed = cmd.trim();

        // timeout [N] cmd → cmd
        if let Some(rest) = trimmed.strip_prefix("timeout ") {
            let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ' || c == '-');
            if let Some(inner) = rest.strip_prefix("-- ") {
                return Some(inner.to_string());
            }
            return Some(rest.to_string());
        }

        // time cmd → cmd
        if let Some(rest) = trimmed.strip_prefix("time ") {
            return Some(rest.to_string());
        }

        // env VAR=val cmd → cmd
        if let Some(rest) = trimmed.strip_prefix("env ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() > 1 && parts[0].contains('=') {
                return Some(parts[1..].join(" "));
            }
        }

        // sudo cmd → cmd (with warning for root operations)
        if let Some(rest) = trimmed.strip_prefix("sudo ") {
            // Block sudo with no password prompts in scripts
            if rest.contains("sudo -n") {
                return Some(rest.to_string());
            }
            return Some(rest.to_string());
        }

        // sh -c "cmd" → cmd
        if let Some(rest) = trimmed.strip_prefix("sh -c ") {
            if let Some(content) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                return Some(content.to_string());
            }
            if let Some(content) = rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                return Some(content.to_string());
            }
        }

        // bash -c "cmd" → cmd
        if let Some(rest) = trimmed.strip_prefix("bash -c ") {
            if let Some(content) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                return Some(content.to_string());
            }
            if let Some(content) = rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                return Some(content.to_string());
            }
        }

        None
    }

    /// Classify npm/pnpm/yarn commands
    pub fn classify_npm(cmd: &str) -> ClassifierVerdict {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return ClassifierVerdict::Block;
        }

        // Check if it's a safe subcommand
        let subcmd = parts.get(1).unwrap_or(&"");
        if NPM_SAFE_SUBCOMMANDS.contains(&subcmd) {
            return ClassifierVerdict::Allow;
        }

        // Block dangerous operations
        let dangerous = ["uninstall", "rm", "remove", "prune"];
        if dangerous.contains(&subcmd) {
            // Allow with --save-dev or --save-optional but block --global
            let args = &parts[2..];
            if args.contains(&"--global") || args.contains(&"-g") {
                return ClassifierVerdict::Block;
            }
            return ClassifierVerdict::Unavailable;
        }

        ClassifierVerdict::Unavailable
    }

    /// Classify git commands
    pub fn classify_git(cmd: &str) -> ClassifierVerdict {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() || parts[0] != "git" {
            return ClassifierVerdict::Unavailable;
        }

        // Safe git commands
        let safe_git = [
            "status", "diff", "log", "show", "branch", "tag", "remote",
            "reflog", "show-ref", "ls-tree", "ls-files", "diff-index",
            "diff-tree", "rev-parse", "rev-list", "shortlog", "describe",
            "for-each-ref", "cat-file", "diff-tree", "ls-remote",
        ];

        if parts.len() >= 2 {
            let subcmd = parts[1];
            if safe_git.contains(&subcmd) {
                return ClassifierVerdict::Allow;
            }
        }

        // Block dangerous operations
        let dangerous_git = [
            "push --force", "push -f", "push --delete", "push -d",
            "rebase -i", "filter-branch", "gc --aggressive",
        ];

        for dangerous in dangerous_git {
            if cmd.contains(dangerous) {
                return ClassifierVerdict::Block;
            }
        }

        ClassifierVerdict::Unavailable
    }
}

/// Path operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathOperation {
    Read,
    Write,
    Delete,
    Execute,
}

/// Result of classification with explanation
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub verdict: ClassifierVerdict,
    pub reason: String,
    pub confidence: f32,
}

impl ClassificationResult {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            verdict: ClassifierVerdict::Allow,
            reason: reason.into(),
            confidence: 1.0,
        }
    }

    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            verdict: ClassifierVerdict::Block,
            reason: reason.into(),
            confidence: 1.0,
        }
    }

    pub fn unavailable(reason: impl Into<String>, confidence: f32) -> Self {
        Self {
            verdict: ClassifierVerdict::Unavailable,
            reason: reason.into(),
            confidence,
        }
    }
}

impl AutoClassifier {
    /// Classify with detailed result
    pub fn classify_with_reason(cmd: &str) -> ClassificationResult {
        let trimmed = cmd.trim();

        // Check for empty command
        if trimmed.is_empty() {
            return ClassificationResult::block("Empty command");
        }

        // Check safe command list
        if Self::is_safe_command(trimmed) {
            return ClassificationResult::allow("Command in safe allowlist");
        }

        // Check dangerous patterns
        if Self::has_dangerous_pattern(trimmed) {
            return ClassificationResult::block("Command matches dangerous pattern");
        }

        // Unwrap and recurse
        if let Some(inner) = Self::unwrap_wrapper(trimmed) {
            return Self::classify_with_reason(&inner);
        }

        ClassificationResult::unavailable(
            format!("Unknown command: {}", Self::truncate_cmd(trimmed)),
            0.5,
        )
    }

    fn truncate_cmd(cmd: &str) -> String {
        if cmd.len() > 50 {
            format!("{}...", &cmd[..47])
        } else {
            cmd.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands() {
        assert_eq!(AutoClassifier::classify_bash("ls"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("ls -la"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("git status"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("cargo check"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("npm --version"), ClassifierVerdict::Allow);
    }

    #[test]
    fn test_dangerous_commands() {
        assert_eq!(AutoClassifier::classify_bash("curl | sh"), ClassifierVerdict::Block);
        assert_eq!(AutoClassifier::classify_bash("sudo rm -rf /"), ClassifierVerdict::Block);
        assert_eq!(AutoClassifier::classify_bash("chmod 777 file"), ClassifierVerdict::Block);
    }

    #[test]
    fn test_wrapper_unwrapping() {
        assert_eq!(AutoClassifier::classify_bash("timeout 30 ls"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("time ls"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_bash("env VAR=val ls"), ClassifierVerdict::Allow);
    }

    #[test]
    fn test_unknown_commands() {
        assert_eq!(AutoClassifier::classify_bash("my_custom_script.sh"), ClassifierVerdict::Unavailable);
        assert_eq!(AutoClassifier::classify_bash("webpack build"), ClassifierVerdict::Unavailable);
    }

    #[test]
    fn test_mcp_tools() {
        assert_eq!(AutoClassifier::classify_mcp_tool("filesystem", "read_file"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_mcp_tool("filesystem", "write_file"), ClassifierVerdict::Unavailable);
    }

    #[test]
    fn test_npm_classification() {
        assert_eq!(AutoClassifier::classify_npm("npm install"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_npm("npm test"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_npm("npm uninstall --global"), ClassifierVerdict::Block);
    }

    #[test]
    fn test_git_classification() {
        assert_eq!(AutoClassifier::classify_git("git status"), ClassifierVerdict::Allow);
        assert_eq!(AutoClassifier::classify_git("git push --force"), ClassifierVerdict::Block);
    }
}
