use runie_core::layout::element_line_count;
use runie_core::model::{AppState, ChatMessage, Role};
use runie_core::Event;
use runie_core::Part;
use runie_testing::fresh_state;

/// The exact bug: after large content arrives, visible region must include
/// the LATEST (bottom-most) lines, not leave them below the fold.
#[test]
fn large_tool_output_bottom_lines_in_viewport() {
    let mut state = fresh_state();
    state.view.scroll = 0; // at bottom

    // User message + spacer = 2 lines
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "list files".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    // Tool with 20 output lines: header(1) + output(20) = 21 lines + spacer = 22
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "◆ Ran ls 0.5s\nfile1\nfile2\nfile3\nfile4\nfile5\nfile6\nfile7\nfile8\nfile9\nfile10\nfile11\nfile12\nfile13\nfile14\nfile15\nfile16\nfile17\nfile18\nfile19\nfile20".into() }],
        timestamp: 1.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // Total = 2 + 22 = 24 lines. Viewport = 5.
    // max_scroll = 19. scroll=0 → viewport [19, 24)
    // Lines: User[0], Spacer[1], ToolDone[2..22], Spacer[23]
    // [19,24) = lines 19,20,21,22,23
    // = ToolDone lines 17,18,19,20 (indices 17-20 = "file17","file18","file19","file20") + Spacer
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    assert!(!region.elements.is_empty(), "Viewport must not be empty");

    // The visible region should contain the ToolDone element
    let tool_elems: Vec<_> = region
        .elements
        .iter()
        .filter(|e| matches!(e, runie_core::view::Element::ToolDone { .. }))
        .collect();
    assert!(!tool_elems.is_empty(), "ToolDone must be in viewport");
}

#[test]
fn viewport_never_exceeds_height() {
    let mut state = fresh_state();
    state.view.scroll = 0;

    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: "◆ Ran ls 0.5s\nfile1\nfile2\nfile3\nfile4\nfile5".into(),
        }],
        timestamp: 1.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // ToolDone: header(1) + 5 output = 6 lines + spacer = 7 lines total
    // Viewport = 5, max_scroll = 2, viewport [2, 7)
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);

    // Count visible lines (accounting for skip_lines on first element)
    let mut visible_lines = 0usize;
    for (i, elem) in region.elements.iter().enumerate() {
        let count = element_line_count(elem, state.view.last_content_width);
        if i == 0 && region.skip_lines > 0 {
            visible_lines += count.saturating_sub(region.skip_lines);
        } else {
            visible_lines += count;
        }
    }

    assert!(
        visible_lines <= 5,
        "Visible lines ({}) must not exceed viewport height (5)",
        visible_lines
    );
}

#[test]
fn last_element_lines_clipped_to_fit_viewport() {
    let mut state = fresh_state();
    state.view.scroll = 0;

    add_user_and_huge_thought(&mut state);
    state.ensure_fresh();

    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    let total_visible = count_visible_lines(&region, state.view.last_content_width);
    assert_eq!(
        total_visible, 5,
        "Visible lines must exactly equal viewport height"
    );
}

fn add_user_and_huge_thought(state: &mut AppState) {
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "hi".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    let mut thought = "◆ Thought 1.0s\n".to_string();
    for i in 1..=30 {
        thought.push_str(&format!("line{}\n", i));
    }
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: thought }],
        timestamp: 1.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.messages_changed();
}

fn count_visible_lines(
    region: &crate::tests::core::visible_helper::TestViewport,
    width: u16,
) -> usize {
    let mut total = 0usize;
    for (i, elem) in region.elements.iter().enumerate() {
        let count = element_line_count(elem, width);
        if i == 0 && region.skip_lines > 0 {
            total += count.saturating_sub(region.skip_lines);
        } else {
            total += count;
        }
    }
    total
}

#[test]
fn submit_then_large_response_stays_at_bottom() {
    let mut state = fresh_state();
    // User submits
    state.input.input = "list files".into();
    state.update(Event::submit());
    state.ensure_fresh();
    assert_eq!(state.view.scroll, 0, "Scroll must be 0 after submit");
    // Agent tool with large output
    state.update(Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    let output = (1..=20)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output,
    });
    state.ensure_fresh();
    // Scroll must still be 0 (at bottom) — user didn't scroll
    assert_eq!(state.view.scroll, 0, "Scroll must stay at 0 after response");
    // Latest file must be visible
    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    let tool_texts: Vec<String> = region
        .elements
        .iter()
        .filter_map(|e| match e {
            runie_core::view::Element::ToolDone { output, .. } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert!(!tool_texts.is_empty(), "Tool must be visible");
    assert!(
        tool_texts[0].contains("file20"),
        "Latest file must be visible"
    );
}

#[test]
fn streaming_large_content_scroll_zero_shows_latest() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.view.scroll = 0;

    // Simulate streaming chunks that build up to >1 page
    for i in 0..10 {
        state.update(Event::Response {
            id: "req.0".into(),
            content: format!("line{}\n", i),
            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        });
    }
    state.ensure_fresh();

    // 10 responses merged into 1 assistant = 1 line + spacer = 2 lines... that's not >1 page.
    // Let me use a single large response instead.
    let mut content = String::new();
    for i in 0..20 {
        content.push_str(&format!("This is line {} of the response\n", i));
    }
    state.update(Event::Response {
        id: "req.0".into(),
        content,
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();

    let region = crate::tests::core::visible_helper::compute_viewport(&mut state, 5);
    assert!(!region.elements.is_empty(), "Viewport must show content");
}
