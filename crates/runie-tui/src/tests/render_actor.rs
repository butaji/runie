#![allow(clippy::too_many_lines)]
use super::*;
use crate::ui::draw_snapshot;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

fn has_content(elem: &Element, text: &str) -> bool {
    match elem {
        Element::AgentMessage { content, .. } => content.contains(text),
        Element::UserMessage { content, .. } => content.contains(text),
        Element::ToolDone { output, .. } => output.contains(text),
        Element::ToolRunning { name, args, .. } => name.contains(text) || args.contains(text),
        Element::ThoughtMarker { content, .. } => content.contains(text),
        Element::ThoughtSummary { content, .. } => content.contains(text),
        Element::ToolSummary { name, .. } => name.contains(text),
        _ => false,
    }
}

fn render_snapshot(snap: &Snapshot, _animation_frame: u32) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, snap)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn snapshot_renders_empty_state() {
    let mut state = AppState::default();
    state.ensure_fresh();
    let snap = state.snapshot();
    let out = render_snapshot(&snap, state.view().animation_frame);
    // In dev (RUNIE_MOCK) the input panel shows "mock/echo". In production
    // the app starts with no provider and the model area is empty.
    if runie_core::provider::is_mock_enabled() {
        assert!(
            out.contains("mock/echo"),
            "input panel should show mock/echo in dev"
        );
    }
}

#[test]
fn snapshot_renders_user_message() {
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    state.ensure_fresh();
    let snap = state.snapshot();
    let out = render_snapshot(&snap, state.view().animation_frame);
    assert!(
        out.contains("❯ Hi"),
        "Should render user message in snapshot"
    );
}

#[test]
fn snapshot_is_immutable_after_creation() {
    let mut state = AppState::default();
    state.input.input = "A".to_string();
    state.update(Event::Submit);
    state.ensure_fresh();
    let snap = state.snapshot();

    // Mutate state AFTER snapshot
    state.input.input = "B".to_string();
    state.update(Event::Submit);
    state.ensure_fresh();

    // Snapshot should still show old state
    let out = render_snapshot(&snap, state.view().animation_frame);
    assert!(out.contains("❯ A"), "Snapshot should be immutable");
    assert!(
        !out.contains("❯ B"),
        "Snapshot should not reflect later changes"
    );
}

#[test]
fn snapshot_spinner_frame_captured() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.view.animation_frame = 5;
    state.ensure_fresh();
    let snap = state.snapshot();
    assert_eq!(
        snap.spinner_frame, '⠾',
        "Spinner frame should be captured in snapshot"
    );
}

#[test]
fn snapshot_scrollbar_metrics_match_state() {
    let mut state = AppState::default();
    for i in 0..50 {
        state.update(Event::Response {
            id: format!("m{}", i),
            content: format!("line {}", i),
            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        });
    }
    state.ensure_fresh();
    let snap = state.snapshot();

    let (t1, o1) = snap.scrollbar_metrics(10);
    let (t2, o2) = snap.scrollbar_metrics(10);
    assert_eq!(t1, t2, "Thumb size should match");
    assert_eq!(o1, o2, "Thumb offset should match");
}

#[test]
fn render_actor_does_not_need_mutable_state() {
    let mut state = AppState::default();
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();
    let snap = state.snapshot();

    // draw_snapshot takes &Snapshot + &mut ThrobberState
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("Hello"),
        "Render actor should draw from immutable snapshot"
    );
}

/// Layer 3: Feed a sequence of events into AppState and verify the produced
/// Snapshot contains the expected messages.
#[test]
fn ui_actor_snapshot_after_events() {
    let mut state = AppState::default();

    // Feed user message event
    state.update(Event::Submit);

    // Feed agent response events
    state.update(Event::Response {
        id: "msg.1".to_string(),
        content: "Hello!".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::Response {
        id: "msg.2".to_string(),
        content: "How can I help?".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });

    // Feed tool call events
    state.update(Event::ToolStart {
        id: "tool.1".to_string(),
        name: "bash".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 1.5, output: "done".to_string() });

    state.ensure_fresh();
    let snap = state.snapshot();

    // Verify snapshot contains the expected messages
    assert!(
        snap.elements.iter().any(|e| has_content(e, "Hello!")),
        "Should contain msg1"
    );
    assert!(
        snap.elements
            .iter()
            .any(|e| has_content(e, "How can I help?")),
        "Should contain msg2"
    );
    assert!(
        snap.elements.iter().any(|e| has_content(e, "done")),
        "Should contain tool output"
    );
    assert!(
        !snap.turn_active || snap.turn_elapsed_secs.is_some(),
        "Should have turn state"
    );
}

/// Integration test: ResponseDelta events (streaming) must produce visible text in the rendered feed.
/// This is the core regression test for the "agent message not showing" bug.
#[test]
fn response_delta_renders_in_feed() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Simulate a streaming response: "hello\n"
    // newline makes the line "stable" in the streaming buffer
    state.update(Event::ResponseDelta { id: "req.0".to_string(), content: "hello\n".to_string() });

    // Done flushes any remaining tail
    state.update(Event::Done { id: "req.0".to_string() });

    state.ensure_fresh();
    let snap = state.snapshot();

    // Verify snapshot has an agent message with "hello"
    let has_hello = snap.elements.iter().any(|e| has_content(e, "hello"));
    assert!(
        has_hello,
        "Snapshot should contain 'hello' in AgentMessage. Elements: {:?}",
        snap.elements
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );

    // Render and verify "hello" appears in the terminal output
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("hello"),
        "Rendered output should contain 'hello'. Got: {}",
        out
    );
}

/// Integration test: ResponseDelta WITHOUT trailing newline (buffered until Done) must render.
#[test]
fn response_delta_without_trailing_newline_renders_after_done() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Send content WITHOUT trailing newline - debounce would prevent flush
    state.update(Event::ResponseDelta { id: "req.0".to_string(), content: "hello".to_string() });

    // Done calls force_flush which should push remaining tail
    state.update(Event::Done { id: "req.0".to_string() });

    state.ensure_fresh();
    let snap = state.snapshot();

    let has_hello = snap.elements.iter().any(|e| has_content(e, "hello"));
    assert!(
        has_hello,
        "Snapshot should contain 'hello' after Done. Elements: {:?}",
        snap.elements
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("hello"),
        "Rendered output should contain 'hello'. Got: {}",
        out
    );
}

/// Integration test: production-style event flow with TextStart + ResponseDelta + Done.
/// Uses empty id (matching production where TurnActor emits id:"").
#[test]
fn text_start_response_delta_done_renders_agent_text() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Simulate the exact production event sequence:
    // 1. TextStart begins a new Part::Text (no id in production)
    state.update(Event::TextStart { id: String::new() });
    // 2. ResponseDelta streams content (empty id from TurnActor in production)
    state.update(Event::ResponseDelta { id: String::new(), content: "hello world\n".to_string() });
    // 3. Done finalizes (empty id from TurnActor in production)
    state.update(Event::Done { id: String::new() });

    state.ensure_fresh();
    let snap = state.snapshot();

    // Verify assistant message exists
    let has_hello = snap.elements.iter().any(|e| has_content(e, "hello"));
    let agent_elems: Vec<_> = snap
        .elements
        .iter()
        .filter(|e| matches!(e, Element::AgentMessage { .. }))
        .collect();
    assert!(
        has_hello,
        "Snapshot should contain 'hello' after TextStart→ResponseDelta→Done. \
         Agent elements: {:?}",
        agent_elems
    );

    // Render and verify
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("hello"),
        "Rendered output should contain 'hello'. Got: {}",
        out
    );
}

/// Diagnostic test: check exactly what elements exist in state after event sequence.
/// This reproduces the production flow step by step with detailed inspection.
#[test]
fn diagnostic_production_flow_elements() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Step 1: TextStart
    state.update(Event::TextStart { id: String::new() });
    state.ensure_fresh();
    let snap1 = state.snapshot();
    eprintln!("After TextStart: elements={:?}", snap1.elements);
    eprintln!("  messages count: {}", state.session().messages.len());
    for (i, m) in state.session().messages.iter().enumerate() {
        eprintln!(
            "  msg[{}]: role={:?} id={:?} content={:?} parts={:?}",
            i,
            m.role,
            m.id,
            m.content(),
            m.parts
        );
    }

    // Step 2: ResponseDelta
    state.update(Event::ResponseDelta { id: String::new(), content: "hello world\n".to_string() });
    state.ensure_fresh();
    let snap2 = state.snapshot();
    eprintln!("After ResponseDelta: elements={:?}", snap2.elements);
    for (i, m) in state.session().messages.iter().enumerate() {
        eprintln!(
            "  msg[{}]: role={:?} id={:?} content={:?} parts={:?}",
            i,
            m.role,
            m.id,
            m.content(),
            m.parts
        );
    }

    // Step 3: Done
    state.update(Event::Done { id: String::new() });
    state.ensure_fresh();
    let snap3 = state.snapshot();
    eprintln!("After Done: elements={:?}", snap3.elements);
    for (i, m) in state.session().messages.iter().enumerate() {
        eprintln!(
            "  msg[{}]: role={:?} id={:?} content={:?} parts={:?}",
            i,
            m.role,
            m.id,
            m.content(),
            m.parts
        );
    }

    // Check total_lines and content_width
    eprintln!(
        "total_lines={} content_width={}",
        snap3.total_lines, snap3.content_width
    );

    // Render
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap3)).unwrap();
    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    eprintln!("RENDERED OUTPUT:\n{}", out);

    assert!(
        snap3
            .elements
            .iter()
            .any(|e| matches!(e, Element::AgentMessage { .. })),
        "Must have AgentMessage element"
    );
}
