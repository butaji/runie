use super::super::*;
use crate::tests::connect_model;
use runie_core::Event;
use runie_core::Part;

fn make_messages(count: usize) -> Vec<ChatMessage> {
    (0..count)
        .map(|i| ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("Message {} with some text here", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        })
        .collect()
}

#[test]
fn test_scrollbar_shows_when_content_overflows() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    connect_model(&mut state);
    state.session.messages = make_messages(20);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let area = buf.area();
    let scrollbar_col = area.width - 1;
    let bar_chars: Vec<String> = (0..area.height)
        .map(|y| buf[(scrollbar_col, y)].symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "▐" || s == "│"),
        "Scrollbar should render at col {}. Got: {:?}",
        scrollbar_col,
        bar_chars
    );
}

#[test]
fn test_scrollbar_thumb_at_bottom_by_default() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    connect_model(&mut state);
    state.session.messages = make_messages(20);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let area = buf.area();
    let scrollbar_col = area.width - 1;
    let bar_chars: Vec<String> = (0..area.height)
        .map(|y| buf[(scrollbar_col, y)].symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "▐"),
        "Thumb should be visible when content overflows. Bar chars: {:?}",
        bar_chars
    );
}

#[test]
fn test_scrollbar_moves_when_scrolled_up() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    connect_model(&mut state);
    state.session.messages = make_messages(50);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_bottom = terminal.backend().buffer().clone();

    for _ in 0..20 {
        state.update(Event::Up);
    }

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_scrolled = terminal.backend().buffer().clone();

    let area = buf_bottom.area();
    // Scrollbar is rendered at width - 1 (rightmost column)
    let scrollbar_col = area.width - 1;

    // Find the thumb position in both buffers
    let bottom_thumb_y = (0..area.height)
        .find(|y| buf_bottom[(scrollbar_col, *y)].symbol() == "▐")
        .expect("thumb at bottom");
    let scrolled_thumb_y = (0..area.height)
        .find(|y| buf_scrolled[(scrollbar_col, *y)].symbol() == "▐")
        .expect("thumb when scrolled");

    assert!(
        scrolled_thumb_y < bottom_thumb_y,
        "Thumb should move up when scrolled. bottom_y={} scrolled_y={}",
        bottom_thumb_y,
        scrolled_thumb_y
    );
}

/// Tests that scrollbar shows when content exceeds viewport.
/// Note: Even a short message has 3 lines (content + top/bottom margin)
/// plus 1 spacer, totaling 4 lines. With messages_area height of 3,
/// this means scrollbar WILL show even for single short messages.
#[test]
fn test_scrollbar_shows_when_content_overflows_small() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    connect_model(&mut state);

    // Single short message - still overflows due to margins
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "Hi".into() }],
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let area = buf.area();
    let scrollbar_col = area.width - 1;

    // With margins and spacers, even short content overflows
    let has_scrollbar_content =
        (0..area.height).any(|y| buf[(scrollbar_col, y)].symbol() == "▐" || buf[(scrollbar_col, y)].symbol() == "│");
    assert!(
        has_scrollbar_content,
        "Scrollbar should show when content has margins"
    );
}
