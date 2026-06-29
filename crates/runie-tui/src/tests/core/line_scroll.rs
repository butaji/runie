use runie_core::model::{ChatMessage, Role};
use runie_core::view::Element;
use runie_core::Part;
use runie_testing::fresh_state;

// Helper: build a thought with N lines of reasoning
fn thought_msg(id: &str, n_lines: usize) -> ChatMessage {
    let content = std::iter::once("◆ Thought 1.0s".to_string())
        .chain((1..=n_lines).map(|i| format!("line{}", i)))
        .collect::<Vec<_>>()
        .join("\n");
    ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text {
            content: content.into(),
        }],
        timestamp: 1.0,
        id: id.to_string(),
        ..Default::default()
    }
}

// Helper: build a tool with N lines of output
fn tool_msg(id: &str, n_output_lines: usize) -> ChatMessage {
    let output = (1..=n_output_lines)
        .map(|i| format!("out{}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("◆ Ran ls 0.5s\n{}", output);
    ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: content.into(),
        }],
        timestamp: 1.0,
        id: id.to_string(),
        ..Default::default()
    }
}

// ── Line count basics ─────────────────────────────────────────────────

#[test]
fn user_message_is_one_line() {
    let msg = ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        timestamp: 0.0,
        id: "u".into(),
        ..Default::default()
    };
    let mut state = fresh_state();
    state.session.messages.push(msg);
    state.refresh_after_message_change();

    assert_eq!(
        state.view.total_lines,
        4,
        "UserMessage (3: margins+content) + Spacer (1) = 4 lines"
    );
}

#[test]
fn thought_line_count_matches_content() {
    let mut state = fresh_state();
    // header + 5 lines = 6 lines of content, + 1 spacer = 7 total element lines
    state.session.messages.push(thought_msg("t1", 5));
    state.refresh_after_message_change();

    let total = state.view.total_lines;
    // ThoughtMarker has 6 lines (header + 5), + leading spacer + trailing spacer = 8
    assert_eq!(
        total, 8,
        "Thought with 5 content lines should be 6+2=8 lines total"
    );
}

#[test]
fn tool_line_count_matches_output() {
    let mut state = fresh_state();
    state.session.messages.push(tool_msg("x1", 3));
    state.refresh_after_message_change();

    let total = state.view.total_lines;
    // ToolDone: header (1) + output (3) = 4, + leading spacer + trailing spacer = 6
    assert_eq!(
        total, 6,
        "Tool with 3 output lines should be 4+2=6 lines total"
    );
}

// ── Visible region: latest at bottom ──────────────────────────────────

#[test]
fn visible_shows_latest_element_at_bottom() {
    let mut state = fresh_state();
    for i in 0..3 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
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

    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 3); // 3 lines viewport
                                                                                  // 3 messages = 3*3 UserMessage + 3 Spacer = 12 lines total
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg2")),
        "Latest message (msg2) must be in visible region"
    );
}

#[test]
fn visible_skips_lines_from_first_element_when_overflow() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "first".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.session.messages.push(thought_msg("t1", 30)); // 31 lines of thought
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "latest".into(),
        }],
        timestamp: 2.0,
        id: "u2".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    state.view.scroll = 0;
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 10);
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "latest")),
        "Latest message must be visible"
    );
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::ThoughtMarker { .. })),
        "Thought must be partially visible"
    );
    assert!(
        region.skip_lines > 0,
        "Should skip lines from top of first visible element"
    );
}

#[test]
fn scroll_up_shows_older_content() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    // 5 messages = 20 lines total (5*3 messages + 5 spacers). Viewport of 3 lines.
    // scroll=8: viewport [9, 12) — msg2 visible, msg4 hidden
    state.view.scroll = 8;
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 3);
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg2")),
        "Scroll up should show older message (msg2)"
    );
    assert!(
        !region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg4")),
        "Scroll up should hide msg4"
    );
}

// ── Scrollbar line-based ──────────────────────────────────────────────

#[test]
fn scrollbar_no_scrollbar_when_lines_fit() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "hi".into(),
        }],
        timestamp: 0.0,
        id: "u".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let (thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(thumb, 0, "No scrollbar when total lines fit in viewport");
    assert_eq!(offset, 0);
}

#[test]
fn scrollbar_shows_when_lines_overflow() {
    let mut state = fresh_state();
    // 20 messages = 40 lines, viewport = 10
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
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
        "Scrollbar thumb should show when line count exceeds viewport"
    );
}

#[test]
fn scrollbar_thumb_at_bottom_when_not_scrolled() {
    let mut state = fresh_state();
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
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

    let (_thumb, offset) = state.snapshot().scrollbar_metrics(10);
    // 80 lines total (20 msgs × 4), viewport 10, position = 70
    // thumb_start = round(70 * 10 / 80) = 9, thumb_end = round(80 * 10 / 80) = 10
    // thumb = 1, offset = 9 (bottom of 10-row track)
    assert_eq!(offset, 9, "Thumb at bottom track edge when scroll=0");
}

#[test]
fn scrollbar_thumb_at_top_when_fully_scrolled() {
    let mut state = fresh_state();
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 100; // clamped

    let (_thumb, offset) = state.snapshot().scrollbar_metrics(10);
    assert_eq!(offset, 0, "Thumb at top track edge when fully scrolled");
}

// ── Large multi-line element overflow ─────────────────────────────────

#[test]
fn large_thought_overflows_viewport() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "before".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.session.messages.push(thought_msg("t1", 30));
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "after".into(),
        }],
        timestamp: 2.0,
        id: "u2".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    state.view.scroll = 0;

    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 10);

    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "after")),
        "Latest message must be visible"
    );
    assert!(
        region.skip_lines >= 15,
        "Should skip many lines from large thought: got skip={}",
        region.skip_lines
    );
}

#[test]
fn multi_line_tool_at_bottom_visible() {
    let mut state = fresh_state();
    for i in 0..3 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    let mut tool = tool_msg("t1", 5);
    tool.timestamp = 3.0; // after all user messages
    state.session.messages.push(tool);
    state.refresh_after_message_change();

    state.view.scroll = 0;

    // Total: 3*4 + 7 = 19 lines (3 users with margins + 3 spacers + tool 6 lines + spacer)
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);

    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::ToolDone { .. })),
        "Tool must be visible at bottom"
    );
}

// ── Autoscroll behavior with line counts ──────────────────────────────

#[test]
fn new_message_at_bottom_auto_shows() {
    let mut state = fresh_state();
    // Fill with enough content to overflow
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
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

    // Add new message — total lines increases, but we're at bottom
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "newest".into(),
        }],
        timestamp: 100.0,
        id: "u99".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    assert!(
        region
            .elements
            .iter()
            .any(|e| matches!(e, Element::UserMessage { content, .. } if content == "newest")),
        "Newest message must be visible when at bottom"
    );
}

#[test]
fn scroll_preserved_when_not_at_bottom() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state.view.scroll = 5; // scrolled up

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "newest".into(),
        }],
        timestamp: 100.0,
        id: "u99".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // scroll preserved when not at bottom
    assert_eq!(
        state.view.scroll, 5,
        "Scroll position should be preserved when not at bottom"
    );
}
