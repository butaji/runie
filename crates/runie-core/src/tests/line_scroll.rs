use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::Element;

fn fresh_state() -> AppState {
    AppState::default()
}

// Helper: build a thought with N lines of reasoning
fn thought_msg(id: &str, n_lines: usize) -> ChatMessage {
    let content = std::iter::once("◆ Thought 1.0s".to_string())
        .chain((1..=n_lines).map(|i| format!("line{}", i)))
        .collect::<Vec<_>>()
        .join("\n");
    ChatMessage {
        role: Role::Thought,
        content,
        timestamp: 1.0,
        id: id.to_string(),
    }
}

// Helper: build a tool with N lines of output
fn tool_msg(id: &str, n_output_lines: usize) -> ChatMessage {
    let output = (1..=n_output_lines).map(|i| format!("out{}", i)).collect::<Vec<_>>().join("\n");
    let content = format!("◆ Ran ls 0.5s\n{}", output);
    ChatMessage {
        role: Role::Tool,
        content,
        timestamp: 1.0,
        id: id.to_string(),
    }
}

// ── Line count basics ─────────────────────────────────────────────────

#[test]
fn user_message_is_one_line() {
    let msg = ChatMessage { role: Role::User, content: "hello".into(), timestamp: 0.0, id: "u".into() };
    let mut state = fresh_state();
    state.messages.push(msg);
    state.messages_changed();
    state.ensure_fresh();

    assert_eq!(state.total_lines(), 2, "UserMessage (1) + Spacer (1) = 2 lines");
}

#[test]
fn thought_line_count_matches_content() {
    let mut state = fresh_state();
    // header + 5 lines = 6 lines of content, + 1 spacer = 7 total element lines
    state.messages.push(thought_msg("t1", 5));
    state.messages_changed();
    state.ensure_fresh();

    let total = state.total_lines();
    // ThoughtMarker has 6 lines (header + 5), + Spacer = 7
    assert_eq!(total, 7, "Thought with 5 content lines should be 6+1=7 lines total");
}

#[test]
fn tool_line_count_matches_output() {
    let mut state = fresh_state();
    state.messages.push(tool_msg("x1", 3));
    state.messages_changed();
    state.ensure_fresh();

    let total = state.total_lines();
    // ToolDone: header (1) + output (3) = 4, + Spacer = 5
    assert_eq!(total, 5, "Tool with 3 output lines should be 4+1=5 lines total");
}

// ── Visible region: latest at bottom ──────────────────────────────────

#[test]
fn visible_shows_latest_element_at_bottom() {
    let mut state = fresh_state();
    for i in 0..3 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0; // at bottom

    let region = state.visible_scroll(3); // 3 lines viewport
    // 3 messages = 3 UserMessage + 3 Spacer = 6 lines total
    // Viewport of 3 lines at bottom shows: msg2 (1) + spacer (1) + msg1 (1) = 3 lines exactly
    assert!(
        region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg2")),
        "Latest message (msg2) must be in visible region"
    );
}

#[test]
fn visible_skips_lines_from_first_element_when_overflow() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "first".into(),
        timestamp: 0.0,
        id: "u0".into(),
    });
    state.messages.push(thought_msg("t1", 10)); // 11 lines of thought
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "latest".into(),
        timestamp: 2.0,
        id: "u2".into(),
    });
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    // Total: msg0(1)+spacer(1) + thought(11)+spacer(1) + msg2(1)+spacer(1) = 17 lines
    // Wait: thought_msg header "◆ Thought 1.0s" + 5 lines = 6 lines, not 11
    // Actually thought_msg("t1", 5) = "◆ Thought 1.0s" + line1..line5 = 6 lines
    // So total: msg0(1)+spacer(1) + thought(6)+spacer(1) + msg2(1)+spacer(1) = 11 lines
    // Viewport of 5 lines at bottom:
    // Bottom-up: msg2(1) + spacer(1) = 2 lines used, 3 remaining
    //            thought: need 3 lines from bottom of thought
    //            thought has 11 lines, so skip 11-3 = 8 lines from top
    let region = state.visible_scroll(5);

    assert!(
        region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "latest")),
        "Latest message must be visible"
    );
    assert!(
        region.elements.iter().any(|e| matches!(e, Element::ThoughtMarker { .. })),
        "Thought must be partially visible"
    );
    // The first visible element should be the thought, and we should skip some lines
    assert!(region.skip_lines > 0, "Should skip lines from top of first visible element");
}

#[test]
fn scroll_up_shows_older_content() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    // 5 messages = 10 lines total (5 messages + 5 spacers). Viewport of 3 lines.
    // Lines: 0:msg0, 1:spacer, 2:msg1, 3:spacer, 4:msg2, 5:spacer, 6:msg3, 7:spacer, 8:msg4, 9:spacer
    // Total 10 lines. viewport=3.
    // scroll=0: viewport [7, 10) → lines 7,8,9 = spacer(msg3), msg4, spacer(msg4) — msg4 visible
    // scroll=2: viewport [5, 8) → lines 5,6,7 = msg2, spacer, msg3 — msg2 visible, msg4 hidden

    // scroll=5: viewport [2, 5) → msg1, spacer, msg2 — msg2 visible, msg4 hidden
    state.scroll = 5;
    let region = state.visible_scroll(3);
    assert!(
        region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg2")),
        "Scroll up should show older message (msg2)"
    );
    assert!(
        !region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "msg4")),
        "Scroll up should hide msg4"
    );
}

// ── Scrollbar line-based ──────────────────────────────────────────────

#[test]
fn scrollbar_no_scrollbar_when_lines_fit() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage { role: Role::User, content: "hi".into(), timestamp: 0.0, id: "u".into() });
    state.messages_changed();
    state.ensure_fresh();

    let (thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(thumb, 0, "No scrollbar when total lines fit in viewport");
    assert_eq!(offset, 0);
}

#[test]
fn scrollbar_shows_when_lines_overflow() {
    let mut state = fresh_state();
    // 20 messages = 40 lines, viewport = 10
    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();

    let (thumb, _offset) = state.scrollbar_metrics(10);
    assert!(thumb > 0, "Scrollbar thumb should show when line count exceeds viewport");
}

#[test]
fn scrollbar_thumb_at_bottom_when_not_scrolled() {
    let mut state = fresh_state();
    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    let (thumb, offset) = state.scrollbar_metrics(10);
    // 40 lines total, viewport 10, max_scroll = 30
    // thumb = max(1, 10*10/40) = max(1, 2) = 2
    // offset = (30 - 0) * (10 - 2) / 30 = 30 * 8 / 30 = 8
    assert_eq!(offset, 10 - thumb, "Thumb at bottom track edge when scroll=0");
}

#[test]
fn scrollbar_thumb_at_top_when_fully_scrolled() {
    let mut state = fresh_state();
    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 100; // clamped

    let (_thumb, offset) = state.scrollbar_metrics(10);
    assert_eq!(offset, 0, "Thumb at top track edge when fully scrolled");
}

// ── Large multi-line element overflow ─────────────────────────────────

#[test]
fn large_thought_overflows_viewport() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "before".into(),
        timestamp: 0.0,
        id: "u0".into(),
    });
    // Thought with 20 lines of content
    state.messages.push(thought_msg("t1", 20));
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "after".into(),
        timestamp: 2.0,
        id: "u2".into(),
    });
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    // Total: before(1)+spacer(1) + thought(21)+spacer(1) + after(1)+spacer(1) = 26 lines
    // Viewport of 5 lines at bottom:
    // after(1) + spacer(1) = 2, need 3 more from thought
    // thought has 21 lines, so skip 21-3 = 18 from top
    let region = state.visible_scroll(5);

    assert!(
        region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "after")),
        "Latest message must be visible"
    );
    // skip_lines should be large since we're deep into the thought
    assert!(region.skip_lines >= 15, "Should skip many lines from large thought: got skip={}", region.skip_lines);
}

#[test]
fn multi_line_tool_at_bottom_visible() {
    let mut state = fresh_state();
    for i in 0..3 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages.push(tool_msg("t1", 5));
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    // Total: 3*2 + 6+1 = 6 + 7 = 13 lines (3 users + 3 spacers + tool header + 5 outputs + spacer)
    // Wait: tool_msg header = "◆ Ran ls 0.5s" (1 line) + 5 output lines = 6 lines for ToolDone + 1 spacer = 7
    // 3 users: 3*2 = 6 lines. Total = 13 lines.
    // Viewport of 5 lines at bottom: should include the tool
    let region = state.visible_scroll(5);

    assert!(
        region.elements.iter().any(|e| matches!(e, Element::ToolDone { .. })),
        "Tool must be visible at bottom"
    );
}

// ── Autoscroll behavior with line counts ──────────────────────────────

#[test]
fn new_message_at_bottom_auto_shows() {
    let mut state = fresh_state();
    // Fill with enough content to overflow
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    // Add new message — total lines increases, but we're at bottom
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "newest".into(),
        timestamp: 100.0,
        id: "u99".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    let region = state.visible_scroll(5);
    assert!(
        region.elements.iter().any(|e| matches!(e, Element::UserMessage { content, .. } if content == "newest")),
        "Newest message must be visible when at bottom"
    );
}

#[test]
fn scroll_preserved_when_not_at_bottom() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 5; // scrolled up

    state.messages.push(ChatMessage {
        role: Role::User,
        content: "newest".into(),
        timestamp: 100.0,
        id: "u99".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    // scroll preserved when not at bottom
    assert_eq!(state.scroll, 5, "Scroll position should be preserved when not at bottom");
}
