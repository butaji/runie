//! Tests for MessageList ViewModel and rendering helpers.

use crate::components::message_list::types::{MessageItem, MessageList};
use crate::components::message_list::feed::{Feed, FeedItem, Thought};
use crate::tui::state::AnimationState;
use ratatui::{buffer::Buffer, layout::Rect};
use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
use crate::theme::ThemeWrapper;

/// Create an empty buffer and wrap cache for testing
fn make_test_buffer(area: Rect) -> (Buffer, WrapCache) {
    (Buffer::empty(area), WrapCache::new())
}

/// Extract row text from buffer at given area's y coordinate
fn get_row_text(buf: &Buffer, area: Rect) -> String {
    (0..area.width)
        .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
        .collect()
}

/// Render an assistant message and return (row_text, buf, area)
fn render_assistant_message(
    text: &str,
    thoughts: Vec<Thought>,
    agent_running: bool,
    thought_duration: Option<f32>,
) -> (String, Buffer, Rect) {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let theme = ThemeWrapper::default_for_test();
    let mut wrap_cache = WrapCache::new();

    let item = FeedItem::AssistantMessage {
        id: "test".to_string(),
        text: text.to_string(),
        thoughts,
        tool_calls: Vec::new(),
        timestamp: None,
        turn_duration: None,
    };

    render_single_msg_feed(
        &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
        &theme,
        ratatui::style::Color::White,
        ratatui::style::Color::Gray,
        ratatui::style::Color::DarkGray,
        ratatui::style::Color::Black,
        ratatui::style::Color::Green,
        ratatui::style::Color::Red,
        ratatui::style::Color::Blue,
        '⠋',
        false,
        false,
        '⠏',
        &AnimationState::default(),
        &mut wrap_cache,
        agent_running,
        thought_duration,
        None,
    );

    (get_row_text(&buf, area), buf, area)
}

/// Render a user message and return (row_text, buf, area)
fn render_user_message(text: &str) -> (String, Buffer, Rect) {
    render_user_message_with_agent(text, false)
}

/// Render a user message with agent_running=true/false and return (row_text, buf, area)
fn render_user_message_with_agent(text: &str, agent_running: bool) -> (String, Buffer, Rect) {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let theme = ThemeWrapper::default_for_test();
    let mut wrap_cache = WrapCache::new();

    let item = FeedItem::UserMessage {
        id: "test".to_string(),
        text: text.to_string(),
        timestamp: None,
    };

    render_single_msg_feed(
        &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
        &theme,
        ratatui::style::Color::White,
        ratatui::style::Color::Gray,
        ratatui::style::Color::DarkGray,
        ratatui::style::Color::Black,
        ratatui::style::Color::Green,
        ratatui::style::Color::Red,
        ratatui::style::Color::Blue,
        '⠋',
        false,
        false,
        '⠏',
        &AnimationState::default(),
        &mut wrap_cache,
        agent_running,
        None,
        None,
    );

    (get_row_text(&buf, area), buf, area)
}

/// Render a system notice and return the row text
fn render_system_notice(text: &str) -> String {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let theme = ThemeWrapper::default_for_test();
    let mut wrap_cache = WrapCache::new();

    let item = FeedItem::SystemNotice { text: text.to_string() };

    render_single_msg_feed(
        &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
        &theme,
        ratatui::style::Color::White,
        ratatui::style::Color::Gray,
        ratatui::style::Color::DarkGray,
        ratatui::style::Color::Black,
        ratatui::style::Color::Green,
        ratatui::style::Color::Red,
        ratatui::style::Color::Blue,
        '⠋',
        false,
        false,
        '⠏',
        &AnimationState::default(),
        &mut wrap_cache,
        false,
        None,
        None,
    );

    get_row_text(&buf, area)
}

/// Render a FeedItem and return (row_text, buf, area)
pub fn render_feed_item(item: &FeedItem, agent_running: bool) -> (String, Buffer, Rect) {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let theme = ThemeWrapper::default_for_test();
    let mut wrap_cache = WrapCache::new();

    let rendered = render_single_msg_feed(
        item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
        &theme,
        ratatui::style::Color::White,
        ratatui::style::Color::Gray,
        ratatui::style::Color::DarkGray,
        ratatui::style::Color::Black,
        ratatui::style::Color::Green,
        ratatui::style::Color::Red,
        ratatui::style::Color::Blue,
        '⠋',
        false,
        false,
        '⠏',
        &AnimationState::default(),
        &mut wrap_cache,
        agent_running,
        None,
        None,
    );

    (get_row_text(&buf, area), buf, area)
}
