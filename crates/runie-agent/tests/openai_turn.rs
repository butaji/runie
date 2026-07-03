//! Replay captured OpenAI SSE streams through the full agent turn with mocked IO.

use std::collections::HashMap;

use runie_agent::agent_command_builder::agent_cmd;
use runie_agent::run_agent_turn_with_skills;
use runie_core::Event;
use runie_testing::fixtures::openai as fixtures;
use runie_testing::{allow_all_gate, capture_events, dyn_replay_provider_with, MockToolSkill};

/// Build a skill registry with canned outputs for list_dir, read_file, and bash.
fn openai_mock_skill() -> runie_core::harness_skills::SkillRegistry {
    let mut outputs = HashMap::new();
    outputs.insert(
        "list_dir".to_string(),
        "Cargo.toml\nREADME.md\n".to_string(),
    );
    outputs.insert(
        "read_file".to_string(),
        "# Runie\n\nA terminal AI assistant.".to_string(),
    );
    outputs.insert("bash".to_string(), "hello\n".to_string());
    let mut registry = runie_core::harness_skills::SkillRegistry::new();
    registry.register(MockToolSkill::new(outputs));
    registry
}

fn command(content: &str) -> runie_agent::AgentCommand {
    agent_cmd(content)
        .provider("openai")
        .model("gpt-4o-mini")
        .build()
}

fn openai_replay(fixture_names: &[&str]) -> runie_provider::BuiltProvider {
    let fixture_strs: Vec<String> = fixture_names.iter().map(|n| fixtures::fixture(n)).collect();
    dyn_replay_provider_with(&fixture_strs, "openai", "gpt-4o-mini")
}

#[tokio::test]
async fn simple_text_turn_completes_without_tools() {
    let provider = openai_replay(&["simple_text_delta.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("say hello"),
        emit,
        5,
        None,
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock();
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}

#[tokio::test]
async fn parallel_tool_turn_executes_list_dir_and_read_file() {
    let provider = openai_replay(&["parallel_tool_calls.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &command("check the directory and read the config"),
        emit,
        5,
        Some(&openai_mock_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let events = events.lock();
    let tool_names: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::ToolStart { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        tool_names.contains(&"list_dir"),
        "expected list_dir in tools, got {:?}",
        tool_names
    );
    assert!(
        tool_names.contains(&"read_file"),
        "expected read_file in tools, got {:?}",
        tool_names
    );
    assert!(events.iter().any(|e| matches!(e, Event::Done { .. })));
}
