use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
};
use crate::theme::ThemeWrapper;

/// Wrap text into lines respecting word boundaries
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

#[derive(Clone)]
pub struct MessageList {
    pub messages: Vec<MessageItem>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageItem {
    User { text: String, model: Option<String>, timestamp: Option<String> },
    Assistant { text: String, model: Option<String>, timestamp: Option<String> },
    Thought { duration_secs: f32 },
    ToolCall { name: String, args: String, result: Option<String>, is_error: bool },
    Edit { filename: String, diff: Option<String> },
    System { text: String },
}

impl Default for MessageList {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
        }
    }
}

impl MessageList {
    pub fn render_ref(messages: &[MessageItem], scroll_offset: usize, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        fill_background(area, buf, theme);

        let messages_iter: Vec<&MessageItem> = messages.iter().skip(scroll_offset).collect();
        let mut row = 0u16;
        let max_rows = area.height;

        let margin_x = area.x + 2;
        let text_x = area.x + 4;

        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let success: ratatui::style::Color = theme.color("success").into();
        let error: ratatui::style::Color = theme.color("error").into();
        let code_path: ratatui::style::Color = theme.color("code.path").into();

        let mut prev_msg_type: Option<&str> = None;

        for msg in messages_iter {
            if row >= max_rows {
                break;
            }

            let msg_type = get_msg_type(msg);

            if prev_msg_type.is_some() && prev_msg_type != Some(msg_type) {
                if row < max_rows {
                    row += 1;
                }
            }
            prev_msg_type = Some(msg_type);

            let rendered = render_single_msg(msg, area, row, margin_x, text_x, max_rows, buf, theme, accent_primary, text_secondary, text_muted, success, error, code_path);
            row += rendered;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
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

fn fill_background(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_base: ratatui::style::Color = theme.color("bg.base").into();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            buf.get_mut(x, y).set_style(Style::default().bg(bg_base));
        }
    }
}

fn get_msg_type(msg: &MessageItem) -> &'static str {
    match msg {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Thought { .. } => "thought",
        MessageItem::ToolCall { .. } => "tool",
        MessageItem::Edit { .. } => "edit",
        MessageItem::System { .. } => "system",
    }
}

fn render_single_msg(
    msg: &MessageItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
) -> u16 {
    match msg {
        MessageItem::User { text, .. } => {
            let text_primary: ratatui::style::Color = theme.color("text.primary").into();
            let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();

            let wrapped = wrap_text(text, (area.width as usize).saturating_sub(8));
            let msg_height = wrapped.len() as u16;
            let total_height = msg_height + 2;

            let panel_start_y = area.y + row;
            let panel_start_x = margin_x - 1;
            let panel_width = area.width - 2;

            for r in 0..total_height {
                if panel_start_y + r >= area.y + area.height {
                    break;
                }
                for x in 0..panel_width {
                    if panel_start_x + x < area.x + area.width {
                        buf.get_mut(panel_start_x + x, panel_start_y + r)
                            .set_style(Style::default().bg(bg_panel));
                    }
                }
            }

            buf.get_mut(margin_x, area.y + row + 1)
                .set_char('❯')
                .set_style(Style::default().fg(accent_primary).bg(bg_panel));

            for (i, line_text) in wrapped.iter().enumerate() {
                if row + 1 + i as u16 >= max_rows {
                    break;
                }
                let line = Line::raw(line_text.as_str())
                    .style(Style::default().fg(text_primary).bg(bg_panel));
                buf.set_line(text_x, area.y + row + 1 + i as u16, &line, area.width - 6);
            }

            total_height
        }
        MessageItem::Assistant { text, .. } => {
            if text.is_empty() {
                let dot = Line::raw("·").style(Style::default().fg(text_muted));
                buf.set_line(margin_x, area.y + row, &dot, area.width - 2);
                1
            } else {
                let wrapped = wrap_text(text, (area.width as usize).saturating_sub(4));
                let msg_height = wrapped.len() as u16;

                for (i, line_text) in wrapped.iter().enumerate() {
                    if row + i as u16 >= max_rows {
                        break;
                    }
                    let line = Line::raw(line_text.as_str())
                        .style(Style::default().fg(text_secondary));
                    buf.set_line(margin_x, area.y + row + i as u16, &line, area.width - 2);
                }

                msg_height
            }
        }
        MessageItem::Thought { duration_secs } => {
            buf.get_mut(margin_x, area.y + row)
                .set_char('◆')
                .set_style(Style::default().fg(text_muted));

            let thought_text = format!("Thought for {:.1}s", duration_secs);
            let line = Line::raw(thought_text.as_str())
                .style(Style::default().fg(text_muted));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);

            1
        }
        MessageItem::ToolCall { name, args, result, is_error } => {
            buf.get_mut(margin_x, area.y + row)
                .set_char('◆')
                .set_style(Style::default().fg(text_muted));

            let header = format!("{}({})", name, args);
            let line = Line::raw(header.as_str())
                .style(Style::default().fg(text_secondary));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);

            let mut rendered = 1;

            if let Some(result_text) = result {
                if row + rendered >= max_rows {
                    return rendered;
                }

                for (i, ch) in "  ".chars().enumerate() {
                    buf.get_mut(text_x - 2 + i as u16, area.y + row + rendered).set_char(ch);
                    buf.get_mut(text_x - 2 + i as u16, area.y + row + rendered)
                        .set_style(Style::default().fg(text_muted));
                }

                buf.get_mut(text_x, area.y + row + rendered)
                    .set_char('→')
                    .set_style(Style::default().fg(text_muted));
                buf.get_mut(text_x + 1, area.y + row + rendered)
                    .set_char(if *is_error { '×' } else { '✓' })
                    .set_style(Style::default().fg(if *is_error { error } else { success }));

                let result_line = Line::raw(result_text.as_str())
                    .style(Style::default().fg(text_muted));
                buf.set_line(text_x + 3, area.y + row + rendered, &result_line, area.width - 7);

                rendered += 1;
            }

            rendered
        }
        MessageItem::Edit { filename, diff: _ } => {
            buf.get_mut(margin_x, area.y + row)
                .set_char('◆')
                .set_style(Style::default().fg(text_muted));

            let edit_label = "Edit ";
            let filename_only = std::path::Path::new(filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(filename);

            let edit_len = edit_label.len() as u16;
            for (i, ch) in edit_label.chars().enumerate() {
                buf.get_mut(text_x + i as u16, area.y + row)
                    .set_char(ch)
                    .set_style(Style::default().fg(text_secondary));
            }

            for (i, ch) in filename_only.chars().enumerate() {
                let x_pos = text_x + edit_len + i as u16;
                if x_pos < area.x + area.width {
                    buf.get_mut(x_pos, area.y + row)
                        .set_char(ch)
                        .set_style(Style::default().fg(code_path));
                }
            }

            1
        }
        MessageItem::System { text } => {
            buf.get_mut(margin_x, area.y + row)
                .set_char('◆')
                .set_style(Style::default().fg(text_muted));

            let line = Line::raw(text.as_str())
                .style(Style::default().fg(text_muted));
            buf.set_line(text_x, area.y + row, &line, area.width - 4);

            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_last_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });
        list.messages.push(MessageItem::Assistant {
            text: "Hi".to_string(),
            model: Some("gpt-4".to_string()),
            timestamp: None,
        });

        list.update_last_assistant("Hi there");
        assert_eq!(
            list.messages.last(),
            Some(&MessageItem::Assistant {
                text: "Hi there".to_string(),
                model: Some("gpt-4".to_string()),
                timestamp: None,
            })
        );
    }

    #[test]
    fn test_add_or_update_assistant_updates_existing() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant {
            text: "Partial".to_string(),
            model: Some("gpt-4".to_string()),
            timestamp: None,
        });

        list.add_or_update_assistant("Complete response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 1);
        assert_eq!(
            list.messages[0],
            MessageItem::Assistant {
                text: "Complete response".to_string(),
                model: Some("gpt-4".to_string()),
                timestamp: None,
            }
        );
    }

    #[test]
    fn test_add_or_update_assistant_adds_new() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });

        list.add_or_update_assistant("Response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 2);
        assert_eq!(
            list.messages[1],
            MessageItem::Assistant {
                text: "Response".to_string(),
                model: Some("gpt-4".to_string()),
                timestamp: None,
            }
        );
    }

    #[test]
    fn test_has_assistant_in_progress_true() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant {
            text: "Thinking...".to_string(),
            model: None,
            timestamp: None,
        });
        assert!(list.has_assistant_in_progress());
    }

    #[test]
    fn test_has_assistant_in_progress_false() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });
        assert!(!list.has_assistant_in_progress());
    }

    #[test]
    fn test_update_last_assistant_no_op_when_no_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        });
        list.update_last_assistant("This should not change anything");
        assert_eq!(
            list.messages[0],
            MessageItem::User {
                text: "Hello".to_string(),
                model: None,
                timestamp: None,
            }
        );
    }
}
