//! Tests for animation frame rate (60fps).
//!
//! Task 25 (idle redraw): the render loop is not flooded at 60fps when idle —
//! the UiActor's animation tick only publishes a snapshot when the state is
//! dirty, and `tick_animation` does not mark an idle state dirty (covered by
//! `crate::tests::token_counters::animation` in runie-core).

use crate::ui_actor::animation_interval_ms;
use runie_core::AppState;

#[test]
fn animation_frame_rate_is_60fps() {
    // 60fps = 1000ms / 60 = ~16.67ms per frame
    // We use 16ms which gives 62.5fps, close enough for smooth rendering
    assert_eq!(
        animation_interval_ms(&AppState::default()),
        16,
        "Animation interval should be 16ms for ~60fps rendering"
    );
}

#[test]
fn animation_interval_allows_smooth_typing() {
    // Verify the interval is fast enough for smooth typing animation
    // At 16ms per frame, we can render 62.5 frames per second
    let fps = 1000.0 / animation_interval_ms(&AppState::default()) as f64;
    assert!(
        fps >= 60.0,
        "Frame rate should be at least 60fps, got {:.1}",
        fps
    );
}

#[test]
fn configured_animation_fps_controls_the_shared_interval() {
    let mut state = AppState::default();
    state.config_mut().animation_fps = 20;
    assert_eq!(animation_interval_ms(&state), 50);

    state.config_mut().animation_fps = 0;
    assert_eq!(animation_interval_ms(&state), 1000);
    state.config_mut().animation_fps = 999;
    assert_eq!(animation_interval_ms(&state), 16);
}
