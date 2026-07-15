//! Tool authorization types and trait.
//!
//! This module separates authorization checks from tool execution. Permission
//! decisions are made before any tool runs, allowing callers to:
//! - Deny dangerous operations early
//! - Prompt the user for approval
//! - Allow safe operations to proceed
//!
//! ## Usage
//!
//! Tools implement [`Authorizable`] to provide both an `authorize` check and
//! a separate `execute_async` method. The authorization step can be called
//! before deciding whether to execute.

use serde_json::Value;

#[cfg(test)]
use serde_json::json;

/// Result of an authorization check for a tool invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthorizeResult {
    /// The tool call is authorized to proceed.
    Allowed,
    /// The tool call is denied with a reason.
    Denied { reason: String },
    /// The tool call requires user confirmation before proceeding.
    AskUser,
}

impl AuthorizeResult {
    /// Check if the result allows execution.
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthorizeResult::Allowed)
    }

    /// Check if the result requires user interaction.
    pub fn requires_user(&self) -> bool {
        matches!(self, AuthorizeResult::AskUser)
    }

    /// Check if the result denies execution.
    pub fn is_denied(&self) -> bool {
        matches!(self, AuthorizeResult::Denied { .. })
    }

    /// Get the denial reason, if denied.
    pub fn reason(&self) -> Option<&str> {
        match self {
            AuthorizeResult::Denied { reason } => Some(reason),
            _ => None,
        }
    }
}

/// Authorization context passed to authorize checks.
///
/// This provides the information needed to make an authorization decision
/// without full tool execution.
#[derive(Debug, Clone)]
pub struct AuthorizationContext<'a> {
    /// Name of the tool being invoked.
    pub tool_name: &'a str,
    /// Arguments passed to the tool.
    pub input: &'a Value,
    /// Whether this is the first call in a turn (vs a retry).
    pub is_first_call: bool,
}

impl<'a> AuthorizationContext<'a> {
    /// Create a new authorization context.
    pub fn new(tool_name: &'a str, input: &'a Value) -> Self {
        Self {
            tool_name,
            input,
            is_first_call: true,
        }
    }

    /// Create a new context with first-call flag.
    pub fn with_first_call(tool_name: &'a str, input: &'a Value, is_first_call: bool) -> Self {
        Self {
            tool_name,
            input,
            is_first_call,
        }
    }
}

/// A tool that supports authorization checks before execution.
///
/// Implement this trait to separate permission checks from actual tool execution.
/// The `authorize` method should perform fast validation without side effects,
/// while `execute_async` performs the actual tool work.
pub trait Authorizable: Send + Sync {
    /// Authorize a tool call without executing it.
    ///
    /// Returns `AuthorizeResult::Allowed` if the call is permitted,
    /// `AuthorizeResult::Denied` with a reason if it should be blocked,
    /// or `AuthorizeResult::AskUser` if user confirmation is needed.
    fn authorize(&self, ctx: &AuthorizationContext<'_>) -> AuthorizeResult;

    /// Execute the tool after authorization has passed.
    ///
    /// This is called only after `authorize` returns `Allowed`.
    /// Implementations should assume the call has been pre-authorized.
    #[allow(async_fn_in_trait)]
    async fn execute_async(&self, input: Value) -> super::ToolOutput;
}

// ---------------------------------------------------------------------------
// Default authorization helpers
// ---------------------------------------------------------------------------

/// Default authorization for read-only tools.
///
/// Read-only tools that don't access sensitive data are always allowed.
pub fn allow_readonly(_ctx: &AuthorizationContext<'_>) -> AuthorizeResult {
    AuthorizeResult::Allowed
}

/// Default authorization that always asks the user.
pub fn always_ask(_ctx: &AuthorizationContext<'_>) -> AuthorizeResult {
    AuthorizeResult::AskUser
}

/// Default authorization that denies with a reason.
pub fn deny(reason: &str) -> impl Fn(&AuthorizationContext<'_>) -> AuthorizeResult + '_ {
    move |_ctx: &AuthorizationContext<'_>| AuthorizeResult::Denied {
        reason: reason.to_owned(),
    }
}

/// Authorization based on tool input patterns.
///
/// Returns `AskUser` if the input contains suspicious patterns, otherwise `Allowed`.
pub fn check_input_patterns(
    ctx: &AuthorizationContext<'_>,
    dangerous_patterns: &[&str],
) -> AuthorizeResult {
    let input_str = ctx.input.to_string();
    for pattern in dangerous_patterns {
        if input_str.contains(pattern) {
            return AuthorizeResult::AskUser;
        }
    }
    AuthorizeResult::Allowed
}

// ---------------------------------------------------------------------------
// Standalone authorize function
// ---------------------------------------------------------------------------

/// Check authorization for a tool by name.
///
/// This is a convenience function for cases where you don't have a concrete
/// tool implementation but need to check basic authorization rules based on
/// tool name conventions.
///
/// For full authorization with input inspection, use the `Authorizable` trait.
pub fn authorize(tool_name: &str, _input: &Value) -> AuthorizeResult {
    // Built-in tools that always require user approval
    let requires_approval = ["bash", "write_file", "edit_file", "rm", "delete"];

    if requires_approval.contains(&tool_name) {
        AuthorizeResult::AskUser
    } else {
        AuthorizeResult::Allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_result_allowed() {
        let result = AuthorizeResult::Allowed;
        assert!(result.is_allowed());
        assert!(!result.is_denied());
        assert!(!result.requires_user());
        assert!(result.reason().is_none());
    }

    #[test]
    fn authorize_result_denied() {
        let result = AuthorizeResult::Denied {
            reason: "dangerous command".to_owned(),
        };
        assert!(!result.is_allowed());
        assert!(result.is_denied());
        assert!(!result.requires_user());
        assert_eq!(result.reason(), Some("dangerous command"));
    }

    #[test]
    fn authorize_result_ask_user() {
        let result = AuthorizeResult::AskUser;
        assert!(!result.is_allowed());
        assert!(!result.is_denied());
        assert!(result.requires_user());
        assert!(result.reason().is_none());
    }

    #[test]
    fn authorization_context_new() {
        let input = json!({"path": "/tmp/test"});
        let ctx = AuthorizationContext::new("read_file", &input);
        assert_eq!(ctx.tool_name, "read_file");
        assert_eq!(ctx.input, &input);
        assert!(ctx.is_first_call);
    }

    #[test]
    fn authorization_context_with_first_call() {
        let input = json!({});
        let ctx = AuthorizationContext::with_first_call("bash", &input, false);
        assert_eq!(ctx.tool_name, "bash");
        assert!(!ctx.is_first_call);
    }

    #[test]
    fn allow_readonly_always_allows() {
        let ctx = AuthorizationContext::new("grep", &json!({}));
        assert!(matches!(allow_readonly(&ctx), AuthorizeResult::Allowed));
    }

    #[test]
    fn always_ask_always_asks() {
        let ctx = AuthorizationContext::new("bash", &json!({}));
        assert!(matches!(always_ask(&ctx), AuthorizeResult::AskUser));
    }

    #[test]
    fn deny_creates_denial() {
        let ctx = AuthorizationContext::new("rm", &json!({}));
        let check = deny("not allowed");
        let result = check(&ctx);
        assert!(result.is_denied());
        assert_eq!(result.reason(), Some("not allowed"));
    }

    #[test]
    fn check_input_patterns_allows_safe() {
        let ctx = AuthorizationContext::new("read_file", &json!({"path": "/safe/path"}));
        let patterns = ["sudo", "rm -rf", "eval"];
        let result = check_input_patterns(&ctx, &patterns);
        assert!(result.is_allowed());
    }

    #[test]
    fn check_input_patterns_asks_on_match() {
        let ctx = AuthorizationContext::new(
            "bash",
            &json!({"command": "sudo rm -rf /important"}),
        );
        let patterns = ["sudo", "rm -rf", "eval"];
        let result = check_input_patterns(&ctx, &patterns);
        assert!(result.requires_user());
    }

    #[test]
    fn authorize_asks_for_dangerous_tools() {
        assert!(authorize("bash", &json!({})).requires_user());
        assert!(authorize("write_file", &json!({})).requires_user());
        assert!(authorize("edit_file", &json!({})).requires_user());
        assert!(authorize("rm", &json!({})).requires_user());
        assert!(authorize("delete", &json!({})).requires_user());
    }

    #[test]
    fn authorize_allows_safe_tools() {
        assert!(authorize("read_file", &json!({})).is_allowed());
        assert!(authorize("grep", &json!({})).is_allowed());
        assert!(authorize("find", &json!({})).is_allowed());
        assert!(authorize("list_dir", &json!({})).is_allowed());
    }

    // Test Authorizable trait implementation
    struct MockTool {
        name: &'static str,
        requires_approval: bool,
    }

    impl Authorizable for MockTool {
        fn authorize(&self, ctx: &AuthorizationContext<'_>) -> AuthorizeResult {
            if self.requires_approval {
                AuthorizeResult::AskUser
            } else {
                AuthorizeResult::Allowed
            }
        }

        async fn execute_async(&self, input: Value) -> super::ToolOutput {
            super::ToolOutput::success(self.name, input, "executed".into())
        }
    }

    #[test]
    fn authorizable_trait_ask_user() {
        let tool = MockTool {
            name: "bash",
            requires_approval: true,
        };
        let ctx = AuthorizationContext::new("bash", &json!({}));
        assert!(matches!(tool.authorize(&ctx), AuthorizeResult::AskUser));
    }

    #[test]
    fn authorizable_trait_allowed() {
        let tool = MockTool {
            name: "read_file",
            requires_approval: false,
        };
        let ctx = AuthorizationContext::new("read_file", &json!({}));
        assert!(matches!(tool.authorize(&ctx), AuthorizeResult::Allowed));
    }

    #[tokio::test]
    async fn authorizable_executes_after_auth() {
        let tool = MockTool {
            name: "read_file",
            requires_approval: false,
        };
        let input = json!({"path": "/test"});
        let result = tool.execute_async(input.clone()).await;
        assert_eq!(result.tool_name, "read_file");
        assert_eq!(result.content, "executed");
    }
}
