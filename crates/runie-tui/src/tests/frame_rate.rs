//! Tests for animation frame rate (60fps).

use crate::ui_actor::ANIM_MS;

#[test]
fn animation_frame_rate_is_60fps() {
    // 60fps = 1000ms / 60 = ~16.67ms per frame
    // We use 16ms which gives 62.5fps, close enough for smooth rendering
    assert_eq!(
        ANIM_MS, 16,
        "Animation interval should be 16ms for ~60fps rendering"
    );
}

#[test]
fn animation_interval_allows_smooth_typing() {
    // Verify the interval is fast enough for smooth typing animation
    // At 16ms per frame, we can render 62.5 frames per second
    let fps = 1000.0 / ANIM_MS as f64;
    assert!(
        fps >= 60.0,
        "Frame rate should be at least 60fps, got {:.1}",
        fps
    );
}
