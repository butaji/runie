use super::super::*;
use super::*;
use runie_core::Event;
use runie_core::Part;
use std::time::Instant;

#[test]
fn test_view_renders_user_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);

    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("❯ Hi"),
        "Chat must render '❯ Hi'. Got: {}",
        content
    );
    assert!(
        content.contains("Hi"),
        "Chat must render content. Got: {}",
        content
    );
}

#[test]
fn test_view_renders_agent_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });

    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("→ Hello"),
        "Must render '→ Hello'. Got: {}",
        content
    );
}

#[test]
fn test_view_renders_multiple_messages_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Response 1".to_string(),
    });
    state.update(Event::Done {
        id: "req.0".to_string(),
    });
    state.update(Event::Input('B'));
    state.update(Event::Submit);

    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(content.contains("❯ A"), "Must show user prefix");
    assert!(content.contains("→ Response 1"), "Must show agent prefix");
}

#[test]
fn test_render_user_message() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(content.contains("❯ H"));
}

#[test]
fn test_render_agent_response() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.agent.streaming = true;
    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(content.contains("→ Hello"));
}

#[test]
fn test_user_message_is_left_aligned_with_chevron() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.input.input = "left aligned".to_string();
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).unwrap();

    let buf = terminal.backend().buffer();
    let mut found_x: Option<u16> = None;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "l" {
                // First char of "left aligned".
                found_x = Some(x);
                break;
            }
        }
        if found_x.is_some() {
            break;
        }
    }
    let x = found_x.expect("user content must be rendered");
    assert!(
        x < 10,
        "User message should be left-aligned with chevron prefix, got x={}",
        x
    );

    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("❯ left aligned"),
        "User message should render with chevron prefix"
    );
}

#[test]
fn test_agent_message_is_left_aligned_plain_text() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "left side".to_string(),
    });
    terminal.draw(|f| view(f, &mut state)).unwrap();

    let buf = terminal.backend().buffer();
    let mut found_x: Option<u16> = None;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "l" {
                // First char of "left side".
                found_x = Some(x);
                break;
            }
        }
        if found_x.is_some() {
            break;
        }
    }
    let x = found_x.expect("agent content must be rendered");
    assert!(
        x < 10,
        "Agent message should be left-aligned plain text, got x={}",
        x
    );
}

#[test]
fn test_render_thinking_indicator() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("◐"),
        "Thinking should show spinner ◐. Got: {}",
        content
    );
}

#[test]
fn test_render_performance_1000_messages() {
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    for i in 0..200 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("Message {} from user with some content here", i),
            }],
            timestamp: 0.0,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: format!("Response {} from agent with detailed explanation", i),
            }],
            timestamp: 0.0,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    let start = Instant::now();
    for _ in 0..100 {
        terminal.draw(|f| view(f, &mut state)).unwrap();
    }
    let elapsed = start.elapsed();
    println!(
        "100 renders with {} messages: {:?}",
        state.session.messages.len(),
        elapsed
    );
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "Rendering too slow: {:?}",
        elapsed
    );
}

#[test]
fn test_stress_many_tool_calls() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    for i in 0..20 {
        simulate_tool_call(&mut state, i);
        state.update(Event::TurnComplete {
            id: format!("req.{}", i),
            duration_secs: 1.0,
        });
        state.update(Event::Done {
            id: format!("req.{}", i),
        });
        if i % 5 == 0 {
            terminal.draw(|f| view(f, &mut state)).expect("draw");
        }
    }
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Files for turn"));
    assert!(
        state.session.messages.len() >= 100,
        "many messages, got {}",
        state.session.messages.len()
    );
}

#[test]
fn think_blocks_not_rendered() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    // Response contains think tags
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "<think>hidden thought</think>visible answer".to_string(),
    });
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(
        !content.contains("<think>"),
        "Think tags should not appear in rendered content"
    );
    assert!(
        !content.contains("</think>"),
        "Think tags should not appear in rendered content"
    );
    assert!(
        content.contains("visible answer"),
        "Visible content should be rendered"
    );
}
