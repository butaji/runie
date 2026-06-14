use crate::policy::{glob_match, path_to_glob_string, PathRule, ToolPolicy, ToolPolicyGroup};

// ─── ToolPolicyGroup ──────────────────────────────────────────────────────────

#[test]
fn read_only_group_no_approval() {
    assert!(!ToolPolicyGroup::ReadOnly.requires_approval());
}

#[test]
fn write_group_requires_approval() {
    assert!(ToolPolicyGroup::Write.requires_approval());
}

#[test]
fn dangerous_group_requires_approval() {
    assert!(ToolPolicyGroup::Dangerous.requires_approval());
}

#[test]
fn for_tool_name_read_only() {
    assert_eq!(ToolPolicy::for_tool_name("read_file"), Some(ToolPolicyGroup::ReadOnly));
    assert_eq!(ToolPolicy::for_tool_name("grep"), Some(ToolPolicyGroup::ReadOnly));
    assert_eq!(ToolPolicy::for_tool_name("list_dir"), Some(ToolPolicyGroup::ReadOnly));
}

#[test]
fn for_tool_name_write() {
    assert_eq!(ToolPolicy::for_tool_name("write_file"), Some(ToolPolicyGroup::Write));
    assert_eq!(ToolPolicy::for_tool_name("edit_file"), Some(ToolPolicyGroup::Write));
}

#[test]
fn for_tool_name_dangerous() {
    assert_eq!(ToolPolicy::for_tool_name("bash"), Some(ToolPolicyGroup::Dangerous));
}

#[test]
fn for_tool_name_unknown() {
    assert_eq!(ToolPolicy::for_tool_name("not_a_tool"), None);
}

// ─── PathRule ────────────────────────────────────────────────────────────────

#[test]
fn path_rule_allow_rust_files() {
    let rule = PathRule::allow("**/*.rs");
    let path = std::path::Path::new("src/lib.rs");
    assert!(rule.matches(path));
    assert!(rule.matches(std::path::Path::new("crates/runie-core/src/main.rs")));
    assert!(!rule.matches(std::path::Path::new("src/main.txt")));
}

#[test]
fn path_rule_deny_ssh_absolute() {
    // Use an absolute path pattern (tilde expansion only works when HOME is set)
    let rule = PathRule::deny("/home/*/.ssh/*");
    assert!(rule.matches(std::path::Path::new("/home/user/.ssh/id_rsa")));
    assert!(!rule.matches(std::path::Path::new("/home/user/.bashrc")));
}

#[test]
fn path_rule_glob_double_star() {
    let rule = PathRule::allow("src/**/*.rs");
    assert!(rule.matches(std::path::Path::new("src/lib.rs")));
    assert!(rule.matches(std::path::Path::new("src/deep/nested.rs")));
    assert!(!rule.matches(std::path::Path::new("tests/foo.rs")));
}

// ─── glob_match ──────────────────────────────────────────────────────────────

#[test]
fn glob_match_simple_star() {
    assert!(glob_match("*.txt", "hello.txt"));
    assert!(!glob_match("*.txt", "hello.rs"));
}

#[test]
fn glob_match_double_star() {
    assert!(glob_match("src/**/*.rs", "src/lib.rs"));
    assert!(glob_match("src/**/*.rs", "src/deep/nested.rs"));
    assert!(!glob_match("src/**/*.rs", "tests/deep.rs"));
}

#[test]
fn glob_match_question_mark() {
    assert!(glob_match("file?.rs", "file1.rs"));
    assert!(glob_match("file?.rs", "fileA.rs"));
    assert!(!glob_match("file?.rs", "file12.rs"));
}

#[test]
fn glob_match_literal() {
    assert!(glob_match("Cargo.toml", "Cargo.toml"));
    assert!(!glob_match("Cargo.toml", "Cargo.lock"));
}

#[test]
fn glob_match_tilde_expansion_uses_home() {
    // Tilde expansion only works when HOME env var is set
    if let Ok(home) = std::env::var("HOME") {
        let path = format!("{}/.ssh/id_rsa", home);
        assert!(glob_match("~/.ssh/id_rsa", &path));
    }
}

// ─── path_to_glob_string ─────────────────────────────────────────────────────

#[test]
fn path_to_glob_string_windows_backslash() {
    let result = path_to_glob_string(std::path::Path::new("C:\\Users\\test\\file.txt"));
    assert_eq!(result, "C:/Users/test/file.txt");
}

// ─── ToolPolicy::effective_approval ──────────────────────────────────────────

#[test]
fn effective_approval_path_denied() {
    let policy = ToolPolicy {
        group: ToolPolicyGroup::Write,
        path_rules: vec![PathRule::deny("/etc/**")],
    };
    assert_eq!(
        policy.effective_approval(Some(std::path::Path::new("/etc/passwd"))),
        Some(false)
    );
}

#[test]
fn effective_approval_path_allowed() {
    let policy = ToolPolicy {
        group: ToolPolicyGroup::Write,
        path_rules: vec![PathRule::allow("/home/**/*.md")],
    };
    assert_eq!(
        policy.effective_approval(Some(std::path::Path::new("/home/user/readme.md"))),
        Some(true)
    );
}

#[test]
fn effective_approval_no_matching_rule() {
    let policy = ToolPolicy {
        group: ToolPolicyGroup::Write,
        path_rules: vec![],
    };
    assert_eq!(policy.effective_approval(Some(std::path::Path::new("/some/path"))), None);
}
