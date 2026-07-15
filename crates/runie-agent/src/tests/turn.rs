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

    // Mock provider echoes "hello" back as a single delta (with trailing newline).
    assert_eq!(
        deltas.len(),
        1,
        "single-word 'hello' should produce exactly 1 ResponseDelta, got {deltas:?}"
    );
    assert_eq!(deltas[0].as_str(), "hello\n");
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
    assert_eq!(deltas, 1); // mock echo returns the input as a single delta
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
    let bash_calls = count_events(
        &events,
        |e| matches!(e, Event::ToolStart { name, .. } if name == "bash"),
    );

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
fn skills_context_injected_into_system_prompt() {
    let cmd = agent_cmd("grill me on caching")
        .skills_context("## Context\n\nInterview me relentlessly.")
        .build();
    let msgs = build_initial_messages(&cmd, false);
    let system = match &msgs[0].role {
        runie_core::message::Role::System => msgs[0].content().clone(),
        _ => panic!("expected system message"),
    };
    assert!(
        system.contains("Interview me relentlessly"),
        "skills_context must be appended to the system prompt, got: {}",
        system
    );
}

#[test]
fn read_only_excludes_write_tools() {
    let cmd = agent_cmd("test").read_only(true).build();
    let msgs = build_initial_messages(&cmd, true);
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
    let msgs = build_initial_messages(&cmd, true);
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

#[test]
fn non_tool_model_omits_tool_instructions() {
    let cmd = agent_cmd("hello").build();
    let msgs = build_initial_messages(&cmd, false);
    let system = match &msgs[0].role {
        runie_core::message::Role::System => msgs[0].content().clone(),
        _ => panic!("expected system message"),
    };
    assert!(
        system.contains("You are a helpful assistant"),
        "base personality should be preserved"
    );
    assert!(
        !system.contains("Available tools"),
        "non-tool model must not include Available tools"
    );
    assert!(
        !system.contains("Use structured JSON format"),
        "non-tool model must not include tool JSON format"
    );
    assert!(
        !system.contains("bash"),
        "non-tool model must not list tools"
    );
}

#[test]
fn tool_model_includes_tool_instructions() {
    let cmd = agent_cmd("hello").build();
    let msgs = build_initial_messages(&cmd, true);
    let system = match &msgs[0].role {
        runie_core::message::Role::System => msgs[0].content().clone(),
        _ => panic!("expected system message"),
    };
    assert!(
        system.contains("Available tools"),
        "tool model must include Available tools"
    );
    assert!(
        system.contains("Use structured JSON format"),
        "tool model must include tool JSON format"
    );
    assert!(system.contains("bash"), "tool model must list tools");
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
    assert_eq!(
        turn_complete_count, 1,
        "text-only turn must emit TurnComplete"
    );
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
    let cmd = agent_cmd("error")
        .id("req.err")
        .provider("error")
        .model("error")
        .build();
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

// ========================================================================
// ISSUE F — denied / repeated tool calls must not loop in the TUI turn path
// ========================================================================
//
// MiniMax bash loop: the agent ran `ls -la`, was denied, then re-issued the
// same call until the iteration cap (and a queued follow-up restarted it).
// The headless runner already stops on denial (`any_blocked`); these tests
// pin the same guard plus a same-call repeat guard into the TUI turn loop.

/// Stop-on-deny: a denied bash tool must terminate the turn after the first
/// denial. The mock provider re-emits the same native bash call every round,
/// so without the guard this would spin to `max_iterations`.
#[tokio::test]
async fn denied_tool_does_not_loop_in_turn() {
    use runie_core::permissions::{DenyAllSink, PermissionManager};
    use std::sync::Arc;

    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
    let cmd = agent_cmd("native tool").build();
    let (events, emit) = capture_events();

    let gate = crate::PermissionGate::new(
        PermissionManager::default(),
        Arc::new(DenyAllSink) as Arc<dyn runie_core::permissions::ApprovalSink>,
    );

    // High cap: without the fix the loop would re-issue the bash call every
    // round up to the cap.
    run_agent_turn(&provider, &cmd, emit, 10, gate)
        .await
        .unwrap();

    let bash_starts = count_events(
        &events,
        |e| matches!(e, Event::ToolStart { name, .. } if name == "bash"),
    );
    assert_eq!(
        bash_starts, 1,
        "denied bash tool must execute exactly once (no re-issue), got {bash_starts}"
    );

    // Exactly one model request: the turn stopped after the denial and never
    // asked the model a second time.
    let thinking = count_events(&events, |e| matches!(e, Event::Thinking { .. }));
    assert_eq!(
        thinking, 1,
        "turn must not issue a second model request after denial, got {thinking}"
    );

    let tool_ends: Vec<String> = events
        .lock()
        .iter()
        .filter_map(|e| match e {
            Event::ToolEnd { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        tool_ends.len(),
        1,
        "expected exactly one ToolEnd, got {tool_ends:?}"
    );
    assert!(
        tool_ends[0].contains("Permission denied"),
        "expected denial content, got: {}",
        tool_ends[0]
    );

    assert_eq!(
        count_events(&events, |e| matches!(e, Event::Done { .. })),
        1,
        "turn must still complete cleanly"
    );
}

/// Repeat guard: a provider that always returns the identical (name, args)
/// bash call with an allow-all gate must execute once and then stop — well
/// before `max_iterations` — instead of spinning to the cap.
#[tokio::test]
async fn repeated_tool_call_does_not_loop_in_turn() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
    let cmd = agent_cmd("native tool").build();
    let (events, emit) = capture_events();

    // High cap on purpose: the repeat guard must stop the loop after the
    // first execution, long before the cap is reached.
    run_agent_turn(&provider, &cmd, emit, 10, allow_all_gate())
        .await
        .unwrap();

    let bash_starts = count_events(
        &events,
        |e| matches!(e, Event::ToolStart { name, .. } if name == "bash"),
    );
    assert_eq!(
        bash_starts, 1,
        "identical re-issued bash call must run once then be stopped by the repeat guard, got {bash_starts}"
    );

    assert_eq!(
        count_events(&events, |e| matches!(e, Event::Done { .. })),
        1,
        "turn must complete cleanly after the repeat guard stops it"
    );
}
