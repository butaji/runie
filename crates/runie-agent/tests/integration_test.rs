use runie_agent::loop_engine::{run_agent_loop, AgentLoopConfig};
use runie_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::providers::MockProvider;
use runie_tools::{create_default_toolkit, Workspace};
use std::path::PathBuf;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_agent_end_to_end() {
    let provider = MockProvider::new();
    let ws = Workspace::new(PathBuf::from("."));
    let registry = std::sync::Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are a helpful assistant.".to_string(),
        model: "mock".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 3,
    };

    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (perm_tx, mut perm_rx) = mpsc::channel(100);

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: "hello".to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    // Spawn the agent loop
    let handle = tokio::spawn(async move {
        run_agent_loop(
            messages,
            config,
            &provider,
            &[],
            event_tx,
            perm_rx,
            Some(registry),
            vec![],
        )
        .await
        .unwrap();
    });

    // Collect events
    let mut got_message_start = false;
    let mut got_message_update = false;
    let mut got_message_end = false;
    let mut got_agent_end = false;

    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::MessageStart { .. } => got_message_start = true,
            AgentEvent::MessageUpdate { .. } => got_message_update = true,
            AgentEvent::MessageEnd { .. } => got_message_end = true,
            AgentEvent::AgentEnd { .. } => {
                got_agent_end = true;
                break;
            }
            AgentEvent::PermissionRequest { tool_call_id, .. } => {
                // Auto-allow permission requests for testing
                let _ = perm_tx.send(PermissionDecision::Allow { tool_call_id });
            }
            _ => {}
        }
    }

    handle.await.ok();

    assert!(got_message_start, "Should receive MessageStart");
    assert!(got_message_update, "Should receive MessageUpdate");
    assert!(got_message_end, "Should receive MessageEnd");
    assert!(got_agent_end, "Should receive AgentEnd");
}

#[tokio::test]
async fn test_agent_with_mock_error_simulation() {
    // Create a mock provider that simulates errors
    let provider = MockProvider::new().with_errors(0.0); // 0% error rate, should succeed

    let ws = Workspace::new(PathBuf::from("/tmp"));
    let registry = std::sync::Arc::new(create_default_toolkit(ws));

    let config = AgentLoopConfig {
        system_prompt: "You are a helpful assistant.".to_string(),
        model: "mock".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let (event_tx, mut event_rx) = mpsc::channel(100);
    let (_perm_tx, perm_rx) = mpsc::channel(100);

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text {
            text: "hi".to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
    }];

    tokio::spawn(async move {
        run_agent_loop(
            messages,
            config,
            &provider,
            &[],
            event_tx,
            perm_rx,
            Some(registry),
            vec![],
        )
        .await
        .unwrap();
    });

    // Should complete without errors
    while let Some(event) = event_rx.recv().await {
        if let AgentEvent::AgentEnd { .. } = event {
            return;
        }
        if let AgentEvent::Error { .. } = event {
            panic!("Should not receive error event");
        }
    }
    panic!("Should have received AgentEnd");
}
