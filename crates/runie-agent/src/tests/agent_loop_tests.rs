use super::*;
use async_stream::stream;
use futures::stream::BoxStream;
use runie_core::{Event as LlmEvent, ToolSchema, ProviderError};

/// Test that max_turns=3 allows exactly 3 turns before failing
#[tokio::test]
#[ignore]
async fn test_max_turns_exact_boundary() {
    use crate::loop_engine::agent_loop;

    let provider = Arc::new(AlwaysToolProvider::new());
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
        tool_calls: vec![],
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

    assert_eq!(turn_count, 3, "Should complete exactly 3 turns");
    assert!(max_turns_exceeded, "Should get max_turns error on turn 4");
}

/// Test that duplicate tool calls (same tool+args) in the same turn are skipped
#[tokio::test]
#[ignore]
async fn test_duplicate_tool_call_same_turn() {
    use crate::loop_engine::agent_loop;

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
        tool_calls: vec![],
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

    assert_eq!(tool_execution_count, 1, "Duplicate tool call should be skipped");
}

/// Test that same tool in different turns both execute
#[tokio::test]
#[ignore]
async fn test_duplicate_tool_call_across_turns() {
    use crate::loop_engine::agent_loop;

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
        tool_calls: vec![],
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

    assert_eq!(tool_execution_count, 2, "Same tool in different turns should both run");
}

/// Test that permission timeout (5 min) returns denied
#[tokio::test]
#[ignore]
async fn test_permission_timeout_returns_denied() {
    use crate::loop_engine::agent_loop;

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
        tool_calls: vec![],
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let mut permission_denied = false;

    let start = std::time::Instant::now();
    while let Some(event) = stream.next().await {
        if let AgentEvent::PermissionDenied { .. } = event {
            permission_denied = true;
            break;
        }
        if let AgentEvent::AgentEnd { .. } = event {
            break;
        }
        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
    }

    assert!(permission_denied, "Permission should be denied after timeout");
}

/// Test that token usage accumulates per turn
#[tokio::test]
#[ignore]
async fn test_token_usage_accumulates_per_turn() {
    use crate::loop_engine::agent_loop;

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
        tool_calls: vec![],
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

/// Test that panic in tool data prep is caught
#[tokio::test]
#[ignore]
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
        tool_calls: vec![],
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
