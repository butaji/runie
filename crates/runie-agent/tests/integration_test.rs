use futures::StreamExt;
use runie_agent::loop_engine::{agent_loop, AgentLoopConfig, AgentEventStream};
use runie_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::providers::MockProvider;
use runie_tools::{create_default_toolkit, Workspace};
use std::path::PathBuf;
use std::sync::Arc;

fn setup_agent_test() -> (
    Arc<MockProvider>,
    Arc<runie_tools::ToolRegistry>,
    AgentLoopConfig,
    Vec<AgentMessage>,
) {
    let provider = Arc::new(MockProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are a helpful assistant.".to_string(),
        model: "mock".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 3,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: "hello".to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }];

    (provider, registry, config, messages)
}

async fn collect_agent_events(stream: &mut AgentEventStream) -> (bool, bool, bool, bool) {
    let mut got_message_start = false;
    let mut got_message_update = false;
    let mut got_message_end = false;
    let mut got_agent_end = false;

    while let Some(event) = stream.next().await {
        match event {
            AgentEvent::MessageStart { .. } => got_message_start = true,
            AgentEvent::MessageUpdate { .. } => got_message_update = true,
            AgentEvent::MessageEnd { .. } => got_message_end = true,
            AgentEvent::AgentEnd { .. } => {
                got_agent_end = true;
                break;
            }
            AgentEvent::PermissionRequest { tool_call_id, tool_name, tool_args, .. } => {
                let _ = stream.send_permission(PermissionDecision::Allow { tool_call_id, tool_name, tool_args }).await;
            }
            _ => {}
        }
    }

    (got_message_start, got_message_update, got_message_end, got_agent_end)
}

#[tokio::test]
async fn test_agent_end_to_end() {
    let (provider, registry, config, messages) = setup_agent_test();

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    let (got_message_start, got_message_update, got_message_end, got_agent_end) =
        collect_agent_events(&mut stream).await;

    let _final_messages = stream.result().await;

    assert!(got_message_start, "Should receive MessageStart");
    assert!(got_message_update, "Should receive MessageUpdate");
    assert!(got_message_end, "Should receive MessageEnd");
    assert!(got_agent_end, "Should receive AgentEnd");
}

fn setup_error_test() -> (
    Arc<MockProvider>,
    Arc<runie_tools::ToolRegistry>,
    AgentLoopConfig,
    Vec<AgentMessage>,
) {
    let provider = Arc::new(MockProvider::new().with_errors(0.0));
    let ws = Workspace::new(PathBuf::from("/tmp"));
    let registry = Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are a helpful assistant.".to_string(),
        model: "mock".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: "hi".to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }];

    (provider, registry, config, messages)
}

#[tokio::test]
async fn test_agent_with_mock_error_simulation() {
    let (provider, registry, config, messages) = setup_error_test();

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![],
    );

    while let Some(event) = stream.next().await {
        if let AgentEvent::AgentEnd { .. } = event {
            let _final_messages = stream.result().await;
            return;
        }
        if let AgentEvent::Error { .. } = event {
            panic!("Should not receive error event");
        }
    }
    panic!("Should have received AgentEnd");
}
