use super::*;

#[test]
fn test_agent_loop_config_default() {
    let config = AgentLoopConfig::default();
    assert_eq!(config.max_turns, 50);
    assert!(config.system_prompt.is_empty());
    assert!(config.model.is_empty());
    assert!(config.thinking_level.is_empty());
}

#[test]
fn test_agent_loop_config_with_values() {
    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "gpt-4".to_string(),
        thinking_level: "high".to_string(),
        max_turns: 20,
    };
    assert_eq!(config.max_turns, 20);
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.thinking_level, "high");
}

#[test]
fn test_hook_decision_allow() {
    let decision = HookDecision::Allow;
    assert!(matches!(decision, HookDecision::Allow));
}

#[test]
fn test_hook_decision_block() {
    let decision = HookDecision::Block { reason: "dangerous".to_string() };
    assert!(matches!(decision, HookDecision::Block { .. }));
}

#[test]
fn test_hook_decision_modify() {
    let new_args = serde_json::json!({"command": "safe"});
    let decision = HookDecision::Modify { args: new_args.clone() };
    assert!(matches!(decision, HookDecision::Modify { .. }));
}

#[test]
fn test_permission_decision_allow() {
    let decision = PermissionDecision::Allow { tool_call_id: "call_1".to_string(), tool_name: "test".to_string(), tool_args: "{}".to_string() };
    assert!(matches!(decision, PermissionDecision::Allow { .. }));
}

#[test]
fn test_permission_decision_deny() {
    let decision = PermissionDecision::Deny { tool_call_id: "call_1".to_string(), tool_name: "test".to_string(), tool_args: "{}".to_string() };
    assert!(matches!(decision, PermissionDecision::Deny { .. }));
}

#[test]
fn test_permission_decision_allow_always() {
    let decision = PermissionDecision::AllowAlways { tool_call_id: "call_1".to_string(), tool_name: "test".to_string(), tool_args: "{}".to_string() };
    assert!(matches!(decision, PermissionDecision::AllowAlways { .. }));
}

#[test]
fn test_permission_decision_skip() {
    let decision = PermissionDecision::Skip { tool_call_id: "call_1".to_string(), tool_name: "test".to_string(), tool_args: "{}".to_string() };
    assert!(matches!(decision, PermissionDecision::Skip { .. }));
}

#[test]
fn test_agent_state_default() {
    let session = Session::new("test".to_string());
    let state = AgentState::new(session.clone());
    assert_eq!(state.turn_count, 0);
}

#[test]
fn test_agent_state_new_with_session() {
    let session = Session::new("my-session".to_string());
    let state = AgentState::new(session.clone());
    assert_eq!(state.session.id, "my-session");
    assert_eq!(state.turn_count, 0);
}

#[test]
fn test_agent_state_add_message() {
    let session = Session::new("test".to_string());
    let mut state = AgentState::new(session);
    let msg = Message::User { content: "Hello".to_string(), attachments: vec![] };
    let id = state.add_message(None, msg);
    assert!(!id.is_empty());
    assert_eq!(state.session.messages.len(), 1);
}

#[test]
fn test_content_part_text() {
    let part = ContentPart::Text { text: "Hello".to_string() };
    assert!(matches!(part, ContentPart::Text { .. }));
}

#[test]
fn test_content_part_tool_use() {
    let part = ContentPart::ToolUse {
        id: "call_1".to_string(),
        name: "bash".to_string(),
        input: serde_json::json!({}),
    };
    assert!(matches!(part, ContentPart::ToolUse { .. }));
}

#[test]
fn test_content_part_tool_result() {
    let part = ContentPart::ToolResult {
        tool_use_id: "call_1".to_string(),
        content: vec![ContentPart::Text { text: "done".to_string() }],
        is_error: false,
    };
    assert!(matches!(part, ContentPart::ToolResult { .. }));
}

#[tokio::test]
async fn test_safety_hook_allows_safe_command() {
    let hook = SafetyHook;
    let tool_call = runie_core::ToolCall {
        id: "call_1".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "echo hello"}),
    };
    let ctx = Context::default();
    let result = hook.before_tool_call(&tool_call, &ctx).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookDecision::Allow));
}

#[tokio::test]
async fn test_safety_hook_blocks_dangerous_command() {
    let hook = SafetyHook;
    let tool_call = runie_core::ToolCall {
        id: "call_1".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "rm -rf /"}),
    };
    let ctx = Context::default();
    let result = hook.before_tool_call(&tool_call, &ctx).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookDecision::Block { .. }));
}

#[tokio::test]
async fn test_safety_hook_allows_non_bash_tool() {
    let hook = SafetyHook;
    let tool_call = runie_core::ToolCall {
        id: "call_1".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({"path": "test.txt"}),
    };
    let ctx = Context::default();
    let result = hook.before_tool_call(&tool_call, &ctx).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookDecision::Allow));
}
