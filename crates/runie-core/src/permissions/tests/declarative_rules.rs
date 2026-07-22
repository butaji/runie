//! Tests for declarative permission rules.
//!
//! These tests cover the new permission rules functionality including:
//! - Scope-based rules
//! - Command pattern matching
//! - Permission modes
//! - Layered evaluation

use super::{PermissionAction, PermissionMode, PermissionRule, PermissionScope, PermissionSet};

#[test]
fn permission_rule_with_path() {
    let rule = PermissionRule::new(PermissionAction::Allow, "read_file").with_path("src/**");
    assert!(rule.matches("read_file", Some("src/main.rs"), None));
    assert!(!rule.matches("read_file", Some("tests/main.rs"), None));
    assert!(!rule.matches("write_file", Some("src/main.rs"), None));
}

#[test]
fn permission_rule_with_pattern() {
    let rule = PermissionRule::new(PermissionAction::Deny, "bash").with_pattern("rm -rf *");
    assert!(rule.matches("bash", None, Some("rm -rf /tmp")));
    assert!(!rule.matches("bash", None, Some("ls -la")));
    assert!(!rule.matches("read_file", None, Some("rm -rf /tmp")));
}

#[test]
fn permission_rule_with_scope() {
    let user_rule = PermissionRule::new(PermissionAction::Allow, "bash").with_scope(PermissionScope::User);
    let project_rule = PermissionRule::new(PermissionAction::Deny, "bash").with_scope(PermissionScope::Project);
    let session_rule = PermissionRule::new(PermissionAction::Ask, "bash").with_scope(PermissionScope::Session);

    assert_eq!(user_rule.scope, PermissionScope::User);
    assert_eq!(project_rule.scope, PermissionScope::Project);
    assert_eq!(session_rule.scope, PermissionScope::Session);
}

#[test]
fn permission_set_with_scope_precedence() {
    let mut rules = PermissionSet::new(vec![]);
    rules.add_rule(PermissionRule::new(PermissionAction::Allow, "bash").with_scope(PermissionScope::User));
    rules.add_rule(PermissionRule::new(PermissionAction::Deny, "bash").with_scope(PermissionScope::Project));
    rules.add_rule(PermissionRule::new(PermissionAction::Ask, "bash").with_scope(PermissionScope::Session));

    // Session rules should take precedence
    assert_eq!(
        rules.evaluate_with_scope("bash", None, None, PermissionScope::Session),
        PermissionAction::Ask
    );
}

#[test]
fn permission_set_filters_by_max_scope() {
    let mut rules = PermissionSet::new(vec![]);
    rules.add_rule(PermissionRule::new(PermissionAction::Allow, "bash").with_scope(PermissionScope::User));
    rules.add_rule(PermissionRule::new(PermissionAction::Deny, "bash").with_scope(PermissionScope::Project));
    rules.add_rule(PermissionRule::new(PermissionAction::Ask, "bash").with_scope(PermissionScope::Session));

    // When max scope is Project, Session rules are filtered out
    assert_eq!(
        rules.evaluate_with_scope("bash", None, None, PermissionScope::Project),
        PermissionAction::Deny
    );

    // When max scope is User, Project and Session rules are filtered out
    assert_eq!(
        rules.evaluate_with_scope("bash", None, None, PermissionScope::User),
        PermissionAction::Allow
    );
}

#[test]
fn permission_set_rules_for_scope() {
    let mut rules = PermissionSet::new(vec![]);
    rules.add_rule(PermissionRule::new(PermissionAction::Allow, "read_file").with_scope(PermissionScope::User));
    rules.add_rule(PermissionRule::new(PermissionAction::Deny, "bash").with_scope(PermissionScope::User));
    rules.add_rule(PermissionRule::new(PermissionAction::Ask, "write_file").with_scope(PermissionScope::Project));

    let user_rules = rules.rules_for_scope(PermissionScope::User);
    assert_eq!(user_rules.len(), 2);

    let project_rules = rules.rules_for_scope(PermissionScope::Project);
    assert_eq!(project_rules.len(), 1);

    let session_rules = rules.rules_for_scope(PermissionScope::Session);
    assert!(session_rules.is_empty());
}

#[test]
fn permission_set_extend() {
    let mut rules1 = PermissionSet::new(vec![PermissionRule::new(
        PermissionAction::Allow,
        "read_file",
    )
    .with_scope(PermissionScope::User)]);
    let rules2 = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Deny, "bash").with_scope(PermissionScope::User)
    ]);
    rules1.extend(rules2);
    assert_eq!(rules1.rules().len(), 2);
}

#[test]
fn accept_edits_rules_auto_approve_file_edits() {
    let rules = PermissionSet::accept_edits_rules();
    assert_eq!(
        rules.effective_action("write_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("edit_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("bash", None, None),
        PermissionAction::Ask
    );
}

#[test]
fn dont_ask_rules_allow_all_except_deny() {
    let rules = PermissionSet::dont_ask_rules();
    assert_eq!(
        rules.effective_action("bash", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("write_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.effective_action("read_file", None, None),
        PermissionAction::Allow
    );
}

// ============================================================================
// Permission mode tests
// ============================================================================

#[test]
fn permission_mode_bypasses_all() {
    assert!(!PermissionMode::Default.bypasses_all());
    assert!(!PermissionMode::Auto.bypasses_all());
    assert!(!PermissionMode::AcceptEdits.bypasses_all());
    assert!(!PermissionMode::DontAsk.bypasses_all());
    assert!(PermissionMode::BypassPermissions.bypasses_all());
    assert!(!PermissionMode::Plan.bypasses_all());
}

#[test]
fn permission_mode_requires_plan() {
    assert!(!PermissionMode::Default.requires_plan());
    assert!(!PermissionMode::Auto.requires_plan());
    assert!(!PermissionMode::AcceptEdits.requires_plan());
    assert!(!PermissionMode::DontAsk.requires_plan());
    assert!(!PermissionMode::BypassPermissions.requires_plan());
    assert!(PermissionMode::Plan.requires_plan());
}

#[test]
fn permission_mode_auto_approves_edits() {
    assert!(!PermissionMode::Default.auto_approves_edits());
    assert!(PermissionMode::AcceptEdits.auto_approves_edits());
    assert!(!PermissionMode::Auto.auto_approves_edits());
}

#[test]
fn permission_mode_auto_approves_safe() {
    assert!(PermissionMode::Auto.auto_approves_safe());
    assert!(PermissionMode::AcceptEdits.auto_approves_safe());
    assert!(!PermissionMode::Default.auto_approves_safe());
    assert!(!PermissionMode::DontAsk.auto_approves_safe());
}

#[test]
fn permission_mode_serialization() {
    use serde_json;
    let modes = vec![
        PermissionMode::Default,
        PermissionMode::Auto,
        PermissionMode::AcceptEdits,
        PermissionMode::DontAsk,
        PermissionMode::BypassPermissions,
        PermissionMode::Plan,
    ];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, parsed);
    }
}

// ============================================================================
// Rule evaluation matrix tests
// ============================================================================

#[test]
fn rule_evaluation_matrix_allow_deny_ask() {
    let mut rules = PermissionSet::new(vec![]);
    rules.add_rule(PermissionRule::new(PermissionAction::Allow, "read_file"));
    rules.add_rule(PermissionRule::new(PermissionAction::Deny, "bash"));

    assert_eq!(
        rules.evaluate("read_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Deny);
    assert_eq!(rules.evaluate("unknown", None, None), PermissionAction::Ask);
}

#[test]
fn rule_evaluation_with_wildcard_patterns() {
    let rules = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Deny, "*"),
        PermissionRule::new(PermissionAction::Allow, "read_*"),
        PermissionRule::new(PermissionAction::Ask, "list_*"),
    ]);

    assert_eq!(
        rules.evaluate("read_file", None, None),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.evaluate("list_dir", None, None),
        PermissionAction::Ask
    );
    assert_eq!(rules.evaluate("bash", None, None), PermissionAction::Deny);
}

#[test]
fn rule_evaluation_complex_pattern() {
    let rules = PermissionSet::new(vec![
        PermissionRule::new(PermissionAction::Deny, "bash").with_pattern("rm -rf *"),
        PermissionRule::new(PermissionAction::Allow, "bash").with_pattern("git *"),
    ]);

    assert_eq!(
        rules.evaluate("bash", None, Some("rm -rf /tmp")),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.evaluate("bash", None, Some("git status")),
        PermissionAction::Allow
    );
    assert_eq!(
        rules.evaluate("bash", None, Some("ls -la")),
        PermissionAction::Ask
    );
}
