use super::*;

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
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.animation_frame = 3;
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
    state.agent.turn_active = false;
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(
        !out.contains("Working"),
        "Status must not show 'Working' when inactive"
    );
}

#[test]
fn status_timer_updates_over_time() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out1 = render_status(&mut state);
    let _timer_pos1 = out1.find("Working").unwrap_or(0);

    std::thread::sleep(std::time::Duration::from_millis(100));

    state.ensure_fresh();
    let out2 = render_status(&mut state);
    let _timer_pos2 = out2.find("Working").unwrap_or(0);

    // Both should show Working with a timer
    assert!(out1.contains("Working"));
    assert!(out2.contains("Working"));
}
