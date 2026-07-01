//! Tests for agent turn execution
use crate::tests::ensure_mock_provider;
use crate::{
    agent_command_builder::agent_cmd, run_agent_turn, run_agent_turn_with_skills,
    turn::build_initial_messages,
};
use anyhow::Result;
use runie_core::event::Event;
use runie_core::harness_skills::{SkillRegistry, ToolCallCtx, ToolCallPhase};
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::ProviderEvent;
use runie_testing::event_helpers::{count_events, find_event};
use runie_testing::mock_tool_skill;
use runie_testing::{allow_all_gate, capture_events, mock_provider, RecordingSkill};

/// Layer 2: Verify a single-word "hello" produces exactly one ResponseDelta
/// and the turn completes without repetition.
///
/// Regression test for the TUI mock echo-loop bug: UiActor was calling
/// `run_if_queued` redundantly from the Done handler, causing the agent to be
/// started twice per turn.
#[tokio::test]
async fn test_agent_loop_single_word_echo_completes_once() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("hello").build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate())
        .await
        .unwrap();

    let deltas: Vec<_> = events
        .lock()
        .iter()
        .filter_map(|e| match e {
            Event::ResponseDelta { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();
    let done_count = count_events(&events, |e| matches!(e, Event::Done { .. }));

    // Mock provider: "hello" splits to ["hello "] → exactly 1 delta.
    assert_eq!(
        deltas.len(),
        1,
        "single-word 'hello' should produce exactly 1 ResponseDelta, got {deltas:?}"
    );
    assert_eq!(deltas[0].as_str(), "hello ");
    // Turn must complete exactly once.
    assert_eq!(done_count, 1, "exactly one Done event expected");
}

#[tokio::test]
async fn test_agent_loop_simple_response() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("Hello World").build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate())
        .await
        .unwrap();

    let thinking = count_events(&events, |e| matches!(e, Event::Thinking { .. }));
    let deltas = count_events(&events, |e| matches!(e, Event::ResponseDelta { .. }));
    let done = count_events(&events, |e| matches!(e, Event::Done { .. }));
    let responses = count_events(&events, |e| matches!(e, Event::Response { .. }));

    assert_eq!(thinking, 1);
    assert_eq!(deltas, 2); // streaming deltas
    assert_eq!(responses, 0); // full-text Response is not emitted; deltas already streamed it
    assert_eq!(done, 1);
}

#[tokio::test]
async fn test_agent_loop_with_tool_call() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("list files").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_starts = count_events(&events, |e| matches!(e, Event::ToolStart { .. }));
    let tool_ends = count_events(&events, |e| matches!(e, Event::ToolEnd { .. }));
    let completes = count_events(&events, |e| matches!(e, Event::TurnComplete { .. }));

    assert!(tool_starts >= 1);
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(completes, 1);
}

#[tokio::test]
async fn test_agent_loop_with_native_tool_call_events() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("run native tool").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        1,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_starts = count_events(&events, |e| matches!(e, Event::ToolStart { .. }));
    let tool_ends = count_events(&events, |e| matches!(e, Event::ToolEnd { .. }));
    let bash_calls = count_events(&events, |e| matches!(e, Event::ToolStart { name, .. } if name == "bash"));

    assert_eq!(tool_starts, 1, "expected one tool start");
    assert_eq!(tool_starts, tool_ends);
    assert_eq!(bash_calls, 1, "expected bash tool from native events");
}

#[tokio::test]
async fn test_agent_loop_respects_max_iterations() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("loop").build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 3, allow_all_gate())
        .await
        .unwrap();
    assert!(!events.lock().is_empty());
}

#[tokio::test]
async fn test_agent_loop_events_have_correct_id() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("test").id("req.42").build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate())
        .await
        .unwrap();

    for evt in events.lock().iter() {
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
        let cmd = agent_cmd("test").read_only(true).build();
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
        let cmd = agent_cmd("test").id("req.1").build();
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
        let cmd = agent_cmd("list files").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_end = find_event(&events, |e| matches!(e, Event::ToolEnd { .. }))
        .map(|e| {
            if let Event::ToolEnd { output, .. } = e {
                output.clone()
            } else {
                String::new()
            }
        })
        .expect("agent turn should emit ToolEnd");

    assert_eq!(tool_end, "Cargo.toml\nREADME.md\n");
}

#[tokio::test]
async fn tool_call_event_matches_mock_output() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("list files").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let tool_ends: Vec<String> = events
        .lock()
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

    let ctx = ctx_ref.lock().take().unwrap();
    assert_eq!(ctx.tool_input, input);
}

// ========================================================================
// Layer 1 — emit-turncomplete tests
// ========================================================================

/// Layer 1: A text-only turn (no tool calls) emits TurnComplete.
/// Previously TurnComplete was gated on `has_intermediate_steps`.
#[tokio::test]
async fn text_turn_emits_turn_complete() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("plain hello").id("req.text").build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate())
        .await
        .unwrap();

    let turn_complete_count = count_events(&events, |e| matches!(e, Event::TurnComplete { .. }));
    assert_eq!(turn_complete_count, 1, "text-only turn must emit TurnComplete");
    let done_count = count_events(&events, |e| matches!(e, Event::Done { .. }));
    assert_eq!(done_count, 1, "turn must emit Done");
}

/// Layer 1: A tool turn also emits TurnComplete (regression: had to exist before).
#[tokio::test]
async fn tool_turn_emits_turn_complete() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("list files").id("req.tools").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let turn_complete_count = count_events(&events, |e| matches!(e, Event::TurnComplete { .. }));
    assert_eq!(turn_complete_count, 1, "tool turn must emit TurnComplete");
}

// ========================================================================
// Layer 1 — emit-thoughtdone-on-stream-error tests
// ========================================================================

/// Layer 1: A provider stream error emits ThoughtDone before propagating the error.
struct ErrorProvider;

impl Provider for ErrorProvider {
    fn generate(
        &self,
        _: Vec<ChatMessage>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<ProviderEvent>> + Send + '_>> {
        let stream = futures::stream::iter([Err(anyhow::anyhow!("stream error"))]);
        Box::pin(stream)
    }
}

/// Layer 1: ThoughtDone is emitted even when the stream returns an error.
#[tokio::test]
async fn stream_error_emits_thought_done() {
    let provider = ErrorProvider;
        let cmd = agent_cmd("error").id("req.err").provider("error").model("error").build();
    let (events, emit) = capture_events();
    let result = run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate()).await;

    // Turn must return an error (provider failure).
    assert!(result.is_err(), "stream error should propagate as Err");

    // ThoughtDone must be emitted even on error path.
    let thought_done_count = count_events(&events, |e| matches!(e, Event::ThoughtDone { .. }));
    assert_eq!(
        thought_done_count, 1,
        "ThoughtDone must be emitted even on stream error"
    );
}
