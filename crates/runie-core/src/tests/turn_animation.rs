//! Turn animation: wall-clock spinner cadence + grok elapsed timer format.
//! See GROK.md §24 (runie-tests) for the verified grok behavior.

use crate::labels::{format_elapsed_secs, BRAILLE_SIX};
use crate::model::AppState;
use std::time::{Duration, Instant};

#[test]
fn elapsed_format_one_decimal_below_ten_seconds() {
    assert_eq!(format_elapsed_secs(0.0), "0.0s");
    assert_eq!(format_elapsed_secs(0.4), "0.4s");
    assert_eq!(format_elapsed_secs(1.5), "1.5s");
    assert_eq!(format_elapsed_secs(9.9), "9.9s");
}

#[test]
fn elapsed_format_integer_at_ten_seconds_and_above() {
    assert_eq!(format_elapsed_secs(10.0), "10s");
    assert_eq!(format_elapsed_secs(24.0), "24s");
    assert_eq!(format_elapsed_secs(61.4), "61s");
}

#[test]
fn spinner_frame_is_wall_clock_driven_when_turn_started() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    // Bucket mid-points keep the test jitter-safe (~120ms per frame).
    // 300ms → bucket 2.
    state.agent.turn_started_at = Some(Instant::now() - Duration::from_millis(300));
    assert_eq!(
        state.spinner_frame(),
        BRAILLE_SIX[2],
        "spinner must derive from elapsed wall time, not render ticks"
    );
    // 1260ms → bucket 10 → 10 % 6 = 4.
    state.agent.turn_started_at = Some(Instant::now() - Duration::from_millis(1260));
    assert_eq!(state.spinner_frame(), BRAILLE_SIX[4]);
}

#[test]
fn spinner_frame_falls_back_to_animation_frame_without_turn_start() {
    let mut state = AppState::default();
    state.view.animation_frame = 5;
    assert_eq!(state.spinner_frame(), BRAILLE_SIX[5]);
}
