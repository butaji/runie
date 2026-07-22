//! Tests for file-based hook discovery and execution.
//!
//! Tests cover:
//! - Discovery from ~/.runie/hooks/{event}/ directories
//! - JSON hook definitions parsing
//! - Trust policy enforcement
//! - pre_tool_use blocking deny/allow
//! - Fail-open behavior on hook failure

use runie_core::hooks::{discover_hooks, default_hooks_dir, HookDecision, HookEvent};
use std::path::Path;
use tempfile::TempDir;

fn create_hook_file(dir: &Path, event: &str, command: &str, trust: &str) {
    let event_dir = dir.join(event);
    std::fs::create_dir_all(&event_dir).unwrap();
    let json = format!(
        r#"{{"command": "{}", "trust": "{}"}}"#,
        command.replace('"', "\\\""),
        trust
    );
    std::fs::write(event_dir.join("test.json"), json).unwrap();
}

#[test]
fn discover_pre_tool_use_hook() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::PreToolUse));
    assert_eq!(hooks.get(&HookEvent::PreToolUse).unwrap().len(), 1);
}

#[test]
fn discover_post_tool_use_hook() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "post_tool_use", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::PostToolUse));
}

#[test]
fn discover_session_start_hook() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "session_start", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::SessionStart));
}

#[test]
fn discover_session_end_hook() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "session_end", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::Stop));
}

#[test]
fn discover_subagent_hooks() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "subagent_start", "echo allow", "trusted");
    create_hook_file(temp.path(), "subagent_stop", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::SubagentStart));
    assert!(hooks.contains_key(&HookEvent::SubagentStop));
}

#[test]
fn multiple_hooks_per_event() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();

    std::fs::write(event_dir.join("hook1.json"), r#"{"command": "echo first", "trust": "trusted"}"#).unwrap();
    std::fs::write(event_dir.join("hook2.json"), r#"{"command": "echo second", "trust": "trusted"}"#).unwrap();

    let hooks = discover_hooks(temp.path());
    assert_eq!(hooks.get(&HookEvent::PreToolUse).unwrap().len(), 2);
}

#[test]
fn hook_allows_action() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "echo allow", "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({"tool": "read_file"}));
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn hook_denies_action() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "echo deny", "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({"tool": "write_file"}));
    assert_eq!(decision, HookDecision::Deny { reason: "hook denied".into() });
}

#[test]
fn hook_modifies_payload() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", r#"echo '{"action": "modified"}'"#, "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({"tool": "read_file"}));
    assert_eq!(
        decision,
        HookDecision::Modify { payload: serde_json::json!({"action": "modified"}) }
    );
}

#[test]
fn hook_fail_open_on_command_failure() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "exit 1", "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({"tool": "write_file"}));
    // Fail-open: command failure returns Allow
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn hook_fail_open_on_nonexistent_command() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "nonexistent_command_xyz", "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({"tool": "write_file"}));
    // Fail-open: nonexistent command returns Allow
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn hook_with_environment_variables() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(
        event_dir.join("env_hook.json"),
        r#"{
            "command": "sh -c 'echo $TEST_VAR'",
            "env": { "TEST_VAR": "test_value" },
            "trust": "trusted"
        }"#,
    )
    .unwrap();

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({}));
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn hook_with_timeout() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(
        event_dir.join("timeout_hook.json"),
        r#"{
            "command": "sleep 10",
            "timeout_ms": 100,
            "trust": "trusted"
        }"#,
    )
    .unwrap();

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({}));
    // Short timeout should cause failure -> fail-open -> Allow
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn untrusted_hook_loads_but_not_enforced() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "echo allow", "untrusted");

    let hooks = discover_hooks(temp.path());
    // Untrusted hook should still be loaded
    assert!(hooks.contains_key(&HookEvent::PreToolUse));
}

#[test]
fn empty_command_returns_allow() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "", "trusted");

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();
    let decision = handler.handle(&serde_json::json!({}));
    assert_eq!(decision, HookDecision::Allow);
}

#[test]
fn invalid_json_skipped() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(event_dir.join("invalid.json"), "{ not valid json").unwrap();

    let hooks = discover_hooks(temp.path());
    assert!(!hooks.contains_key(&HookEvent::PreToolUse));
}

#[test]
fn non_json_extension_skipped() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(event_dir.join("readme.txt"), "not a hook").unwrap();
    std::fs::write(event_dir.join("hook.json"), r#"{"command": "echo test"}"#).unwrap();

    let hooks = discover_hooks(temp.path());
    assert_eq!(hooks.get(&HookEvent::PreToolUse).unwrap().len(), 1);
}

#[test]
fn subdirectory_not_loaded() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    // Subdirectory with hook should not be loaded
    let subdir = event_dir.join("nested");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(subdir.join("hook.json"), r#"{"command": "echo test"}"#).unwrap();

    let hooks = discover_hooks(temp.path());
    // No hooks loaded because only subdirectory has hooks, not top-level
    assert!(!hooks.contains_key(&HookEvent::PreToolUse));
}

#[test]
fn case_insensitive_event_names() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("SESSION_START");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(event_dir.join("hook.json"), r#"{"command": "echo allow", "trust": "trusted"}"#).unwrap();

    let hooks = discover_hooks(temp.path());
    assert!(hooks.contains_key(&HookEvent::SessionStart));
}

#[test]
fn invalid_event_directory_ignored() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("invalid_event");
    std::fs::create_dir_all(&event_dir).unwrap();
    std::fs::write(event_dir.join("hook.json"), r#"{"command": "echo test"}"#).unwrap();

    let hooks = discover_hooks(temp.path());
    assert!(hooks.is_empty());
}

#[test]
fn default_hooks_dir_exists() {
    let dir = default_hooks_dir();
    // Should be a valid path
    assert!(dir.components().count() >= 2);
    assert!(dir.to_string_lossy().contains("hooks"));
}

#[test]
fn pre_tool_use_blocking_denies() {
    let temp = TempDir::new().unwrap();
    create_hook_file(temp.path(), "pre_tool_use", "echo deny", "trusted");

    let hooks = discover_hooks(temp.path());
    let handlers = hooks.get(&HookEvent::PreToolUse).unwrap();

    // First deny wins
    let decision = handlers[0].handle(&serde_json::json!({"tool": "any_tool"}));
    assert!(matches!(decision, HookDecision::Deny { .. }));
}

#[test]
fn pre_tool_use_modify_wins_on_allow() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    // First hook allows
    std::fs::write(event_dir.join("allow.json"), r#"{"command": "echo allow", "trust": "trusted"}"#).unwrap();
    // Second hook modifies
    std::fs::write(
        event_dir.join("modify.json"),
        r#"{"command": "echo '{\"modified\": true}'", "trust": "trusted"}"#,
    )
    .unwrap();

    let hooks = discover_hooks(temp.path());
    let handlers = hooks.get(&HookEvent::PreToolUse).unwrap();

    // Run all handlers
    let mut decision = HookDecision::Allow;
    for handler in handlers {
        decision = match handler.handle(&serde_json::json!({})) {
            d @ HookDecision::Deny { .. } => return assert!(matches!(d, HookDecision::Deny { .. })),
            modify @ HookDecision::Modify { .. } => modify,
            HookDecision::Allow => decision,
        };
    }
    // Last modification wins
    assert!(matches!(decision, HookDecision::Modify { .. }));
}

#[test]
fn hook_with_complex_command() {
    let temp = TempDir::new().unwrap();
    let event_dir = temp.path().join("pre_tool_use");
    std::fs::create_dir_all(&event_dir).unwrap();
    // Hook with environment variables passed via JSON env field
    std::fs::write(
        event_dir.join("complex.json"),
        r#"{
            "command": "sh -c 'if [ \"$TOOL\" = \"write_file\" ]; then echo deny; else echo allow; fi'",
            "env": { "TOOL": "write_file" },
            "trust": "trusted"
        }"#,
    )
    .unwrap();

    let hooks = discover_hooks(temp.path());
    let handler = hooks.get(&HookEvent::PreToolUse).unwrap().first().unwrap();

    // Tool is write_file (from env) -> should deny
    let decision = handler.handle(&serde_json::json!({"tool": "write_file"}));
    assert!(matches!(decision, HookDecision::Deny { .. }));
}
