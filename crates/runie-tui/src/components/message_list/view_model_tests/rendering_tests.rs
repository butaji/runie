//! Tests for MessageList rendering

use super::helper_helpers::*;

#[test]
fn test_render_empty_state_does_not_panic() {
    use crate::components::message_list::render::render_empty_state;

    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    render_empty_state(
        area,
        &mut buf,
        ratatui::style::Color::DarkGray,
        ratatui::style::Color::Gray,
        area.x + 4,
    );
    let non_empty = buf.content().iter().any(|c| c.symbol() != " ");
    assert!(non_empty, "Empty state should render some visible characters");
}

#[test]
fn test_assistant_empty_agent_running_shows_thinking() {
    let (row_text, _, _) = render_assistant_message("", Vec::new(), true, None);
    assert!(row_text.contains("Thinking"), "Expected 'Thinking' indicator, got: '{}'", row_text.trim());
}

#[test]
fn test_assistant_empty_no_agent_running_shows_dot() {
    let area = Rect::new(0, 0, 80, 24);
    let (_, buf, _) = render_assistant_message("", Vec::new(), false, None);
    let cell = buf.cell((area.x + 2, area.y)).unwrap();
    assert_eq!(cell.symbol(), "·", "Expected '·' when agent not running");
}

#[test]
fn test_assistant_non_empty_shows_text() {
    let (row_text, _, _) = render_assistant_message("Hello world", Vec::new(), true, None);
    assert!(row_text.contains("Hello world"), "Expected 'Hello world' in row, got: '{}'", row_text.trim());
}

#[test]
fn test_user_message_renders() {
    let area = Rect::new(0, 0, 80, 24);
    let (_, buf, _) = render_user_message("Hello");
    let cell = buf.cell((area.x + 2, area.y)).unwrap();
    assert_eq!(cell.symbol(), "\u{203A}", "Expected chevron for user message");
}

#[test]
fn test_user_message_never_shows_thinking_indicator() {
    // User message should NEVER show "Thinking..." regardless of agent_running state
    let (row_text, _, _) = render_user_message_with_agent("hey!", true);
    assert!(row_text.contains("hey!"), "Expected 'hey!' in user message, got: '{}'", row_text.trim());
    assert!(!row_text.contains("Thinking"), "User message should NOT show 'Thinking' indicator, got: '{}'", row_text.trim());
}

#[test]
fn test_user_message_only_shows_text_no_status() {
    // User message should only show text content, no status indicators
    let (row_text, _, _) = render_user_message_with_agent("hello world", true);
    assert!(row_text.contains("hello world"), "Expected 'hello world' in user message");
    assert!(!row_text.contains("Thinking"), "User message must not contain 'Thinking'");
    assert!(!row_text.contains("·"), "User message must not contain status dot");
}

#[test]
fn test_user_message_chevron_color_matches_input_border() {
    // Verify chevron uses border.unfocused color (same as input box border)
    let theme = ThemeWrapper::default_for_test();
    let expected_chevron_color: ratatui::style::Color = theme.color("border.unfocused").into();

    let area = Rect::new(0, 0, 80, 24);
    let (_, buf, _) = render_user_message("Hello");
    let cell = buf.cell((area.x + 2, area.y)).unwrap();

    let chevron_fg = cell.style().fg.unwrap();
    assert_eq!(
        chevron_fg, expected_chevron_color,
        "Chevron foreground should match border.unfocused (input box border color)"
    );
}

#[test]
fn test_system_notice_renders() {
    let row_text = render_system_notice("System message");
    assert!(row_text.contains("System message"), "Expected 'System message' in row");
}

#[test]
fn test_assistant_with_thought_duration() {
    let thoughts = vec![Thought { duration: 1.5 }];
    let (row_text, _, _) = render_assistant_message("Response", thoughts, false, None);
    assert!(row_text.contains("Thought"), "Expected 'Thought' indicator in row");
}
