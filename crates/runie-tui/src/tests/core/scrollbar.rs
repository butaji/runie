use super::*;
use runie_core::model::AppState;
use runie_core::Event;
use runie_core::Part;
use runie_testing::fresh_state;

#[test]
fn scrollbar_no_scrollbar_when_content_fits() {
    let mut state = fresh_state();
    for i in 0..2 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: 0.0,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    let (thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(thumb, 0, "No scrollbar when content fits");
    assert_eq!(offset, 0);
}

#[test]
fn scrollbar_shows_when_content_overflows() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    let (thumb, _offset) = state.snapshot().scrollbar_metrics(10);
    assert!(
        thumb > 0,
        "Scrollbar thumb should be visible when content overflows, count={}",
        state.snapshot().element_count()
    );
}

#[test]
fn scrollbar_thumb_at_bottom_when_not_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 0; // at bottom
    let (thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(offset, 10 - thumb, "Thumb at bottom when scroll=0");
}

#[test]
fn scrollbar_thumb_at_top_when_fully_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 200; // way up, clamped to max_scroll
    let (_thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(offset, 0, "Thumb at top when fully scrolled");
}

#[test]
fn scrollbar_thumb_in_middle_when_half_scrolled() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    // 30 messages = 120 lines (30*3 messages + 30 spacers)
    // max_scroll = 110, thumb = max(1, 10*10/120) = 1
    state.view.scroll = 25; // halfway
    let (thumb, offset) = state.snapshot().scrollbar_metrics(10);
    // position = 110 - 25 = 85
    // thumb_start = round(85 * 10 / 120) = 7, thumb_end = round(95 * 10 / 120) = 8
    assert_eq!(thumb, 1, "Thumb size");
    assert_eq!(offset, 7, "Thumb should be in middle");
}

#[test]
fn scroll_clamped_to_max() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 500;
    let (_thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(offset, 0, "Scroll should be clamped to max");
}

#[test]
fn visible_uses_scroll_offset() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    // 10 messages = 40 lines (10*3 messages + 10 spacers), max_scroll = 35

    // At scroll=0 (bottom), we see newest 5 lines worth of elements
    let visible_bottom = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    assert!(visible_bottom.elements.iter().any(|e| matches!(e, runie_core::view::elements::Element::UserMessage { content, .. } if content == "msg9")), "Bottom should show latest");

    // At scroll=35 (top), we see oldest: first is msg0
    state.view.scroll = 35;
    let visible_top = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    assert!(visible_top.elements.iter().any(|e| matches!(e, runie_core::view::elements::Element::UserMessage { content, .. } if content == "msg0")), "Top should show oldest");
}

#[test]
fn scrollbar_thumb_never_exceeds_track() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    for scroll in [0, 5, 10, 20, 50, 100] {
        state.view.scroll = scroll;
        let (thumb, offset) = state.snapshot().scrollbar_metrics(10);
        assert!(
            thumb <= 10,
            "thumb={} must not exceed track=10 at scroll={}",
            thumb,
            scroll
        );
        assert!(
            offset + thumb <= 10,
            "thumb+offset={}+{}={} must not exceed track=10 at scroll={}",
            thumb,
            offset,
            thumb + offset,
            scroll
        );
    }
}

#[test]
fn scrollbar_consistent_between_offset_and_metrics() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    for scroll in [0, 5, 10, 25, 50, 100] {
        state.view.scroll = scroll;
        let offset = state.snapshot().scroll_offset(10) as usize;
        // View cache is now built into Snapshot
        let max_scroll = state.snapshot().total_lines.saturating_sub(10);
        let clamped_scroll = scroll.min(max_scroll);
        let expected_offset = max_scroll.saturating_sub(clamped_scroll);
        assert_eq!(
            offset, expected_offset,
            "scroll_offset mismatch at scroll={}",
            scroll
        );
    }
}

#[test]
fn compute_viewport_handles_partial_element_at_top() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    // View cache is now built into Snapshot
    state.view.scroll = state.snapshot().total_lines.saturating_sub(3);
    let visible = crate::tests::core::visible_helper::compute_viewport(&mut state, 3);
    assert!(!visible.elements.is_empty(), "Should have visible elements");
}

#[test]
fn pageup_scrolls_by_five_lines() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

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
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 20;

    state.update(Event::PageDown);
    assert_eq!(
        state.view.scroll, 15,
        "PageDown should scroll down by 5 lines"
    );

    state.update(Event::PageDown);
    assert_eq!(state.view.scroll, 10, "PageDown should accumulate");
}

#[test]
fn pagedown_stops_at_zero() {
    let mut state = fresh_state();
    for i in 0..30 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

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
    assert!(
        state.input.input_flash > 0,
        "PageUp on empty feed should flash"
    );
}

#[test]
fn pagedown_flashes_at_bottom() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 0;

    state.update(Event::PageDown);
    assert!(
        state.input.input_flash > 0,
        "PageDown at bottom should flash"
    );
}

#[test]
fn scrollbar_with_single_message() {
    let mut state = fresh_state();
    state.session.messages.push(runie_core::model::ChatMessage {
        role: runie_core::model::Role::User,
        parts: vec![Part::Text {
            content: "only".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let (thumb, offset) = state.snapshot().scrollbar_metrics(1);
    // View cache is now built into Snapshot
    let total = state.snapshot().total_lines;
    if total > 1 {
        assert!(
            thumb > 0,
            "Should have thumb when total={} > height=1",
            total
        );
    } else {
        assert_eq!(thumb, 0, "No thumb when content fits");
    }
    assert!(offset + thumb <= 1, "thumb+offset must fit in track");
}

#[test]
fn page_down_scrolls_by_rendered_lines() {
    let mut state = fresh_state();
    // Fill with enough content that wrapping occurs at the default width.
    for i in 0..10 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{} {}", i, "x".repeat(100)),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    // Start at the top of the feed so PageDown has room to move.
    // View cache is now built into Snapshot
    let max_scroll = state.snapshot().total_lines.saturating_sub(10);
    state.view.scroll = max_scroll;

    let before = state.view.scroll;
    state.update(Event::PageDown);
    let after = state.view.scroll;
    // PageDown moves toward the bottom by PAGE_SIZE rendered lines.
    assert_eq!(
        after,
        before.saturating_sub(5),
        "PageDown should scroll by 5 rendered lines"
    );
}

#[test]
fn scrollbar_thumb_position_matches_line_count() {
    let mut state = fresh_state();
    for i in 0..20 {
        state.session.messages.push(runie_core::model::ChatMessage {
            role: runie_core::model::Role::User,
            parts: vec![Part::Text {
                content: format!("msg{} {}", i, "x".repeat(100)),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    let snap = state.snapshot();
    let width = state.view.last_content_width;
    let expected_total: usize = snap
        .elements
        .iter()
        .map(|e| runie_core::layout::element_line_count(e, width))
        .sum();
    assert_eq!(
        snap.total_lines, expected_total,
        "Snapshot total_lines must match sum of rendered element line counts"
    );

    let visible_height = 10;
    let (thumb, offset) = state.snapshot().scrollbar_metrics(visible_height);
    let max_scroll = state.view.total_lines.saturating_sub(visible_height);
    let position = max_scroll.saturating_sub(state.view.scroll.min(max_scroll));
    let track = visible_height as f64;
    let expected_start = (position as f64 * track / snap.total_lines as f64)
        .round()
        .clamp(0.0, track - 1.0) as usize;
    assert_eq!(offset, expected_start, "Scrollbar thumb offset mismatch");
    assert!(thumb >= 1, "Thumb must be at least 1 row");
}
