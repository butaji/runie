use crate::model::AppState;

#[test]
fn spinner_frame_cycles_twelve_times() {
    let mut state = AppState::default();
    let first = state.spinner_frame();
    for i in 1..=12 {
        state.view.animation_frame = i;
        if i == 12 {
            assert_eq!(
                state.spinner_frame(),
                first,
                "spinner should cycle after 12 frames"
            );
        } else {
            assert_ne!(
                state.spinner_frame(),
                first,
                "frame {} should differ from frame 0",
                i
            );
        }
    }
}

#[test]
fn snapshot_has_turn_elapsed_when_active() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let snap = state.snapshot();
    assert!(
        snap.turn_elapsed_secs.is_some(),
        "Snapshot must include turn elapsed time when active"
    );
    let elapsed = snap.turn_elapsed_secs.unwrap();
    assert!(elapsed >= 0.0, "Elapsed must be non-negative");
}

#[test]
fn snapshot_turn_elapsed_none_when_inactive() {
    let mut state = AppState::default();
    state.agent.turn_active = false;
    state.agent.turn_started_at = None;
    state.ensure_fresh();

    let snap = state.snapshot();
    assert!(
        snap.turn_elapsed_secs.is_none(),
        "Snapshot must not include turn elapsed time when inactive"
    );
}

#[test]
fn snapshot_turn_elapsed_calculates_correctly() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    // Set started_at to 500ms in the past
    let started = std::time::Instant::now() - std::time::Duration::from_millis(500);
    state.agent.turn_started_at = Some(started);
    state.ensure_fresh();

    let snap = state.snapshot();
    let elapsed = snap.turn_elapsed_secs.unwrap();
    // Elapsed should be approximately 0.5s (within 50ms tolerance)
    assert!(
        (elapsed - 0.5).abs() < 0.05,
        "Elapsed should be ~0.5s, got {}",
        elapsed
    );
}

#[test]
fn status_text_contains_timer_when_turn_active() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let snap = state.snapshot();
    // Build a renderable string from the snapshot status
    let mut parts = Vec::new();
    if snap.turn_active {
        if let Some(elapsed) = snap.turn_elapsed_secs {
            parts.push(format!("{} Working {:.1}s", snap.spinner_frame, elapsed));
        } else {
            parts.push(format!("{} Working", snap.spinner_frame));
        }
    }
    let status_text = parts.join(" | ");

    assert!(
        status_text.contains("Working"),
        "Status must contain 'Working'"
    );
    assert!(
        status_text.contains("s"),
        "Status must contain timer with 's' suffix"
    );
}

#[test]
fn status_text_no_timer_when_turn_inactive() {
    let mut state = AppState::default();
    state.agent.turn_active = false;
    state.ensure_fresh();

    let snap = state.snapshot();
    let mut parts = Vec::new();
    if snap.turn_active {
        if let Some(elapsed) = snap.turn_elapsed_secs {
            parts.push(format!("{} Working {:.1}s", snap.spinner_frame, elapsed));
        } else {
            parts.push(format!("{} Working", snap.spinner_frame));
        }
    }
    let status_text = parts.join(" | ");

    assert!(
        status_text.is_empty(),
        "Status must be empty when turn inactive"
    );
}

#[test]
fn turn_elapsed_survives_snapshot_cloning() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let snap = state.snapshot();
    let elapsed = snap.turn_elapsed_secs;
    assert!(elapsed.is_some());

    // Clone the snapshot (simulating render actor receiving it)
    let snap2 = snap.clone();
    assert_eq!(
        snap2.turn_elapsed_secs, elapsed,
        "Elapsed must survive snapshot clone"
    );
}
