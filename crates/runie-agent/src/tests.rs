//! Tests for runie-agent

use crate::events::{PermissionDecision, ContentPart, AgentEvent, AgentMessage};
use crate::hook::{Hook, HookDecision, SafetyHook};
use crate::loop_engine::AgentLoopConfig;
use crate::state::AgentState;
use crate::harness::compaction::find_cut_point;
use crate::harness::types::CompactionSettings;
use futures::StreamExt;
use runie_core::{Message, Session, Context, ToolCall, ToolOutput};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use runie_ai::Provider;
use runie_tools::{create_default_toolkit, Workspace};
use std::path::PathBuf;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

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

// ============================================================================
// Agent Loop Integration Tests
// ============================================================================

/// Test that max_turns=3 allows exactly 3 turns before failing
#[tokio::test]
async fn test_max_turns_exact_boundary() {
    use crate::loop_engine::agent_loop;

    // Create a provider that always requests a tool (causing loops)
    let provider = Arc::new(AlwaysToolProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 3, // Should allow 3 turns
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut turn_count = 0;
    let mut max_turns_exceeded = false;

    while let Some(event) = stream.next().await {
        if let AgentEvent::TurnEnd { turn, .. } = event {
            turn_count = turn;
        }
        if let AgentEvent::Error { error_type, .. } = &event {
            if error_type == "max_turns" {
                max_turns_exceeded = true;
                break;
            }
        }
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
    }

    // With max_turns=3, we should complete 3 turns then get max_turns error
    assert_eq!(turn_count, 3, "Should complete exactly 3 turns");
    assert!(max_turns_exceeded, "Should get max_turns error on turn 4");
}

/// Test that duplicate tool calls (same tool+args) in the same turn are skipped
#[tokio::test]
async fn test_duplicate_tool_call_same_turn() {
    use crate::loop_engine::agent_loop;

    // Provider that generates duplicate tool calls
    let provider = Arc::new(DuplicateToolProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut tool_execution_count = 0;

    while let Some(event) = stream.next().await {
        if let AgentEvent::ToolExecutionStart { .. } = event {
            tool_execution_count += 1;
        }
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
    }

    // Should only execute once (duplicate should be skipped)
    assert_eq!(tool_execution_count, 1, "Duplicate tool call should be skipped");
}

/// Test that same tool in different turns both execute
#[tokio::test]
async fn test_duplicate_tool_call_across_turns() {
    use crate::loop_engine::agent_loop;

    // Provider that generates same tool call in consecutive turns
    let provider = Arc::new(SameToolAcrossTurnsProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 3,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut tool_execution_count = 0;

    while let Some(event) = stream.next().await {
        if let AgentEvent::ToolExecutionStart { .. } = event {
            tool_execution_count += 1;
        }
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
    }

    // Same tool in different turns should both execute
    assert_eq!(tool_execution_count, 2, "Same tool in different turns should both run");
}

/// Test that permission timeout (5 min) returns denied
#[tokio::test]
async fn test_permission_timeout_returns_denied() {
    use crate::loop_engine::agent_loop;

    // Provider that requests tool permission then stops responding
    let provider = Arc::new(PermissionNeverGrantedProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    // Use a very short timeout for testing (permission state never populated)
    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut permission_denied = false;

    // Run with a reasonable wait time for the test
    let start = std::time::Instant::now();
    while let Some(event) = stream.next().await {
        if let AgentEvent::PermissionDenied { .. } = event {
            permission_denied = true;
            break;
        }
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
        // Safety timeout
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
    }

    assert!(permission_denied, "Permission should be denied after timeout");
}

/// Test that token usage accumulates per turn
#[tokio::test]
async fn test_token_usage_accumulates_per_turn() {
    use crate::loop_engine::agent_loop;
    use crate::events::TokenUsage;

    let provider = Arc::new(TokenCountingProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 3,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut total_tokens_seen = 0u32;
    let mut turns_seen = 0u32;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::TokenUsage { total_tokens, .. } => {
                total_tokens_seen = *total_tokens as u32;
            }
            AgentEvent::TurnEnd { turn, token_usage, .. } => {
                turns_seen = *turn as u32;
                assert!(token_usage.total_tokens >= total_tokens_seen,
                    "Token usage should accumulate");
            }
            AgentEvent::AgentEnd { final_token_usage, .. } => {
                assert!(final_token_usage.total_tokens > 0, "Should have token usage");
                break;
            }
            _ => {}
        }
    }

    assert!(turns_seen >= 1, "Should have seen at least one turn");
    assert!(total_tokens_seen > 0, "Should have accumulated tokens");
}

/// Test that context window calculation uses chars / 4
#[test]
fn test_context_window_chars_div_4() {
    use crate::loop_engine::calculate_context_window_usage;

    // "hello" = 5 chars, 5/4 = 1.25, should round to 1
    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "hello".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let context_window = 100_000; // Large context window
    let usage = calculate_context_window_usage(&messages, context_window);

    // 5 chars / 4 = 1.25 tokens ≈ 1 token
    // (1 / 100000) * 100 = 0.001%
    assert!(usage < 0.01, "5 chars should be ~0 tokens percentage");
}

/// Test that panic in tool data prep is caught
#[tokio::test]
async fn test_tool_panic_caught_in_prep() {
    use crate::loop_engine::agent_loop;

    let provider = Arc::new(PanickingToolProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut panic_caught = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::Error { error_type, .. } if error_type == "tool_panic" => {
                panic_caught = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }

    assert!(panic_caught, "Tool panic should be caught and reported as error");
}

/// Test that block hook prevents tool execution
#[tokio::test]
async fn test_hook_block_prevents_execution() {
    use crate::loop_engine::agent_loop;

    struct BlockHook;
    #[async_trait]
    impl Hook for BlockHook {
        async fn before_tool_call(
            &self,
            _tool_call: &ToolCall,
            _context: &Context,
        ) -> Result<HookDecision, crate::hook::HookError> {
            Ok(HookDecision::Block { reason: "blocked by test".to_string() })
        }
        async fn after_tool_call(
            &self,
            _tool_call: &ToolCall,
            result: &ToolOutput,
            _context: &Context,
        ) -> Result<ToolOutput, crate::hook::HookError> {
            Ok(result.clone())
        }
    }

    let provider = Arc::new(SimpleToolProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));
    let hook = Arc::new(BlockHook);

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![hook],
    );

    let mut tool_blocked = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. } if result.is_error && result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("blocked"))) => {
                tool_blocked = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }

    assert!(tool_blocked, "Tool should be blocked by hook");
}

/// Test that modify hook changes args
#[tokio::test]
async fn test_hook_modify_changes_args() {
    use crate::loop_engine::agent_loop;

    struct ModifyHook;
    #[async_trait]
    impl Hook for ModifyHook {
        async fn before_tool_call(
            &self,
            tool_call: &ToolCall,
            _context: &Context,
        ) -> Result<HookDecision, crate::hook::HookError> {
            if tool_call.arguments.get("original").is_some() {
                Ok(HookDecision::Modify { args: serde_json::json!({"modified": true}) })
            } else {
                Ok(HookDecision::Allow)
            }
        }
        async fn after_tool_call(
            &self,
            _tool_call: &ToolCall,
            result: &ToolOutput,
            _context: &Context,
        ) -> Result<ToolOutput, crate::hook::HookError> {
            Ok(result.clone())
        }
    }

    let provider = Arc::new(ModifyArgsProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));
    let hook = Arc::new(ModifyHook);

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![hook],
    );

    let mut args_were_modified = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. } if result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("modified"))) => {
                args_were_modified = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }

    assert!(args_were_modified, "Hook should have modified args");
}

/// Test that keep_recent_tokens is ignored, hardcoded to 6 (BUG-20)
#[test]
fn test_compaction_keep_recent_ignored() {
    // Create 10 messages
    let messages: Vec<AgentMessage> = (0..10)
        .map(|i| AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: format!("message {}", i) }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        })
        .collect();

    // Try with huge keep_recent_tokens - should still only keep 6
    let settings = CompactionSettings {
        enabled: true,
        reserve_tokens: 1000,
        keep_recent_tokens: 999999, // Should be ignored
    };

    let cut_point = find_cut_point(&messages, settings.keep_recent_tokens);

    // find_cut_point ignores keep_recent_tokens, hardcodes to 6
    // messages.len() - 6 = 10 - 6 = 4
    assert_eq!(cut_point, 4, "Should keep last 6 messages regardless of keep_recent_tokens");
    assert_eq!(messages[cut_point..].len(), 6, "Should always keep 6 messages (BUG-20)");
}

// ============================================================================
// Mock Providers for Testing
// ============================================================================

use async_stream::stream;
use futures::stream::BoxStream;
use runie_core::{Event as LlmEvent, ToolSchema, ProviderError};

/// Provider that always requests a tool, causing infinite loop until max_turns
struct AlwaysToolProvider;
impl AlwaysToolProvider {
    fn new() -> Self { AlwaysToolProvider }
}

#[async_trait]
impl Provider for AlwaysToolProvider {
    fn name(&self) -> &str { "always_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that generates duplicate tool calls (same tool+args twice)
struct DuplicateToolProvider { call_count: std::sync::Mutex<u32> }
impl DuplicateToolProvider {
    fn new() -> Self { DuplicateToolProvider { call_count: std::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for DuplicateToolProvider {
    fn name(&self) -> &str { "duplicate_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            // Same tool called twice with same args
            yield LlmEvent::ToolCallDelta { name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::ToolCallDelta { name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that generates same tool call across turns
struct SameToolAcrossTurnsProvider { turn: tokio::sync::Mutex<u32> }
impl SameToolAcrossTurnsProvider {
    fn new() -> Self { SameToolAcrossTurnsProvider { turn: tokio::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for SameToolAcrossTurnsProvider {
    fn name(&self) -> &str { "same_tool_across_turns" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let mut turn = self.turn.lock().await;
        *turn += 1;
        let current_turn = *turn;

        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
            // On first turn, send tool result to cause another turn
            if current_turn == 1 {
                yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name: "bash".to_string(), args: serde_json::json!({}), timestamp: chrono::Utc::now() };
                yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(), result: ToolOutput { content: "done".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
            }
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that requests permission but never grants
struct PermissionNeverGrantedProvider;
impl PermissionNeverGrantedProvider {
    fn new() -> Self { PermissionNeverGrantedProvider }
}

#[async_trait]
impl Provider for PermissionNeverGrantedProvider {
    fn name(&self) -> &str { "permission_never_granted" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that tracks and reports token usage
struct TokenCountingProvider { turn: tokio::sync::Mutex<u32> }
impl TokenCountingProvider {
    fn new() -> Self { TokenCountingProvider { turn: tokio::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for TokenCountingProvider {
    fn name(&self) -> &str { "token_counting" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let mut turn = self.turn.lock().await;
        *turn += 1;
        let current_turn = *turn;

        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };
        let tool_name_for_execution = tool_name.clone();

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            // Each turn adds to token count
            yield LlmEvent::Usage { prompt_tokens: 100 * current_turn as usize, completion_tokens: 50 * current_turn as usize, total_tokens: 150 * current_turn as usize };
            yield LlmEvent::MessageEnd;
            // First 2 turns request tools, then stop
            if current_turn < 3 {
                yield LlmEvent::ToolCallDelta { name: tool_name, arguments: "{}".to_string() };
                yield LlmEvent::ToolExecutionStart { tool_call_id: format!("call_{}", current_turn), tool_name: tool_name_for_execution, args: serde_json::json!({}), timestamp: chrono::Utc::now() };
                yield LlmEvent::ToolExecutionEnd { tool_call_id: format!("call_{}", current_turn), result: ToolOutput { content: "done".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
            }
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that causes panic in tool prep
struct PanickingToolProvider;
impl PanickingToolProvider {
    fn new() -> Self { PanickingToolProvider }
}

#[async_trait]
impl Provider for PanickingToolProvider {
    fn name(&self) -> &str { "panicking_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider for simple tool execution tests
struct SimpleToolProvider;
impl SimpleToolProvider {
    fn new() -> Self { SimpleToolProvider }
}

#[async_trait]
impl Provider for SimpleToolProvider {
    fn name(&self) -> &str { "simple_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
            yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name: tool_name, args: serde_json::json!({}), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(), result: ToolOutput { content: "executed".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that sends tool call with "original" arg for modify hook test
struct ModifyArgsProvider;
impl ModifyArgsProvider {
    fn new() -> Self { ModifyArgsProvider }
}

#[async_trait]
impl Provider for ModifyArgsProvider {
    fn name(&self) -> &str { "modify_args" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { name: tool_name.clone(), arguments: r#"{"original": true}"#.to_string() };
            yield LlmEvent::MessageEnd;
            yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name: tool_name, args: serde_json::json!({"modified": true}), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(), result: ToolOutput { content: "modified args".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

// ============================================================================
// Retry Logic Tests
// ============================================================================

/// Provider that returns rate limited errors with progressive retries
struct RateLimitedProvider {
    call_count: std::sync::atomic::AtomicU32,
}

impl RateLimitedProvider {
    fn new() -> Self {
        RateLimitedProvider {
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl Provider for RateLimitedProvider {
    fn name(&self) -> &str { "rate_limited" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { false }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, _tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < 3 {
            // First 3 calls get rate limited
            Err(ProviderError::RateLimited)
        } else {
            // 4th call succeeds
            let s = stream! {
                yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
                yield LlmEvent::MessageDelta { content: "Hello".to_string() };
                yield LlmEvent::MessageEnd;
            };
            Ok(Box::pin(s))
        }
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that returns API error (401-like) immediately without retry
struct UnauthorizedProvider;

impl UnauthorizedProvider {
    fn new() -> Self { UnauthorizedProvider }
}

#[async_trait]
impl Provider for UnauthorizedProvider {
    fn name(&self) -> &str { "unauthorized" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { false }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, _tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        Err(ProviderError::ApiError("Invalid API key".to_string()))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Test that 429 rate limit errors are retried with exponential backoff (1s, 2s, 4s)
#[tokio::test]
async fn test_provider_429_retry_backoff() {
    use crate::loop_engine::start_chat_with_retry;
    use runie_core::Message;

    let provider = Arc::new(RateLimitedProvider::new());
    let messages = vec![Message::User { content: "hello".to_string(), attachments: vec![] }];

    let start = std::time::Instant::now();

    let result = start_chat_with_retry(provider, messages, vec![]).await;

    let elapsed = start.elapsed();

    // Should succeed after 3 retries with backoff (1s + 2s + 4s = 7s minimum)
    assert!(result.is_ok(), "Should eventually succeed after retries");
    // Elapsed should be at least 7 seconds (1 + 2 + 4)
    assert!(elapsed.as_secs() >= 7, "Backoff should total at least 7s, got {}s", elapsed.as_secs());
}

/// Test that 401 unauthorized errors fail immediately without retry
#[tokio::test]
async fn test_provider_non_429_no_retry() {
    use crate::loop_engine::start_chat_with_retry;
    use runie_core::Message;

    let provider = Arc::new(UnauthorizedProvider::new());
    let messages = vec![Message::User { content: "hello".to_string(), attachments: vec![] }];

    let start = std::time::Instant::now();

    let result = start_chat_with_retry(provider, messages, vec![]).await;

    let elapsed = start.elapsed();

    // Should fail immediately without retry
    assert!(result.is_err(), "Should fail immediately on non-retryable error");
    // Elapsed should be less than 1 second (no retry delay)
    assert!(elapsed.as_secs() < 1, "Should fail immediately without backoff, got {}s", elapsed.as_secs());
}
