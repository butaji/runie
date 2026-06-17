use runie_core::permissions::{
    is_read_only_tool, is_sensitive_path, PermissionAction, PermissionSet,
};

#[test]
fn policy_matches_core() {
    let rules = PermissionSet::default_rules();

    assert_read_only_tools_allowed(&rules);
    assert_mutating_tools_ask(&rules);
    assert_sensitive_paths_denied(&rules);
    assert_non_sensitive_paths_follow_rules(&rules);
}

fn assert_read_only_tools_allowed(rules: &PermissionSet) {
    for tool in ["read_file", "list_dir", "grep", "find", "fetch_docs"] {
        assert_eq!(
            rules.effective_action(tool, None),
            PermissionAction::Allow,
            "{} should be allowed by default",
            tool
        );
        assert!(
            is_read_only_tool(tool),
            "{} should be classified read-only",
            tool
        );
    }
}

fn assert_mutating_tools_ask(rules: &PermissionSet) {
    for tool in ["write_file", "edit_file", "bash"] {
        assert_eq!(
            rules.effective_action(tool, None),
            PermissionAction::Ask,
            "{} should ask by default",
            tool
        );
        assert!(
            !is_read_only_tool(tool),
            "{} should not be classified read-only",
            tool
        );
    }
}

fn assert_sensitive_paths_denied(rules: &PermissionSet) {
    assert!(is_sensitive_path("/home/user/.ssh/id_rsa"));
    assert_eq!(
        rules.effective_action("read_file", Some("/home/user/.ssh/id_rsa")),
        PermissionAction::Deny
    );
    assert_eq!(
        rules.effective_action("write_file", Some("/project/.env")),
        PermissionAction::Deny
    );
}

fn assert_non_sensitive_paths_follow_rules(rules: &PermissionSet) {
    assert!(!is_sensitive_path("/project/src/main.rs"));
    assert_eq!(
        rules.effective_action("read_file", Some("/project/src/main.rs")),
        PermissionAction::Allow
    );
}
