use crate::model::AppState;

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
fn snapshot_turn_elapsed_increases_over_time() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let e1 = snap1.turn_elapsed_secs.unwrap();

    // Small delay to let time pass
    std::thread::sleep(std::time::Duration::from_millis(50));

    let snap2 = state.snapshot();
    let e2 = snap2.turn_elapsed_secs.unwrap();

    assert!(e2 > e1, "Elapsed time must increase: e1={} e2={}", e1, e2);
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
