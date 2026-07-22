//! Sensitive path blocklist for security-critical files and directories.
//!
//! Paths matching these patterns are always denied or require explicit approval,
//! regardless of permission mode.

use async_trait::async_trait;

use super::{PermissionContext, PermissionPolicy, PermissionResult};

/// Patterns for sensitive paths that should be blocked or require approval.
pub const SENSITIVE_PATH_PATTERNS: &[&str] = &[
    "**/.ssh/*",
    "**/id_rsa",
    "**/id_ed25519",
    "**/.aws/credentials",
    "**/.aws/config",
    "**/.azure/*",
    "**/.kube/config",
    "**/.docker/config.json",
    "**/docker/config.json",
    "**/.git/objects/**",
    "**/.git/objects",
];

/// Returns true if the path matches any sensitive path pattern.
pub fn is_sensitive_path(path: &str) -> bool {
    SENSITIVE_PATH_PATTERNS
        .iter()
        .any(|p| glob_matches(p, path))
}

/// Match a glob pattern against a string using the `glob` crate.
fn glob_matches(pattern: &str, name: &str) -> bool {
    glob::Pattern::new(pattern)
        .map(|p| p.matches(name))
        .unwrap_or(false)
}

/// Policy that blocks or asks for access to sensitive paths.
#[derive(Debug, Default, Clone, Copy)]
pub struct SensitivePathBlocklist {
    /// If true, always denies. If false, returns Ask (requires confirmation).
    deny: bool,
}

impl SensitivePathBlocklist {
    /// Create a blocklist policy that denies sensitive paths.
    pub fn new() -> Self {
        Self { deny: true }
    }

    /// Create a blocklist policy that asks for sensitive paths (doesn't auto-deny).
    pub fn ask() -> Self {
        Self { deny: false }
    }
}

#[async_trait]
impl PermissionPolicy for SensitivePathBlocklist {
    fn name(&self) -> &str {
        "sensitive_path_blocklist"
    }

    fn matches(&self, ctx: &PermissionContext<'_>) -> bool {
        ctx.path.is_some()
    }

    async fn evaluate(&self, ctx: &PermissionContext<'_>) -> Option<PermissionResult> {
        if let Some(path) = ctx.path {
            if is_sensitive_path(&path.to_string_lossy()) {
                return if self.deny {
                    Some(PermissionResult::Deny)
                } else {
                    Some(PermissionResult::Ask)
                };
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_path_ssh() {
        assert!(is_sensitive_path("/home/user/.ssh/id_rsa"));
        assert!(is_sensitive_path("/home/user/.ssh/id_ed25519"));
        assert!(is_sensitive_path("/project/.ssh/config"));
        assert!(is_sensitive_path(".ssh/authorized_keys"));
        assert!(is_sensitive_path("**/.ssh/*"));
    }

    #[test]
    fn sensitive_path_id_keys() {
        assert!(is_sensitive_path("/home/user/id_rsa"));
        assert!(is_sensitive_path("/home/user/id_ed25519"));
        assert!(is_sensitive_path("/etc/ssh/id_rsa"));
    }

    #[test]
    fn sensitive_path_aws() {
        assert!(is_sensitive_path("/home/user/.aws/credentials"));
        assert!(is_sensitive_path("/project/.aws/config"));
    }

    #[test]
    fn sensitive_path_azure() {
        assert!(is_sensitive_path("/home/user/.azure/config"));
        assert!(is_sensitive_path("/project/.azure/auth"));
    }

    #[test]
    fn sensitive_path_kubeconfig() {
        assert!(is_sensitive_path("/home/user/.kube/config"));
    }

    #[test]
    fn sensitive_path_docker() {
        assert!(is_sensitive_path("/home/user/.docker/config.json"));
        assert!(is_sensitive_path("/var/lib/docker/config.json"));
    }

    #[test]
    fn sensitive_path_git_objects() {
        assert!(is_sensitive_path("/project/.git/objects"));
        assert!(is_sensitive_path("/project/.git/objects/pack"));
    }

    #[test]
    fn non_sensitive_path() {
        assert!(!is_sensitive_path("/project/src/main.rs"));
        assert!(!is_sensitive_path("/home/user/.ssh"));
        assert!(!is_sensitive_path("/project/.git/config"));
        assert!(!is_sensitive_path("/project/.env"));
        assert!(!is_sensitive_path("/home/user/Documents/readme.txt"));
    }
}
