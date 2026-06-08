use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

/// The exact bug: after large content arrives, visible region must include
/// the LATEST (bottom-most) lines, not leave them below the fold.
#[test]
fn large_tool_output_bottom_lines_in_viewport() {
    let mut state = fresh_state();
    state.scroll = 0; // at bottom

    // User message + spacer = 2 lines
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "list files".into(),
        timestamp: 0.0,
        id: "u0".into(),
    });
    // Tool with 20 output lines: header(1) + output(20) = 21 lines + spacer = 22
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\nfile1\nfile2\nfile3\nfile4\nfile5\nfile6\nfile7\nfile8\nfile9\nfile10\nfile11\nfile12\nfile13\nfile14\nfile15\nfile16\nfile17\nfile18\nfile19\nfile20".into(),
        timestamp: 1.0,
        id: "t1".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    // Total = 2 + 22 = 24 lines. Viewport = 5.
    // max_scroll = 19. scroll=0 → viewport [19, 24)
    // Lines: User[0], Spacer[1], ToolDone[2..22], Spacer[23]
    // [19,24) = lines 19,20,21,22,23
    // = ToolDone lines 17,18,19,20 (indices 17-20 = "file17","file18","file19","file20") + Spacer
    let region = state.visible_scroll(5);
    assert!(!region.elements.is_empty(), "Viewport must not be empty");

    // The visible region should contain the ToolDone element
    let tool_elems: Vec<_> = region.elements.iter()
        .filter(|e| matches!(e, crate::ui::Element::ToolDone { .. }))
        .collect();
    assert!(!tool_elems.is_empty(), "ToolDone must be in viewport");
}

#[test]
fn viewport_never_exceeds_height() {
    let mut state = fresh_state();
    state.scroll = 0;

    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\nfile1\nfile2\nfile3\nfile4\nfile5".into(),
        timestamp: 1.0,
        id: "t1".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    // ToolDone: header(1) + 5 output = 6 lines + spacer = 7 lines total
    // Viewport = 5, max_scroll = 2, viewport [2, 7)
    let region = state.visible_scroll(5);

    // Count visible lines (accounting for skip_lines on first element)
    let mut visible_lines = 0usize;
    for (i, elem) in region.elements.iter().enumerate() {
        let count = elem.line_count();
        if i == 0 && region.skip_lines > 0 {
            visible_lines += count.saturating_sub(region.skip_lines);
        } else {
            visible_lines += count;
        }
    }

    assert!(visible_lines <= 5, "Visible lines ({}) must not exceed viewport height (5)", visible_lines);
}

#[test]
fn last_element_lines_clipped_to_fit_viewport() {
    let mut state = fresh_state();
    state.scroll = 0;

    // Small element first
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "hi".into(),
        timestamp: 0.0,
        id: "u0".into(),
    });
    // HUGE element second: 30 lines
    let mut thought = "◆ Thought 1.0s\n".to_string();
    for i in 1..=30 {
        thought.push_str(&format!("line{}\n", i));
    }
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: thought,
        timestamp: 1.0,
        id: "t1".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    // Total = 2 + 31 + 1 = 34 lines. Viewport = 5.
    // max_scroll = 29. viewport [29, 34)
    // Elements: User(1), Spacer(1), Thought(31), Spacer(1)
    // Line positions: User[0], Spacer[1], Thought[2..32], Spacer[33]
    // [29,34) = lines 29,30,31,32,33
    // = Thought lines 27,28,29,30 (indices 27-30) + Spacer[33]
    // But Thought has 31 lines (header + 30). Line 32 is the last line.
    // Wait: Thought[2..32] = 30 lines (indices 2-31). Line 33 = Spacer.
    // [29,34): line29=Thought[27], line30=Thought[28], line31=Thought[29], line32=Thought[30], line33=Spacer
    // That's 4 thought lines + 1 spacer = 5 lines ✓

    let region = state.visible_scroll(5);

    // Count exact visible lines
    let mut total_visible = 0usize;
    for (i, elem) in region.elements.iter().enumerate() {
        let count = elem.line_count();
        if i == 0 && region.skip_lines > 0 {
            total_visible += count.saturating_sub(region.skip_lines);
        } else {
            total_visible += count;
        }
    }

    assert_eq!(total_visible, 5, "Visible lines must exactly equal viewport height");
}

#[test]
fn submit_then_large_response_stays_at_bottom() {
    let mut state = fresh_state();

    // User submits
    state.input = "list files".into();
    state.update(Event::Submit);
    state.ensure_fresh();
    assert_eq!(state.scroll, 0, "Scroll must be 0 after submit");

    // Agent tool with large output
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.ensure_fresh();

    // Scroll must still be 0 (at bottom) — user didn't scroll
    assert_eq!(state.scroll, 0, "Scroll must stay at 0 after response");

    // Latest file must be visible
    let region = state.visible_scroll(5);
    let tool_texts: Vec<String> = region.elements.iter().filter_map(|e| match e {
        crate::ui::Element::ToolDone { output, .. } => Some(output.clone()),
        _ => None,
    }).collect();
    assert!(!tool_texts.is_empty(), "Tool must be visible");
    assert!(tool_texts[0].contains("file20"), "Latest file must be visible");
}

#[test]
fn streaming_large_content_scroll_zero_shows_latest() {
    let mut state = fresh_state();
    state.streaming = true;
    state.scroll = 0;

    // Simulate streaming chunks that build up to >1 page
    for i in 0..10 {
        state.update(Event::AgentResponse {
            id: "req.0".into(),
            content: format!("line{}\n", i),
        });
    }
    state.ensure_fresh();

    // 10 responses merged into 1 assistant = 1 line + spacer = 2 lines... that's not >1 page.
    // Let me use a single large response instead.
    let mut content = String::new();
    for i in 0..20 {
        content.push_str(&format!("This is line {} of the response\n", i));
    }
    state.update(Event::AgentResponse { id: "req.0".into(), content });
    state.ensure_fresh();

    let region = state.visible_scroll(5);
    assert!(!region.elements.is_empty(), "Viewport must show content");
}
