use crate::{AgentCommand, AgentEvent, run_agent_turn};

#[tokio::test]
async fn test_agent_loop_simple_response() {
    let cmd = AgentCommand {
        content: "Hello World".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5,
    ).await.unwrap();

    let thinking = events.iter().filter(|e| matches!(e, AgentEvent::Thinking { .. })).count();
    let responses = events.iter().filter(|e| matches!(e, AgentEvent::Response { .. })).count();
    let done = events.iter().filter(|e| matches!(e, AgentEvent::Done { .. })).count();

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
    run_agent_turn(&cmd, |evt| events.push(evt), 5,
    ).await.unwrap();

    let tool_starts = events.iter().filter(|e| matches!(e, AgentEvent::ToolStart { .. })).count();
    let tool_ends = events.iter().filter(|e| matches!(e, AgentEvent::ToolEnd { .. })).count();
    let completes = events.iter().filter(|e| matches!(e, AgentEvent::TurnComplete { .. })).count();

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
    run_agent_turn(&cmd, |evt| events.push(evt), 3,
    ).await.unwrap();
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
    run_agent_turn(&cmd, |evt| events.push(evt), 5,
    ).await.unwrap();

    for evt in &events {
        let evt_id = match evt {
            AgentEvent::Thinking { id } => id,
            AgentEvent::ThoughtDone { id } => id,
            AgentEvent::ToolStart { id, .. } => id,
            AgentEvent::Response { id, .. } => id,
            AgentEvent::TurnComplete { id, .. } => id,
            AgentEvent::Done { id } => id,
            AgentEvent::Error { id, .. } => id,
            _ => continue,
        };
        assert_eq!(evt_id, "req.42");
    }
}

#[test]
fn test_agent_command_structure() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
    };
    assert_eq!(cmd.content, "test");
    assert_eq!(cmd.id, "req.0");
}
