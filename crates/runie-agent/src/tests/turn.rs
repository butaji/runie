//! Tests for agent turn execution
use crate::tests::ensure_mock_provider;
use crate::{
    run_agent_turn, run_agent_turn_with_skills, turn::build_initial_messages, AgentCommand,
};
use runie_core::harness_skills::{HarnessSkill, SkillRegistry, ToolCallCtx, ToolCallPhase};
use runie_core::Event;
use runie_testing::mock_tool_skill;
use runie_testing::{allow_all_gate, mock_provider, RecordingSkill};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_agent_loop_simple_response() {
    let _mock_guard = ensure_mock_provider().await;
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
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        5,
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let thinking = events
        .iter()
        .filter(|e| matches!(e, Event::Thinking { .. }))
        .count();
    let deltas = events
        .iter()
        .filter(|e| matches!(e, Event::ResponseDelta { .. }))
        .count();
    let done = events
        .iter()
        .filter(|e| matches!(e, Event::Done { .. }))
        .count();

    let responses = events
        .iter()
        .filter(|e| matches!(e, Event::Response { .. }))
        .count();

    assert_eq!(thinking, 1);
    assert_eq!(deltas, 2); // streaming deltas
    assert_eq!(responses, 0); // full-text Response is not emitted; deltas already streamed it
    assert_eq!(done, 1);
}

#[tokio::test]
async fn test_agent_loop_with_tool_call() {
    let _mock_guard = ensure_mock_provider().await;
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
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, Event::ToolStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, Event::ToolEnd { .. }))
        .count();
    let completes = events
        .iter()
        .filter(|e| matches!(e, Event::TurnComplete { .. }))
        .count();

    assert!(tool_starts >= 1);
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(completes, 1);
}

#[tokio::test]
async fn test_agent_loop_with_native_tool_call_events() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
    let cmd = AgentCommand {
        content: "run native tool".to_string(),
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
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        1,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, Event::ToolStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, Event::ToolEnd { .. }))
        .count();
    let bash_calls = events
        .iter()
        .filter(|e| matches!(e, Event::ToolStart { name, .. } if name == "bash"))
        .count();

    assert_eq!(tool_starts, 1, "expected one tool start");
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(bash_calls, 1, "expected bash tool from native events");
}

#[tokio::test]
async fn test_agent_loop_respects_max_iterations() {
    let _mock_guard = ensure_mock_provider().await;
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
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        3,
        allow_all_gate(),
    )
    .await
    .unwrap();
    assert!(!events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_agent_loop_events_have_correct_id() {
    let _mock_guard = ensure_mock_provider().await;
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
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        5,
        allow_all_gate(),
    )
    .await
    .unwrap();

    for evt in events.lock().unwrap().iter() {
        let evt_id = match evt {
            Event::Thinking { id } => id.clone(),
            Event::ThoughtDone { id } => id.clone(),
            Event::ToolStart { id, .. } => id.clone(),
            Event::Response { id, .. } => id.clone(),
            Event::TurnComplete { id, .. } => id.clone(),
            Event::Done { id } => id.clone(),
            Event::Error { id, .. } => id.clone(),
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
        runie_core::message::Role::System => msgs[0].content().clone(),
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
        runie_core::message::Role::System => msgs[0].content().clone(),
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
async fn agent_tool_event_carries_mock_output() {
    let _mock_guard = ensure_mock_provider().await;
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
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_end = events
        .lock()
        .unwrap()
        .iter()
        .find_map(|e| match e {
            Event::ToolEnd { output, .. } => Some(output.clone()),
            _ => None,
        })
        .expect("agent turn should emit ToolEnd");

    assert_eq!(tool_end, "Cargo.toml\nREADME.md\n");
}

#[tokio::test]
async fn tool_call_event_matches_mock_output() {
    let _mock_guard = ensure_mock_provider().await;
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
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().unwrap().push(evt)
        })),
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_ends: Vec<String> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            Event::ToolEnd { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();

    assert!(!tool_ends.is_empty(), "ToolEnd events should be emitted");

    for output in tool_ends {
        assert_eq!(output, "Cargo.toml\nREADME.md\n");
    }
}

#[test]
fn tool_call_hook_receives_input() {
    let skill = RecordingSkill::new();
    let ctx_ref = skill.ctx.clone();
    let mut registry = SkillRegistry::new();
    registry.register(skill);

    let input = serde_json::json!({"path": "src/main.rs"});
    registry.on_tool_call(&ToolCallCtx {
        tool_name: "read_file".into(),
        tool_input: input.clone(),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    });

    let ctx = ctx_ref.lock().unwrap().take().unwrap();
    assert_eq!(ctx.tool_input, input);
}
