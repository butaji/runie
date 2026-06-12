use ratatui::{buffer::Buffer, layout::Rect};
use crate::glyphs;
use crate::components::message_list::{FeedItem, MessageList, MessageItem, Thought, WrapCache};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_last_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
        list.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false, thought_duration: None, turn_duration: None });
        list.update_last_assistant("Hi there");
        assert_eq!(list.messages.last(), Some(&MessageItem::Assistant { text: "Hi there".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false, thought_duration: None, turn_duration: None }));
    }

    #[test]
    fn test_update_last_assistant_updates_existing() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant { text: "Partial".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false, thought_duration: None, turn_duration: None });
        list.update_last_assistant("Complete response");
        assert_eq!(list.messages.len(), 1);
        assert_eq!(list.messages[0], MessageItem::Assistant { text: "Complete response".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false, thought_duration: None, turn_duration: None });
    }

    #[test]
    fn test_has_assistant_in_progress() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant { text: "Thinking...".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });
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

    fn render_assistant_msg(text: &str, agent_running: bool) -> (String, Buffer, Rect) {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();
        let item = make_assistant_item(text);

        let _rendered = crate::components::message_list::render::render_single_msg_feed(
            &item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Cyan,
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
            true,
            false,
            None,
        );

        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect();
        (row_text, buf, area)
    }

    fn make_assistant_item(text: &str) -> FeedItem {
        FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: text.to_string(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
            thoughts_collapsed: false,
            expanded: true,
            streaming_thinking_elapsed_ms: None,
            streaming_total_elapsed_ms: None,
            streaming_download_bytes: None,
        }
    }

    fn make_assistant_feed_item(text: &str, thoughts: Vec<Thought>) -> FeedItem {
        FeedItem::AssistantMessage {
            id: "test".to_string(),
            text: text.to_string(),
            thoughts,
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
            thoughts_collapsed: false,
            expanded: true,
            streaming_thinking_elapsed_ms: None,
            streaming_total_elapsed_ms: None,
            streaming_download_bytes: None,
        }
    }

    fn make_user_feed_item(text: &str) -> FeedItem {
        FeedItem::UserMessage {
            id: "test".to_string(),
            text: text.to_string(),
            timestamp: None,
        }
    }

    fn make_system_feed_item(text: &str) -> FeedItem {
        FeedItem::SystemNotice { text: text.to_string() }
    }

    fn render_feed_item(item: &FeedItem) -> (String, Buffer, Rect) {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let _rendered = crate::components::message_list::render::render_single_msg_feed(
            item, area, 0, area.x + 1, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Cyan,
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
            true,
            false,
            None,
        );

        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect();
        (row_text, buf, area)
    }

    #[test]
    fn test_assistant_empty_agent_running_shows_thinking() {
        let (row_text, _, _) = render_assistant_msg("", true);
        assert!(row_text.contains("Thinking"), "Expected 'Thinking' indicator in row, got: '{}'", row_text.trim());
    }

    #[test]
    fn test_assistant_empty_no_agent_running_shows_dot() {
        let (_, buf, area) = render_assistant_msg("", false);
        let cell = buf.cell((area.x + 2, area.y)).unwrap();
        assert_eq!(cell.symbol(), glyphs::DOT.to_string(), "Expected dot when agent not running");
    }

    #[test]
    fn test_assistant_non_empty_shows_text() {
        let (row_text, _, _) = render_assistant_msg("Hello world", true);
        assert!(row_text.contains("Hello world"), "Expected 'Hello world' in row, got: '{}'", row_text.trim());
    }

    #[test]
    fn test_user_message_renders() {
        let (_row_text, buf, area) = render_feed_item(&make_user_feed_item("Hello"));
        // Check that the buffer has content (message rendered)
        let non_empty = buf.content().iter().any(|c| c.symbol() != " ");
        assert!(non_empty, "User message should render some content");
    }

    #[test]
    fn test_system_notice_renders() {
        let (row_text, _, _) = render_feed_item(&make_system_feed_item("System message"));
        assert!(row_text.contains("System message"), "Expected 'System message' in row");
    }

    #[test]
    fn test_assistant_with_thought_duration() {
        let item = make_assistant_feed_item("Response", vec![Thought { duration: 1.5 }]);
        let (row_text, _, _) = render_feed_item(&item);
        assert!(row_text.contains("Thought"), "Expected 'Thought' indicator in row");
    }

    #[test]
    fn test_old_assistant_placeholder_shows_status() {
        let item = FeedItem::AssistantMessage {
            id: "old".to_string(),
            text: String::new(),
            thoughts: Vec::new(),
            tool_calls: Vec::new(),
            timestamp: None,
            turn_duration: None,
            thoughts_collapsed: false,
            expanded: true,
            streaming_thinking_elapsed_ms: None,
            streaming_total_elapsed_ms: None,
            streaming_download_bytes: None,
        };
        let (row_text, _, _) = render_feed_item_not_last(&item);
        // Empty assistant shows placeholder (Thinking or Waiting depending on state)
        let has_content = row_text.trim().len() > 0;
        assert!(has_content, "Placeholder should render something, got: '{}'", row_text.trim());
    }

    fn render_feed_item_not_last(item: &FeedItem) -> (String, Buffer, Rect) {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let theme = ThemeWrapper::default_for_test();
        let mut wrap_cache = WrapCache::new();

        let _rendered = crate::components::message_list::render::render_single_msg_feed(
            item, area, 0, area.x + 2, area.x + 4, area.height, &mut buf,
            &theme,
            ratatui::style::Color::White,
            ratatui::style::Color::Cyan,
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
            true,
            false,
            None,
        );

        let row_text: String = (0..area.width)
            .filter_map(|x| buf.cell((x, area.y)).map(|c| c.symbol().to_string()))
            .collect();
        (row_text, buf, area)
    }
}
