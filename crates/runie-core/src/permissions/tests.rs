//! Layer 1 tests for the permission policy chain.

use std::path::Path;

use serde_json::Value;

use super::{
    ApprovalSink, AutoAllowSink, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove,
    PermissionAction, PermissionContext, PermissionManager, PermissionMode, PermissionPolicy,
    PermissionResult, PermissionRule, PermissionSet, ScriptedSink,
};

fn ctx<'a>(tool: &'a str, path: Option<&'a Path>, cwd: Option<&'a Path>) -> PermissionContext<'a> {
    PermissionContext {
        tool,
        path,
        input: None,
        cwd,
    }
}

#[tokio::test]
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
fn permission_mode_yolo_allows_everything() {
    let manager = PermissionManager::new(PermissionMode::Yolo);
    let ctx = ctx(
        "bash",
        Some(Path::new("/etc/passwd")),
        Some(Path::new("/tmp")),
    );
    // evaluate is async; use a minimal runtime block.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(manager.evaluate(&ctx));
    assert_eq!(result, PermissionResult::Allow);
}

#[test]
fn permission_mode_manual_always_asks() {
    let manager = PermissionManager::new(PermissionMode::Manual);
    let ctx = ctx("read_file", None, None);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(manager.evaluate(&ctx));
    assert_eq!(result, PermissionResult::Ask);
}

#[test]
fn wildcard_rule_matches_tool() {
    let rules = PermissionSet::new(vec![PermissionRule {
        tool_pattern: "*".into(),
        path_pattern: None,
        action: PermissionAction::Allow,
    }]);
    assert_eq!(rules.evaluate("bash", None), PermissionAction::Allow);
    assert_eq!(rules.evaluate("read_file", None), PermissionAction::Allow);
}

#[test]
fn path_rule_matches_file() {
    let rules = PermissionSet::new(vec![PermissionRule {
        tool_pattern: "read_file".into(),
        path_pattern: Some("src/**".into()),
        action: PermissionAction::Allow,
    }]);
    assert_eq!(
        rules.evaluate("read_file", Some("src/main.rs")),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.evaluate("read_file", Some("other/file.rs")),
        PermissionAction::Ask
    );
}

#[test]
fn last_rule_wins() {
    let rules = PermissionSet::new(vec![
        PermissionRule {
            tool_pattern: "bash".into(),
            path_pattern: None,
            action: PermissionAction::Allow,
        },
        PermissionRule {
            tool_pattern: "bash".into(),
            path_pattern: None,
            action: PermissionAction::Deny,
        },
    ]);
    assert_eq!(rules.evaluate("bash", None), PermissionAction::Deny);
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
    assert_eq!(rules.evaluate("bash", None), PermissionAction::Ask);
}

#[test]
fn permission_set_evaluates_rules() {
    let rules = PermissionSet::new(vec![
        PermissionRule {
            tool_pattern: "*".into(),
            path_pattern: None,
            action: PermissionAction::Deny,
        },
        PermissionRule {
            tool_pattern: "read_*".into(),
            path_pattern: None,
            action: PermissionAction::Allow,
        },
        PermissionRule {
            tool_pattern: "bash".into(),
            path_pattern: None,
            action: PermissionAction::Ask,
        },
    ]);
    assert_eq!(rules.evaluate("read_file", None), PermissionAction::Allow);
    assert_eq!(rules.evaluate("bash", None), PermissionAction::Ask);
    assert_eq!(rules.evaluate("unknown", None), PermissionAction::Deny);
}

#[test]
fn default_rules_read_only_allowed_write_asks() {
    let rules = PermissionSet::default_rules();
    assert_eq!(
        rules.effective_action("read_file", None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("list_dir", None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("grep", None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("find", None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("fetch_docs", None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("write_file", None),
        PermissionAction::Ask
    );
    assert_eq!(
        rules.effective_action("edit_file", None),
        PermissionAction::Ask
    );
    assert_eq!(rules.effective_action("bash", None), PermissionAction::Ask);
}

#[test]
fn effective_action_denies_sensitive_paths() {
    let rules = PermissionSet::default_rules();
    assert_eq!(
        rules.effective_action("read_file", Some("/home/user/.ssh/id_rsa")),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("write_file", Some("/project/.env")),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("read_file", Some("/project/src/main.rs")),
        PermissionAction::Allow
    );
}
