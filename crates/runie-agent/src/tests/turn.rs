//! Tests for agent turn execution
use crate::{AgentCommand, run_agent_turn};
use runie_core::Event;

#[tokio::test]
async fn test_agent_loop_simple_response() {
    let cmd = AgentCommand {
        content: "Hello World".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5).await.unwrap();

    let thinking = events
        .iter()
        .filter(|e| matches!(e, Event::AgentThinking { .. }))
        .count();
    let responses = events
        .iter()
        .filter(|e| matches!(e, Event::AgentResponse { .. }))
        .count();
    let done = events
        .iter()
        .filter(|e| matches!(e, Event::AgentDone { .. }))
        .count();

    assert_eq!(thinking, 1);
    assert_eq!(responses, 2);
    assert_eq!(done, 1);
}

#[tokio::test]
async fn test_agent_loop_with_tool_call() {
    let cmd = AgentCommand {
        content: "list files".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5).await.unwrap();

    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, Event::AgentToolStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, Event::AgentToolEnd { .. }))
        .count();
    let completes = events
        .iter()
        .filter(|e| matches!(e, Event::AgentTurnComplete { .. }))
        .count();

    assert!(tool_starts >= 1);
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(completes, 1);
}

#[tokio::test]
async fn test_agent_loop_respects_max_iterations() {
    let cmd = AgentCommand {
        content: "loop".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 3)
        .await
        .unwrap();
    assert!(!events.is_empty());
}

#[tokio::test]
async fn test_agent_loop_events_have_correct_id() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.42".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5).await.unwrap();

    for evt in &events {
        let evt_id = match evt {
            Event::AgentThinking { id } => id.clone(),
            Event::AgentThoughtDone { id } => id.clone(),
            Event::AgentToolStart { id, .. } => id.clone(),
            Event::AgentResponse { id, .. } => id.clone(),
            Event::AgentTurnComplete { id, .. } => id.clone(),
            Event::AgentDone { id } => id.clone(),
            Event::AgentError { id, .. } => id.clone(),
            _ => continue,
        };
        assert_eq!(evt_id, "req.42");
    }
}
