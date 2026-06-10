use crate::model::AppState;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn scrollbar_no_scrollbar_when_content_fits() {
    let mut state = fresh_state();
    for i in 0..2 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: 0.0,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    let (thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(thumb, 0, "No scrollbar when content fits");
    assert_eq!(offset, 0);
}

#[test]
fn scrollbar_shows_when_content_overflows() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    let (thumb, _offset) = state.scrollbar_metrics(10);
    assert!(thumb > 0, "Scrollbar thumb should be visible when content overflows, count={}", state.count());
}

#[test]
fn scrollbar_thumb_at_bottom_when_not_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 0; // at bottom
    let (thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(offset, 10 - thumb, "Thumb at bottom when scroll=0");
}

#[test]
fn scrollbar_thumb_at_top_when_fully_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 100; // way up
    let (_thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(offset, 0, "Thumb at top when fully scrolled");
}

#[test]
fn scrollbar_thumb_in_middle_when_half_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    // 30 messages = 120 lines (30*3 messages + 30 spacers)
    // max_scroll = 110, thumb = max(1, 10*10/120) = 1
    state.view.scroll = 25; // halfway
    let (thumb, offset) = state.scrollbar_metrics(10);
    let expected_offset = (110 - 25) * (10 - thumb) / 110;
    assert_eq!(offset, expected_offset, "Thumb should be in middle");
}

#[test]
fn scroll_clamped_to_max() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.scroll = 500;
    let (_thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(offset, 0, "Scroll should be clamped to max");
}

#[test]
fn visible_uses_scroll_offset() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.session.messages.push(crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    // 10 messages = 40 lines (10*3 messages + 10 spacers), max_scroll = 35

    // At scroll=0 (bottom), we see newest 5 lines worth of elements
    let visible_bottom = state.visible_scroll(5);
    assert!(visible_bottom.elements.iter().any(|e| matches!(e, crate::ui::elements::Element::UserMessage { content, .. } if content == "msg9")), "Bottom should show latest");

    // At scroll=35 (top), we see oldest: first is msg0
    state.view.scroll = 35;
    let visible_top = state.visible_scroll(5);
    assert!(visible_top.elements.iter().any(|e| matches!(e, crate::ui::elements::Element::UserMessage { content, .. } if content == "msg0")), "Top should show oldest");
}
