//! Tests for agent turn execution
use crate::{run_agent_turn, turn::build_initial_messages, AgentCommand};
use runie_core::Event;

#[tokio::test]
async fn test_agent_loop_simple_response() {
    let cmd = AgentCommand {
        content: "Hello World".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5)
        .await
        .unwrap();

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
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5)
        .await
        .unwrap();

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
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
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
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let mut events = Vec::new();
    run_agent_turn(&cmd, |evt| events.push(evt), 5)
        .await
        .unwrap();

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

#[test]
fn read_only_excludes_write_tools() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.0".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: true,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let msgs = build_initial_messages(&cmd);
    let system = match &msgs[0] {
        runie_core::Message::System { content } => content.clone(),
        _ => panic!("expected system message"),
    };
    assert!(system.contains("read_file"), "read-only includes read_file");
    assert!(system.contains("list_dir"), "read-only includes list_dir");
    assert!(system.contains("grep"), "read-only includes grep");
    assert!(system.contains("find"), "read-only includes find");
    assert!(
        !system.contains("write_file"),
        "read-only excludes write_file"
    );
    assert!(
        !system.contains("edit_file"),
        "read-only excludes edit_file"
    );
    assert!(!system.contains("bash"), "read-only excludes bash");
}

#[test]
fn read_write_includes_all_tools() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.1".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let msgs = build_initial_messages(&cmd);
    let system = match &msgs[0] {
        runie_core::Message::System { content } => content.clone(),
        _ => panic!("expected system message"),
    };
    assert!(
        system.contains("write_file"),
        "read-write includes write_file"
    );
    assert!(
        system.contains("edit_file"),
        "read-write includes edit_file"
    );
    assert!(system.contains("bash"), "read-write includes bash");
}
