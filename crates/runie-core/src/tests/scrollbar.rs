use crate::model::AppState;
use crate::Event;

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

#[test]
fn scrollbar_thumb_never_exceeds_track() {
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
    for scroll in [0, 5, 10, 20, 50, 100] {
        state.view.scroll = scroll;
        let (thumb, offset) = state.scrollbar_metrics(10);
        assert!(thumb <= 10, "thumb={} must not exceed track=10 at scroll={}", thumb, scroll);
        assert!(offset + thumb <= 10, "thumb+offset={}+{}={} must not exceed track=10 at scroll={}", thumb, offset, thumb + offset, scroll);
    }
}

#[test]
fn scrollbar_consistent_between_offset_and_metrics() {
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
    for scroll in [0, 5, 10, 25, 50, 100] {
        state.view.scroll = scroll;
        let offset = state.scroll_offset(10) as usize;
        let max_scroll = state.view.total_lines.saturating_sub(10);
        let clamped_scroll = scroll.min(max_scroll);
        let expected_offset = max_scroll.saturating_sub(clamped_scroll);
        assert_eq!(offset, expected_offset, "scroll_offset mismatch at scroll={}", scroll);
    }
}

#[test]
fn visible_scroll_handles_partial_element_at_top() {
    let mut state = fresh_state();
    for i in 0..5 {
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
    state.view.scroll = state.view.total_lines.saturating_sub(3);
    let visible = state.visible_scroll(3);
    assert!(!visible.elements.is_empty(), "Should have visible elements");
}

#[test]
fn pageup_scrolls_by_five_lines() {
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

    state.update(Event::PageUp);
    assert_eq!(state.view.scroll, 5, "PageUp should scroll by 5 lines");

    state.update(Event::PageUp);
    assert_eq!(state.view.scroll, 10, "PageUp should accumulate");
}

#[test]
fn pagedown_scrolls_down_by_five_lines() {
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
    state.view.scroll = 20;

    state.update(Event::PageDown);
    assert_eq!(state.view.scroll, 15, "PageDown should scroll down by 5 lines");

    state.update(Event::PageDown);
    assert_eq!(state.view.scroll, 10, "PageDown should accumulate");
}

#[test]
fn pagedown_stops_at_zero() {
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
    state.view.scroll = 3;

    state.update(Event::PageDown);
    assert_eq!(state.view.scroll, 0, "PageDown should clamp at 0");

    state.update(Event::PageDown);
    assert_eq!(state.view.scroll, 0, "PageDown at 0 should stay 0");
}

#[test]
fn pageup_flashes_when_empty() {
    let mut state = fresh_state();
    state.update(Event::PageUp);
    assert!(state.input.input_flash > 0, "PageUp on empty feed should flash");
}

#[test]
fn pagedown_flashes_at_bottom() {
    let mut state = fresh_state();
    for i in 0..5 {
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
    state.view.scroll = 0;

    state.update(Event::PageDown);
    assert!(state.input.input_flash > 0, "PageDown at bottom should flash");
}

#[test]
fn scrollbar_with_single_message() {
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "only".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    let (thumb, offset) = state.scrollbar_metrics(1);
    let total = state.view.total_lines;
    if total > 1 {
        assert!(thumb > 0, "Should have thumb when total={} > height=1", total);
    } else {
        assert_eq!(thumb, 0, "No thumb when content fits");
    }
    assert_eq!(offset + thumb <= 1, true, "thumb+offset must fit in track");
}
