use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;
pub mod types;
pub mod render;
pub mod builder;
pub mod feed;

pub use types::{MessageItem, MessageList, PlanStatus, BRAILLE_FRAMES, REVERSE_BRAILLE_FRAMES};
pub use builder::FeedBuilder;
pub use feed::{Feed, FeedItem, Thought, ToolCall};

/// ViewModel for rendering MessageList
pub struct MessageListViewModel {
    pub feed: Feed,
    pub scroll_offset: usize,
    pub agent_running: bool,
    pub animation: AnimationState,
    pub wrap_cache: WrapCache,
}

impl MessageListViewModel {
    pub fn new(feed: Feed, scroll_offset: usize, agent_running: bool, animation: AnimationState, wrap_cache: WrapCache) -> Self {
        Self {
            feed,
            scroll_offset,
            agent_running,
            animation,
            wrap_cache,
        }
    }
}

impl MessageList {
    pub fn render_ref(
        vm: &MessageListViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        let colors = extract_message_colors(theme);
        let mut wrap_cache = vm.wrap_cache.clone();
        let spinner = BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let rewind_spinner = REVERSE_BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let mut row = render_message_list(vm, area, buf, theme, &colors, spinner, rewind_spinner, &mut wrap_cache);

        if vm.feed.is_empty() && !vm.agent_running {
            render::render_empty_state(area, buf, colors.text_muted, colors.text_dim, area.x + 4);
        }
    }

    pub fn update_last_assistant(&mut self, new_text: &str) {
        if let Some(last) = self.messages.last_mut() {
            if let MessageItem::Assistant { ref mut text, .. } = last {
                *text = new_text.to_string();
            }
        }
    }

    pub fn has_assistant_in_progress(&self) -> bool {
        matches!(self.messages.last(), Some(MessageItem::Assistant { .. }))
    }

    pub fn add_or_update_assistant(&mut self, text: &str, model: Option<String>) {
        if let Some(last) = self.messages.last_mut() {
            if let MessageItem::Assistant { text: ref mut existing_text, .. } = last {
                *existing_text = text.to_string();
                return;
            }
        }
        self.messages.push(MessageItem::Assistant {
            text: text.to_string(),
            model,
            timestamp: None,
        });
    }
}

struct MessageColors {
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
}

fn extract_message_colors(theme: &ThemeWrapper) -> MessageColors {
    MessageColors {
        accent_primary: theme.color("accent.primary").into(),
        text_secondary: theme.color("text.secondary").into(),
        text_muted: theme.color("text.muted").into(),
        text_dim: theme.color("text.dim").into(),
        success: theme.color("success").into(),
        error: theme.color("error").into(),
        code_path: theme.color("code.path").into(),
    }
}

fn render_message_list(
    vm: &MessageListViewModel,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    colors: &MessageColors,
    spinner: char,
    rewind_spinner: char,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let mut row = 0u16;
    let max_rows = area.height;
    let margin_x = area.x + 2;
    let text_x = area.x + 4;
    let total_items = vm.feed.len();

    for (idx, item) in vm.feed.items().iter().skip(vm.scroll_offset).enumerate() {
        if row >= max_rows { break; }
        let absolute_idx = vm.scroll_offset + idx;

        let show_cursor = render::should_show_cursor_feed(&vm.animation, vm.agent_running, absolute_idx, total_items, item);
        let show_spinner = false; // Spinners for ToolRunning/PlanStep not rendered via Feed

        // For AssistantMessage, get thought duration from inline thoughts
        let thought_duration = if let FeedItem::AssistantMessage { thoughts, .. } = item {
            thoughts.first().map(|t| t.duration)
        } else {
            None
        };

        // Get turn_duration from AssistantMessage
        let turn_duration = if let FeedItem::AssistantMessage { turn_duration, .. } = item {
            *turn_duration
        } else {
            None
        };

        let rendered = render_single_msg_item(
            item, area, row, margin_x, text_x, max_rows, buf, theme, colors, spinner, show_cursor, show_spinner, rewind_spinner,
            &vm.animation, wrap_cache, vm.agent_running, thought_duration, turn_duration,
        );
        row += rendered;
    }
    row
}

fn render_single_msg_item(
    item: &FeedItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    colors: &MessageColors,
    spinner: char,
    show_cursor: bool,
    show_spinner: bool,
    rewind_spinner: char,
    animation: &AnimationState,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
    thought_duration: Option<f32>,
    turn_complete: Option<f32>,
) -> u16 {
    render::render_single_msg_feed(
        item, area, row, margin_x, text_x, max_rows, buf, theme,
        colors.accent_primary, colors.text_secondary, colors.text_muted, colors.text_dim,
        colors.success, colors.error, colors.code_path, spinner, show_cursor, show_spinner, rewind_spinner,
        animation, wrap_cache, agent_running, thought_duration, turn_complete,
    )
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_last_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        list.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
        list.update_last_assistant("Hi there");
        assert_eq!(list.messages.last(), Some(&MessageItem::Assistant { text: "Hi there".to_string(), model: Some("gpt-4".to_string()), timestamp: None }));
    }

    #[test]
    fn test_add_or_update_assistant_updates_existing() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant { text: "Partial".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
        list.add_or_update_assistant("Complete response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 1);
        assert_eq!(list.messages[0], MessageItem::Assistant { text: "Complete response".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
    }

    #[test]
    fn test_add_or_update_assistant_adds_new() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        list.add_or_update_assistant("Response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 2);
        assert_eq!(list.messages[1], MessageItem::Assistant { text: "Response".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
    }

    #[test]
    fn test_has_assistant_in_progress() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant { text: "Thinking...".to_string(), model: None, timestamp: None });
        assert!(list.has_assistant_in_progress());
        list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        assert!(!list.has_assistant_in_progress());
    }

    #[test]
    fn test_update_last_assistant_no_op_when_no_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        list.update_last_assistant("This should not change anything");
        assert_eq!(list.messages[0], MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    }

    #[test]
    fn test_render_empty_state_does_not_panic() {
        use ratatui::{
            buffer::Buffer,
            layout::Rect,
        };
        use crate::components::message_list::render::render_empty_state;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        // Should not panic — just verify the function exists and can be called
        render_empty_state(
            area,
            &mut buf,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Gray,
            area.x + 4,
        );
        // Verify that some characters were rendered
        let non_empty = buf.content().iter().any(|c| c.symbol() != " ");
        assert!(non_empty, "Empty state should render some visible characters");
    }

    #[test]
    fn test_assistant_empty_agent_running_shows_thinking() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
        };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            true, // agent_running
            None, // thought_duration
            None, // turn_complete
        );

        // Should render "⠋ Thinking..." instead of "·"
        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect();
        assert!(row_text.contains("Thinking"), "Expected 'Thinking' indicator in row, got: '{}'", row_text.trim());
    }

    #[test]
    fn test_assistant_empty_no_agent_running_shows_dot() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
        };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            false, // agent_running
            None, // thought_duration
            None, // turn_complete
        );

        // Should render "·" when agent not running
        let cell = buf.cell((area.x + 2, area.y)).unwrap();
        let symbol = cell.symbol();
        assert_eq!(symbol, "·", "Expected '·' when agent not running, got: {}", symbol);
    }

    #[test]
    fn test_assistant_non_empty_shows_text() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: "Hello world".to_string(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
        };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            true, // agent_running - should be ignored for non-empty text
            None, // thought_duration
            None, // turn_complete
        );

        // Should render the actual text
        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect::<String>();
        assert!(row_text.contains("Hello world"), "Expected 'Hello world' in row, got: '{}'", row_text.trim());
    }

    #[test]
    fn test_user_message_renders() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::UserMessage {
            id: "test".to_string(),
            text: "Hello".to_string(),
            timestamp: None,
        };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            false, // agent_running
            None, // thought_duration
            None, // turn_complete
        );

        // Should render chevron and text
        let cell = buf.cell((area.x + 2, area.y)).unwrap();
        let symbol = cell.symbol();
        assert_eq!(symbol, "\u{203A}", "Expected chevron for user message, got: {}", symbol);
    }

    #[test]
    fn test_system_notice_renders() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::SystemNotice { text: "System message".to_string() };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            false, // agent_running
            None, // thought_duration
            None, // turn_complete
        );

        // Should render the system message
        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect::<String>();
        assert!(row_text.contains("System message"), "Expected 'System message' in row, got: '{}'", row_text.trim());
    }

    #[test]
    fn test_assistant_with_thought_duration() {
        use ratatui::{buffer::Buffer, layout::Rect};
        use crate::components::message_list::render::{render_single_msg_feed, WrapCache};
        use crate::theme::ThemeWrapper;

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let item = FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: "Response".to_string(),
            thoughts: vec![Thought { duration: 1.5 }],
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
        };
        let _rendered = render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Gray,
            ratatui::style::Color::DarkGray,
            ratatui::style::Color::Black,
            ratatui::style::Color::Green,
            ratatui::style::Color::Red,
            ratatui::style::Color::Blue,
            '⠋', // spinner
            false, // cursor_visible
            false, // show_spinner
            '⠏', // rewind_spinner
            &AnimationState::default(),
            &mut wrap_cache,
            false, // agent_running
            None, // thought_duration - should be overridden by inline thoughts
            None, // turn_complete
        );

        // Should render thought duration indicator
        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect::<String>();
        assert!(row_text.contains("Thought"), "Expected 'Thought' indicator in row, got: '{}'", row_text.trim());
    }
}
