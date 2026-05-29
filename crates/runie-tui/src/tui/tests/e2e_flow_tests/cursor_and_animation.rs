use super::*;

#[test]
fn test_e2e_cursor_blink_toggles() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert!(state.animation.streaming_cursor_visible);

    update(&mut state, &mut palette, Msg::CursorBlink);
    assert!(!state.animation.streaming_cursor_visible);

    update(&mut state, &mut palette, Msg::CursorBlink);
    assert!(state.animation.streaming_cursor_visible);
}

#[test]
fn test_e2e_animation_tick_advances() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    let initial_frame = state.animation.braille_frame;

    update(&mut state, &mut palette, Msg::Tick);

    // Frame should advance (modulo 10)
    assert_eq!(state.animation.braille_frame, (initial_frame + 1) % 10);
}
