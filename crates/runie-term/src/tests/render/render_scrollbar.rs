use super::super::*;

fn make_messages(count: usize) -> Vec<ChatMessage> {
    (0..count)
        .map(|i| ChatMessage {
            role: Role::User,
            content: format!("Message {} with some text here", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        })
        .collect()
}

#[test]
fn test_scrollbar_shows_when_content_overflows() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.messages = make_messages(20);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let area = buf.area();
    let scrollbar_col = 38;
    let bar_chars: Vec<String> = (1..area.height - 1)
        .map(|y| buf[(scrollbar_col, y)].symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "█" || s == "│"),
        "Scrollbar should render at col 38. Got: {:?}", bar_chars
    );
}

#[test]
fn test_scrollbar_thumb_at_bottom_by_default() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.messages = make_messages(20);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let scrollbar_col = 38;
    let area = buf.area();
    let bar_chars: Vec<String> = (1..area.height - 1)
        .map(|y| buf[(scrollbar_col, y)].symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "█"),
        "Thumb should be visible when content overflows. Bar chars: {:?}",
        bar_chars
    );
}

#[test]
fn test_scrollbar_moves_when_scrolled_up() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.messages = make_messages(50);

    // Render at bottom
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_bottom = terminal.backend().buffer().clone();

    // Scroll up enough to move thumb visibly
    for _ in 0..20 {
        state.update(Event::ScrollUp);
    }

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_scrolled = terminal.backend().buffer().clone();

    let area = buf_bottom.area();
    let right_col = area.width - 2;

    let bottom_thumb_y = (0..area.height)
        .find(|y| buf_bottom[(right_col, *y)].symbol() == "█")
        .expect("thumb at bottom");
    let scrolled_thumb_y = (0..area.height)
        .find(|y| buf_scrolled[(right_col, *y)].symbol() == "█")
        .expect("thumb when scrolled");

    assert!(
        scrolled_thumb_y < bottom_thumb_y,
        "Thumb should move up when scrolled. bottom_y={} scrolled_y={}",
        bottom_thumb_y, scrolled_thumb_y
    );
}

#[test]
fn test_no_scrollbar_when_content_fits() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
    });

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let scrollbar_col = 38;
    let area = buf.area();
    let has_thumb = (1..area.height - 1)
        .any(|y| buf[(scrollbar_col, y)].symbol() == "█");
    assert!(!has_thumb, "No scrollbar thumb when content fits");
}
