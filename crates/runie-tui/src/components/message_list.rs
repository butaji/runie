use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;
pub mod types;
pub mod render;

pub use types::{MessageItem, MessageList, PlanStatus, BRAILLE_FRAMES, REVERSE_BRAILLE_FRAMES};

/// ViewModel for rendering MessageList
pub struct MessageListViewModel {
    pub messages: Vec<MessageItem>,
    pub scroll_offset: usize,
    pub agent_running: bool,
    pub animation: AnimationState,
}

impl MessageListViewModel {
    pub fn new(messages: Vec<MessageItem>, scroll_offset: usize, agent_running: bool, animation: AnimationState) -> Self {
        Self {
            messages,
            scroll_offset,
            agent_running,
            animation,
        }
    }
}

impl MessageList {
    pub fn render_ref(
        vm: &MessageListViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
        wrap_cache: &mut WrapCache,
    ) {
        let mut row = 0u16;
        let max_rows = area.height;

        let margin_x = area.x + 2;
        let text_x = area.x + 4;

        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let text_dim: ratatui::style::Color = theme.color("text.dim").into();
        let success: ratatui::style::Color = theme.color("success").into();
        let error: ratatui::style::Color = theme.color("error").into();
        let code_path: ratatui::style::Color = theme.color("code.path").into();

        let spinner = BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let rewind_spinner = REVERSE_BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let total_messages = vm.messages.len();
        let mut prev_msg_type: Option<&str> = None;

        let most_recent_spinner = render::find_most_recent_spinner_index(&vm.messages);

        for (idx, msg) in vm.messages.iter().skip(vm.scroll_offset).enumerate() {
            if row >= max_rows { break; }

            let absolute_idx = vm.scroll_offset + idx;
            let msg_type = render::get_msg_type(msg);
            if prev_msg_type.is_some() && prev_msg_type != Some(msg_type) && row < max_rows {
                row += 1;
            }
            prev_msg_type = Some(msg_type);

            let show_cursor = render::should_show_cursor(&vm.animation, vm.agent_running, absolute_idx, total_messages, msg);
            let show_spinner = most_recent_spinner == Some(absolute_idx);
            let rendered = render::render_single_msg(
                msg, area, row, margin_x, text_x, max_rows, buf, theme,
                accent_primary, text_secondary, text_muted, text_dim,
                success, error, code_path, spinner, show_cursor, show_spinner, rewind_spinner,
                &vm.animation, wrap_cache,
            );
            row += rendered;
        }

        // Empty state: no messages and no active agent
        if vm.messages.is_empty() && !vm.agent_running {
            render::render_empty_state(area, buf, text_muted, text_dim, text_x);
        }
    }

    /// Update the last assistant message with new text (for streaming)
    pub fn update_last_assistant(&mut self, new_text: &str) {
        if let Some(last) = self.messages.last_mut() {
            if let MessageItem::Assistant { ref mut text, .. } = last {
                *text = new_text.to_string();
            }
        }
    }

    /// Check if the last message is an Assistant message
    pub fn has_assistant_in_progress(&self) -> bool {
        matches!(self.messages.last(), Some(MessageItem::Assistant { .. }))
    }

    /// Add or update assistant message. If last message is assistant, updates it.
    /// Otherwise, adds a new assistant message.
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
}
