//! Layer 1 tests for the permission policy chain.

use std::path::Path;

use serde_json::Value;

#[cfg(feature = "mcp")]
use super::DefaultToolApprove;
#[cfg(feature = "git")]
use super::GitTrackedWriteApprove;
use super::{
    ApprovalSink, AutoAllowSink, FileAccessAsk, PermissionAction, PermissionContext,
    PermissionManager, PermissionMode, PermissionPolicy, PermissionResult, PermissionRule,
    PermissionScope, PermissionSet, ScriptedSink,
};

mod declarative_rules;

fn ctx<'a>(tool: &'a str, path: Option<&'a Path>, cwd: Option<&'a Path>) -> PermissionContext<'a> {
    PermissionContext {
        tool,
        path,
        input: None,
        cwd,
        #[cfg(feature = "mcp")]
        annotations: crate::tool::annotations::get_tool_annotations(tool),
    }
}

#[tokio::test]
#[cfg(feature = "mcp")]
async fn permission_policy_chain_first_match_wins() {
    let manager = PermissionManager::new(PermissionMode::Auto).with_policies(vec![
        Box::new(DefaultToolApprove::new()),
        Box::new(FileAccessAsk::new()),
    ]);
    let cwd = Path::new("/tmp/project");
    let inside_path = cwd.join("src/main.rs");
    let inside = ctx("read_file", Some(inside_path.as_path()), Some(cwd));
    assert_eq!(manager.evaluate(&inside).await, PermissionResult::Allow);

    let outside = ctx("read_file", Some(Path::new("/etc/passwd")), Some(cwd));
    assert_eq!(manager.evaluate(&outside).await, PermissionResult::Allow);

    let write_outside = ctx("write_file", Some(Path::new("/etc/passwd")), Some(cwd));
    assert_eq!(
        manager.evaluate(&write_outside).await,
        PermissionResult::Ask
    );
}

#[tokio::test]
#[cfg(feature = "mcp")]
async fn default_tool_approve_allows_safe_tools() {
    let policy = DefaultToolApprove::new();
    for tool in ["read_file", "list_dir", "grep", "find", "fetch_docs"] {
        let context = ctx(tool, None, None);
        assert!(policy.matches(&context));
        assert_eq!(
            policy.evaluate(&context).await,
            Some(PermissionResult::Allow)
        );
    }
    let bash = ctx("bash", None, None);
    assert!(!policy.matches(&bash));
}

#[tokio::test]
#[cfg(feature = "git")]
async fn git_tracked_write_approve_passes_git_files() {
    let temp = tempfile::tempdir().unwrap();
    let repo_path = temp.path();
    let repo = git2::Repository::init(repo_path).unwrap();
    let file_path = repo_path.join("tracked.txt");
    std::fs::write(&file_path, "hello").unwrap();

    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();

    let policy = GitTrackedWriteApprove::new();
    let write_tracked = ctx("write_file", Some(&file_path), Some(repo_path));
    assert!(policy.matches(&write_tracked));
    assert_eq!(
        policy.evaluate(&write_tracked).await,
        Some(PermissionResult::Allow)
    );

    let untracked_path = repo_path.join("untracked.txt");
    std::fs::write(&untracked_path, "nope").unwrap();
    let write_untracked = ctx("write_file", Some(&untracked_path), Some(repo_path));
    assert!(!policy.matches(&write_untracked));
}

#[tokio::test]
async fn file_access_ask_requires_approval() {
    let policy = FileAccessAsk::new();
    let cwd = Path::new("/tmp/project");
    let inside_path = cwd.join("file.txt");
    let inside = ctx("read_file", Some(inside_path.as_path()), Some(cwd));
    assert!(!policy.matches(&inside));

    let outside = ctx("read_file", Some(Path::new("/etc/passwd")), Some(cwd));
    assert!(policy.matches(&outside));
    assert_eq!(policy.evaluate(&outside).await, Some(PermissionResult::Ask));
}

#[test]
fn wildcard_rule_matches_tool() {
    let rules = PermissionSet::new(vec![PermissionRule::new(PermissionAction::Allow, "*")]);
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Allow);
    assert_eq!(
        rules.evaluate("read_file", None, None),
        PermissionAction::Allow
    );
}

#[test]
fn path_rule_matches_file() {
    let rules = PermissionSet::new(vec![PermissionRule::new(
        PermissionAction::Allow,
        "read_file",
    )
    .with_path("src/**")]);
    assert_eq!(
        rules.evaluate("read_file", Some("src/main.rs"), None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.evaluate("read_file", Some("other/file.rs"), None),
        PermissionAction::Ask
    );
}

#[test]
fn last_rule_wins() {
    let rules = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Allow, "bash"),
        PermissionRule::new(PermissionAction::Deny, "bash"),
    ]);
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Deny);
}

#[test]
fn sensitive_path_denied() {
    assert!(super::is_sensitive_path("/home/user/.ssh/id_rsa"));
    assert!(super::is_sensitive_path("/project/.env"));
    assert!(!super::is_sensitive_path("/project/src/main.rs"));
}

#[test]
fn read_only_tool_classification() {
    assert!(super::is_read_only_tool("read_file"));
    assert!(super::is_read_only_tool("grep"));
    assert!(!super::is_read_only_tool("bash"));
    assert!(!super::is_read_only_tool("write_file"));
}

#[tokio::test]
async fn auto_allow_sink_always_allows() {
    let sink = AutoAllowSink;
    let action = sink
        .ask("bash", &serde_json::json!({"command": "ls"}))
        .await;
    assert_eq!(action, PermissionAction::Allow);
}

#[tokio::test]
async fn scripted_sink_returns_decisions() {
    let sink = ScriptedSink::new();
    sink.add_decision("bash", PermissionAction::Allow);
    sink.add_decision("write_file", PermissionAction::Deny);
    assert_eq!(
        sink.ask("bash", &Value::Null).await,
        PermissionAction::Allow
    );
    assert_eq!(
        sink.ask("write_file", &Value::Null).await,
        PermissionAction::Deny
    );
    assert_eq!(
        sink.ask("read_file", &Value::Null).await,
        PermissionAction::Ask
    );
}

#[test]
fn permission_set_default_is_ask() {
    let rules = PermissionSet::default();
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Ask);
}

#[test]
fn permission_set_evaluates_rules() {
    let rules = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Deny, "*"),
        PermissionRule::new(PermissionAction::Allow, "read_*"),
        PermissionRule::new(PermissionAction::Ask, "bash"),
    ]);
    assert_eq!(
        rules.evaluate("read_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Ask);
    assert_eq!(
        rules.evaluate("unknown", None, None),
        PermissionAction::Deny
    );
}

#[test]
fn default_rules_read_only_allowed_write_asks() {
    let rules = PermissionSet::default_rules();
    assert_eq!(
        rules.effective_action("read_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("list_dir", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("grep", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("find", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("fetch_docs", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("write_file", None, None),
        PermissionAction::Ask
    );
    assert_eq!(
        rules.effective_action("edit_file", None, None),
        PermissionAction::Ask
    );
    assert_eq!(
        rules.effective_action("bash", None, None),
        PermissionAction::Ask
    );
}

#[test]
fn effective_action_denies_sensitive_paths() {
    let rules = PermissionSet::default_rules();
    assert_eq!(
        rules.effective_action("read_file", Some("/home/user/.ssh/id_rsa"), None),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("write_file", Some("/project/.env"), None),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("read_file", Some("/project/src/main.rs"), None),
        PermissionAction::Allow
    );
}

// ============================================================================
// build_sink tests
// ============================================================================

#[tokio::test]
async fn build_sink_yolo_true_allows_all() {
    let sink = super::build_sink(true);
    let result = sink.ask("bash", &serde_json::Value::Null).await;
    assert_eq!(result, PermissionAction::Allow);
}

#[tokio::test]
async fn build_sink_yolo_false_denies_all() {
    let sink = super::build_sink(false);
    let result = sink.ask("bash", &serde_json::Value::Null).await;
    assert_eq!(result, PermissionAction::Deny);
}

// ============================================================================
// ApprovalDecision → PermissionAction conversion
// ============================================================================

#[test]
fn approval_decision_allow_maps_to_permission_allow() {
    use crate::proto::op::ApprovalDecision;
    let result = PermissionAction::from(ApprovalDecision::Allow);
    assert_eq!(result, PermissionAction::Allow);
}

#[test]
fn approval_decision_deny_maps_to_permission_deny() {
    use crate::proto::op::ApprovalDecision;
    let result = PermissionAction::from(ApprovalDecision::Deny);
    assert_eq!(result, PermissionAction::Deny);
}

// ============================================================================
// Layer 1: State/Logic tests for unify-permission-system-rules
// ============================================================================

/// Layer 1: Safe tools are approved by default.
#[tokio::test]
#[cfg(feature = "mcp")]
async fn default_allow_for_safe_tools() {
    let manager = PermissionManager::new(PermissionMode::Auto);
    for tool in ["read_file", "list_dir", "grep", "find", "fetch_docs"] {
        let context = ctx(tool, None, None);
        let result = manager.evaluate(&context).await;
        assert_eq!(
            result,
            PermissionResult::Allow,
            "{tool} should be auto-approved in Auto mode"
        );
    }
}

/// Layer 1: File access outside cwd triggers an approval request.
#[tokio::test]
async fn file_access_triggers_ask() {
    let manager = PermissionManager::new(PermissionMode::Default);
    let cwd = Path::new("/project");
    let outside_path = Path::new("/etc/passwd");
    let context = ctx("read_file", Some(outside_path), Some(cwd));
    let result = manager.evaluate(&context).await;
    assert_eq!(
        result,
        PermissionResult::Ask,
        "File access outside cwd should ask"
    );
}

/// Layer 1: Explicit deny rule wins over a default allow.
#[test]
fn explicit_deny_overrides() {
    let rules = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Allow, "bash"),
        PermissionRule::new(PermissionAction::Deny, "bash"),
    ]);
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Deny);
}

/// Layer 1: BypassPermissions mode approves everything.
#[tokio::test]
async fn bypass_permissions_approves_all() {
    let manager = PermissionManager::new(PermissionMode::BypassPermissions);
    let context = ctx("bash", None, None);
    let result = manager.evaluate(&context).await;
    assert_eq!(
        result,
        PermissionResult::Allow,
        "BypassPermissions should allow all"
    );
}

/// Layer 1: Plan mode blocks write tools.
#[tokio::test]
async fn plan_mode_blocks_writes() {
    let manager = PermissionManager::new(PermissionMode::Plan);
    for tool in ["write_file", "edit_file", "bash"] {
        let context = ctx(tool, None, None);
        let result = manager.evaluate(&context).await;
        assert_eq!(
            result,
            PermissionResult::Ask,
            "{tool} should ask in Plan mode"
        );
    }
}

/// Layer 1: AcceptEdits mode auto-approves edits.
#[tokio::test]
async fn accept_edits_mode_approves_writes() {
    let manager = PermissionManager::new(PermissionMode::AcceptEdits);
    for tool in ["write_file", "edit_file"] {
        let context = ctx(tool, None, None);
        let result = manager.evaluate(&context).await;
        assert_eq!(
            result,
            PermissionResult::Allow,
            "{tool} should be allowed in AcceptEdits mode"
        );
    }
}
