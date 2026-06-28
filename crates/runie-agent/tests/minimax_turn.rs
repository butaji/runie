//! Replay captured MiniMax streams through the full agent turn with mocked IO.

use runie_agent::{run_agent_turn_with_skills, AgentCommand};
use runie_core::model::ThinkingLevel;
use runie_core::Event;
use runie_testing::fixtures::minimax as fixtures;
use runie_testing::{allow_all_gate, capture_events, dyn_replay_provider, mock_tool_skill_minimax};

fn command(content: &str) -> AgentCommand {
    AgentCommand {
        content: content.to_string(),
        id: "req.0".to_string(),
        provider: "minimax".to_string(),
        model: "MiniMax-M3".to_string(),
        thinking_level: ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: runie_agent::truncate::TruncationPolicy::default(),
    }
}

fn minimax_replay(fixture_names: &[&str]) -> runie_provider::DynProvider {
    let fixture_strs: Vec<String> = fixture_names
        .iter()
        .map(|n| fixtures::fixture(n))
        .collect();
    dyn_replay_provider(&fixture_strs)
}

#[tokio::test]
async fn m3_list_files_turn_executes_list_dir() {
    let provider = minimax_replay(&["m3_list_files_call.sse", "m3_list_files_final.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("list files in the current directory"),
        emit,
        5,
        Some(&mock_tool_skill_minimax()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::ToolStart { name, .. } if name == "list_dir"
    )));
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}

#[tokio::test]
async fn m3_read_file_turn_executes_read_file() {
    let provider = minimax_replay(&["m3_read_file_call.sse", "m3_read_file_final.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("read README.md"),
        emit,
        5,
        Some(&mock_tool_skill_minimax()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::ToolStart { name, .. } if name == "read_file"
    )));
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}

#[tokio::test]
async fn m3_multi_tool_turn_executes_list_dir_and_read_file() {
    let provider =
        minimax_replay(&["m3_multi_tool_list_dir.sse", "m3_multi_tool_readme.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("list files and read README.md"),
        emit,
        5,
        Some(&mock_tool_skill_minimax()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    let tool_names: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::ToolStart { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(tool_names.contains(&"list_dir"));
    assert!(tool_names.contains(&"read_file"));
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}

#[tokio::test]
async fn m27_multi_tool_turn_executes_read_file() {
    let provider = minimax_replay(&["m27_multi_tool_readme.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("read README.md"),
        emit,
        5,
        Some(&mock_tool_skill_minimax()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::ToolStart { name, .. } if name == "read_file"
    )));
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}
