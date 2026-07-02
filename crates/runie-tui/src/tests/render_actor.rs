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

fn render_snapshot(snap: &Snapshot, animation_frame: u32) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut throbber = throbber_widgets_tui::ThrobberState::default();
    // Sync throbber index to the animation frame from AppState.
    let frame_idx = (animation_frame % 6) as i8;
    if frame_idx != 0 {
        throbber.calc_step(frame_idx);
    }
    terminal.draw(|f| draw_snapshot(f, snap, &mut throbber)).unwrap();
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
    let mut throbber = throbber_widgets_tui::ThrobberState::default();
    terminal.draw(|f| draw_snapshot(f, &snap, &mut throbber)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("→ Hello"),
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
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 1.5,
        output: "done".to_string(),
    });

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
