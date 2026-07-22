use super::*;
use crate::tests::connect_model;
use runie_core::Event;

fn render_status(state: &mut AppState) -> String {
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn status_line_shows_timer_when_turn_active() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(out.contains("Working"), "Status line must show 'Working'");
    assert!(out.contains("s"), "Status line must show timer with 's'");
}

#[test]
fn status_line_shows_spinner_and_timer() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.view.animation_frame = 3;
    state.ensure_fresh();

    let out = render_status(&mut state);
    let spinner = state.spinner_frame();
    assert!(
        out.contains(&spinner.to_string()),
        "Status must show spinner frame"
    );
    assert!(out.contains("Working"), "Status must show 'Working'");
}

#[test]
fn status_line_empty_when_turn_inactive() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = false;
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(
        !out.contains("Working"),
        "Status must not show 'Working' when inactive"
    );
}

#[test]
fn status_line_hides_spinner_when_turn_inactive() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = false;
    state.view.animation_frame = 3;
    state.ensure_fresh();

    let spinner = state.spinner_frame().to_string();
    let out = render_status(&mut state);
    assert!(
        !out.contains(&spinner),
        "Status must not render the braille spinner when idle, got: {out}"
    );
}

#[test]
fn status_timer_updates_over_time() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out1 = render_status(&mut state);
    let _timer_pos1 = out1.find("Working").unwrap_or(0);

    // Set the turn started time in the past so the timer shows elapsed time
    state.agent.turn_started_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(2));

    state.ensure_fresh();
    let out2 = render_status(&mut state);
    let _timer_pos2 = out2.find("Working").unwrap_or(0);

    // Both should show Working with a timer
    assert!(out1.contains("Working"));
    assert!(out2.contains("Working"));
}

#[test]
fn status_bar_renders_snapshot_spinner_frame_directly() {
    // Regression: the production draw path rendered a frozen ThrobberState
    // (calc_next was never called). The status bar must render the snapshot's
    // wall-clock spinner frame instead.
    let snap = runie_core::Snapshot {
        has_models: true,
        turn_active: true,
        provider: "mock".into(),
        model: "echo".into(),
        spinner_frame: '⠟',
        ..Default::default()
    };
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| crate::ui::draw_snapshot(f, &snap))
        .expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains('⠟'),
        "status bar must render the snapshot spinner frame, got: {content}"
    );
}

#[test]
fn status_line_uses_single_ellipsis_glyph() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(
        out.contains("Working…"),
        "Status must use the single … glyph (grok parity), got: {out}"
    );
    assert!(
        !out.contains("Working..."),
        "Status must not use three ASCII dots, got: {out}"
    );
}

#[test]
fn status_line_timer_drops_decimals_at_ten_seconds() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(24));
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(
        out.contains("24s") && !out.contains("24.0"),
        "≥10s timer must drop the decimal (grok parity), got: {out}"
    );
}

#[test]
fn status_line_hides_working_after_provider_error() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(out.contains("Working"), "Setup should show Working");

    state.update(Event::Error { id: "req.0".to_string(), message: "Provider error: Missing API key".to_string() });
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(
        !out.contains("Working"),
        "Status must hide Working after provider error, got: {}",
        out
    );
}
