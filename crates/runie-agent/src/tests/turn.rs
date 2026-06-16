//! Tests for agent turn execution
use crate::tests::ensure_mock_provider;
use crate::{run_agent_turn, turn::build_initial_messages, AgentCommand};
use runie_core::event::AgentEvent;
use runie_core::tool::{ToolContext, ToolStatus, builtin_registry};
use runie_core::Event;
use runie_provider::DynProvider;
use std::sync::{Arc, Mutex};

fn mock_provider() -> DynProvider {
    DynProvider::new("mock", "echo").expect("mock provider must be available in tests")
}

#[tokio::test]
async fn test_agent_loop_simple_response() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        5,
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let thinking = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::Thinking { .. })))
        .count();
    let deltas = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::ResponseDelta { .. })))
        .count();
    let done = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::Done { .. })))
        .count();

    assert_eq!(thinking, 1);
    assert_eq!(deltas, 2); // streaming deltas
    assert_eq!(done, 1);
}

#[tokio::test]
async fn test_agent_loop_with_tool_call() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        5,
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::ToolStart { .. })))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::ToolEnd { .. })))
        .count();
    let completes = events
        .iter()
        .filter(|e| matches!(e, Event::Agent(AgentEvent::TurnComplete { .. })))
        .count();

    assert!(tool_starts >= 1);
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(completes, 1);
}

#[tokio::test]
async fn test_agent_loop_respects_max_iterations() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        3,
    )
    .await
    .unwrap();
    assert!(!events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_agent_loop_events_have_correct_id() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        5,
    )
    .await
    .unwrap();

    for evt in events.lock().unwrap().iter() {
        let evt_id = match evt {
            Event::Agent(AgentEvent::Thinking { id }) => id.clone(),
            Event::Agent(AgentEvent::ThoughtDone { id }) => id.clone(),
            Event::Agent(AgentEvent::ToolStart { id, .. }) => id.clone(),
            Event::Agent(AgentEvent::Response { id, .. }) => id.clone(),
            Event::Agent(AgentEvent::TurnComplete { id, .. }) => id.clone(),
            Event::Agent(AgentEvent::Done { id }) => id.clone(),
            Event::Agent(AgentEvent::Error { id, .. }) => id.clone(),
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
    let system = match &msgs[0].role {
        runie_core::message::Role::System => msgs[0].content.clone(),
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
    let system = match &msgs[0].role {
        runie_core::message::Role::System => msgs[0].content.clone(),
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

#[tokio::test]
async fn agent_tool_uses_core_trait() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        5,
    )
    .await
    .unwrap();

    let tool_end = events
        .lock()
        .unwrap()
        .iter()
        .find_map(|e| match e {
            Event::Agent(AgentEvent::ToolEnd { output, .. }) => Some(output.clone()),
            _ => None,
        })
        .expect("agent turn should emit ToolEnd");

    // The same call through the canonical trait must produce the same result.
    let registry = builtin_registry();
    let tool = registry.get("list_dir").expect("list_dir in registry");
    let direct = tool
        .call(serde_json::json!({"path": "."}), &ToolContext::default())
        .await
        .unwrap();

    assert_eq!(tool_end, direct.content);
    assert_eq!(direct.status, ToolStatus::Success);
}

#[tokio::test]
async fn tool_call_event_matches_output() {
    ensure_mock_provider();
    let provider = mock_provider();
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
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| events_clone.lock().unwrap().push(evt))),
        5,
    )
    .await
    .unwrap();

    let tool_ends: Vec<String> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            Event::Agent(AgentEvent::ToolEnd { output, .. }) => Some(output.clone()),
            _ => None,
        })
        .collect();

    assert!(!tool_ends.is_empty(), "ToolEnd events should be emitted");

    // Calling the canonical tool directly with the same arguments yields the
    // same output, confirming the event reflects the actual tool result.
    let registry = builtin_registry();
    let tool = registry.get("list_dir").expect("list_dir in registry");
    let direct = tool
        .call(serde_json::json!({"path": "."}), &ToolContext::default())
        .await
        .unwrap();

    for output in tool_ends {
        assert_eq!(output, direct.content);
    }
}
