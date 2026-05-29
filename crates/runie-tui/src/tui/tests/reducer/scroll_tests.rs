use super::*;

#[test]
fn test_scroll_up_at_boundary() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.scroll.feed_offset = 0;
    update(&mut state, &mut palette, Msg::ScrollUp);
    assert_eq!(state.scroll.feed_offset, 0, "ScrollUp at 0 stays at 0");
}

#[test]
fn test_scroll_down_at_boundary() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.messages.push(MessageItem::User { text: "a".to_string(), model: Some("You".to_string()), timestamp: None });
    state.messages.push(MessageItem::User { text: "b".to_string(), model: Some("You".to_string()), timestamp: None });
    state.scroll.feed_offset = 1;

    update(&mut state, &mut palette, Msg::ScrollDown);

    assert_eq!(state.scroll.feed_offset, 1, "ScrollDown at max stays at max");
}

// BG-8: State preserved when switching modes
#[test]
fn test_scroll_preserved_on_mode_switch() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Chat;
    state.scroll.feed_offset = 100;

    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
    assert_eq!(state.scroll.feed_offset, 100, "Scroll should be preserved when returning to Chat");
}
