use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, Event, Snapshot};
use runie_tui::ui::draw_snapshot;

fn render_snapshot(snap: &Snapshot) -> String {
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
    let out = render_snapshot(&snap);
    // In dev (RUNIE_MOCK) the input panel shows "mock/echo". In production
    // the app starts with no provider and the model area is empty.
    if runie_core::provider_registry::is_mock_enabled() {
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
    let out = render_snapshot(&snap);
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
    let out = render_snapshot(&snap);
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
        snap.spinner_frame, '⠴',
        "Spinner frame should be captured in snapshot"
    );
}

#[test]
fn snapshot_scrollbar_metrics_match_state() {
    let mut state = AppState::default();
    for i in 0..50 {
        state.update(Event::AgentResponse {
            id: format!("m{}", i),
            content: format!("line {}", i),
        });
    }
    state.ensure_fresh();
    let snap = state.snapshot();

    let (t1, o1) = state.scrollbar_metrics(10);
    let (t2, o2) = snap.scrollbar_metrics(10);
    assert_eq!(t1, t2, "Thumb size should match");
    assert_eq!(o1, o2, "Thumb offset should match");
}

#[test]
fn render_actor_does_not_need_mutable_state() {
    let mut state = AppState::default();
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    state.ensure_fresh();
    let snap = state.snapshot();

    // draw_snapshot takes &Snapshot, not &mut
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_snapshot(f, &snap)).unwrap();

    let buf = terminal.backend().buffer();
    let out: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        out.contains("→ Hello"),
        "Render actor should draw from immutable snapshot"
    );
}
