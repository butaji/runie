use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;

fn lerp_color(c1: ratatui::style::Color, c2: ratatui::style::Color, t: f32) -> ratatui::style::Color {
    match (c1, c2) {
        (ratatui::style::Color::Rgb(r1, g1, b1), ratatui::style::Color::Rgb(r2, g2, b2)) => {
            ratatui::style::Color::Rgb(
                (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
                (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
                (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
            )
        }
        _ => c1,
    }
}

fn render_pill(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    start_color: ratatui::style::Color,
    end_color: ratatui::style::Color,
    text: &str,
    text_color: ratatui::style::Color,
) {
    let width = area.width as usize;
    if width == 0 {
        return;
    }

    // Generate gradient colors
    let mut colors = Vec::with_capacity(width);
    for i in 0..width {
        let t = if width > 1 { i as f32 / (width - 1) as f32 } else { 0.0 };
        colors.push(lerp_color(start_color, end_color, t));
    }

    // Left cap
    buf.get_mut(area.x, area.y).set_char('◗');
    buf.get_mut(area.x, area.y).set_style(ratatui::style::Style::default().fg(colors[0]).bg(ratatui::style::Color::Reset));

    // Text with gradient background
    let text_len = text.len().min(width - 2);
    let start_x = area.x + 1 + (width.saturating_sub(text_len + 2) as u16) / 2;

    for (i, ch) in text.chars().enumerate() {
        if i >= text_len {
            break;
        }
        let col = (start_x + i as u16 - area.x) as usize;
        if col < width {
            buf.get_mut(start_x + i as u16, area.y).set_char(ch);
            buf.get_mut(start_x + i as u16, area.y).set_style(
                ratatui::style::Style::default().fg(text_color).bg(colors[col])
            );
        }
    }

    // Fill gaps with gradient background
    for col in 1..width-1 {
        let x = area.x + col as u16;
        let cell = buf.cell((x, area.y));
        // Only fill if cell is empty (no text char set)
        if cell.map(|c| c.symbol() == " ").unwrap_or(true) {
            buf.get_mut(x, area.y).set_char(' ');
            buf.get_mut(x, area.y).set_style(ratatui::style::Style::default().bg(colors[col]));
        }
    }

    // Right cap
    buf.get_mut(area.x + area.width - 1, area.y).set_char('◖');
    buf.get_mut(area.x + area.width - 1, area.y).set_style(
        ratatui::style::Style::default().fg(colors[width - 1]).bg(ratatui::style::Color::Reset)
    );
}

fn render_bookmark(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    theme: &ThemeWrapper,
    text: &str,
) {
    let height = area.height as usize;
    let width = area.width as usize;

    if height == 0 || width == 0 {
        return;
    }

    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();

    // Generate vertical gradient colors (top to bottom)
    let mut colors = Vec::with_capacity(height);
    for i in 0..height {
        let t = if height > 1 { i as f32 / (height - 1) as f32 } else { 0.0 };
        colors.push(lerp_color(accent_primary, accent_secondary, t));
    }

    // Top border: ╭ followed by ─────
    buf.get_mut(area.x, area.y)
        .set_char('╭')
        .set_style(Style::default().fg(colors[0]).bg(colors[0]));
    for col in 1..width - 1 {
        buf.get_mut(area.x + col as u16, area.y)
            .set_char('─')
            .set_style(Style::default().fg(colors[0]).bg(colors[0]));
    }
    buf.get_mut(area.x + width as u16 - 1, area.y)
        .set_char('╮')
        .set_style(Style::default().fg(colors[0]).bg(colors[0]));

    // Middle rows with text (if height > 2)
    let text_len = text.len();
    let text_start_col = (width.saturating_sub(text_len + 2)) / 2;

    for row in 1..height - 1 {
        let color = colors[row.min(colors.len() - 1)];
        // Left border
        buf.get_mut(area.x, area.y + row as u16)
            .set_char('│')
            .set_style(Style::default().fg(color).bg(color));

        // Text area
        for col in 1..width - 1 {
            let local_col = col - text_start_col;
            if local_col >= 0 && (local_col as usize) < text_len {
                let ch = text.chars().nth(local_col as usize).unwrap_or(' ');
                buf.get_mut(area.x + col as u16, area.y + row as u16)
                    .set_char(ch)
                    .set_style(Style::default().fg(ratatui::style::Color::Black).bg(color));
            } else {
                buf.get_mut(area.x + col as u16, area.y + row as u16)
                    .set_char(' ')
                    .set_style(Style::default().bg(color));
            }
        }

        // Right border
        buf.get_mut(area.x + width as u16 - 1, area.y + row as u16)
            .set_char('│')
            .set_style(Style::default().fg(color).bg(color));
    }

    // Bottom border: ╰ followed by ─────
    let bottom_row = height - 1;
    let bottom_color = colors[colors.len().saturating_sub(1)];
    buf.get_mut(area.x, area.y + bottom_row as u16)
        .set_char('╰')
        .set_style(Style::default().fg(bottom_color).bg(bottom_color));
    for col in 1..width - 1 {
        buf.get_mut(area.x + col as u16, area.y + bottom_row as u16)
            .set_char('─')
            .set_style(Style::default().fg(bottom_color).bg(bottom_color));
    }
    buf.get_mut(area.x + width as u16 - 1, area.y + bottom_row as u16)
        .set_char('╯')
        .set_style(Style::default().fg(bottom_color).bg(bottom_color));
}

#[derive(Clone)]
pub struct MessageList {
    pub messages: Vec<MessageItem>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageItem {
    User { text: String, model: Option<String> },
    Assistant { text: String, model: Option<String> },
    Thought { duration_secs: f32 },
    ToolCall { name: String, args: String },
    ToolResult { name: String, result: String, is_error: bool },
    Edit { filename: String },
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
    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        // Fill background with bg.base
        let bg_base: ratatui::style::Color = theme.color("bg.base").into();
        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_style(Style::default().bg(bg_base));
            }
        }

        let mut y = 0i32;
        let max_y = area.height as i32;

        for msg in self.messages.iter().skip(self.scroll_offset) {
            if y >= max_y {
                break;
            }

            match msg {
                MessageItem::User { text, model: _ } => {
                    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(10));
                    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
                    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();

                    let msg_height = wrapped.len() as i32;
                    let msg_area_height = msg_height.max(1);

                    // Draw gray background panel for user message
                    let bg_area = Rect::new(area.x + 1, area.y + y as u16, area.width - 3, msg_area_height as u16);
                    for row in 0..msg_area_height {
                        for x in bg_area.x..bg_area.x + bg_area.width {
                            buf.get_mut(x, bg_area.y + row as u16).set_style(Style::default().bg(bg_panel));
                        }
                    }

                    // Draw left border indicator (│ in accent.primary)
                    for row in 0..msg_area_height {
                        buf.get_mut(area.x + 1, area.y + y as u16 + row as u16)
                            .set_char('│')
                            .set_style(Style::default().fg(accent_primary).bg(bg_panel));
                    }

                    // Draw message text left-aligned
                    for line_text in wrapped {
                        if y >= max_y { break; }
                        let line = Line::from(vec![Span::styled(line_text.as_str(), Style::default().fg(text_primary))]);
                        buf.set_line(area.x + 3, area.y + y as u16, &line, area.width - 10);
                        y += 1;
                    }

                    // Draw bookmark on right edge
                    let bookmark_text = "You";
                    let bookmark_width = 6u16; // enough for " You " with box chars
                    let bookmark_x = area.x + area.width - 1 - bookmark_width;
                    let bookmark_area = Rect::new(bookmark_x, area.y + y as u16 - msg_height as u16, bookmark_width, msg_area_height as u16);
                    render_bookmark(buf, bookmark_area, theme, bookmark_text);
                    y += 1; // spacing
                }
                MessageItem::Assistant { text, model } => {
                    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(10));
                    let assistant_fg: ratatui::style::Color = theme.color("text.primary").into();

                    let msg_height = wrapped.len() as i32;
                    let msg_area_height = msg_height.max(1);

                    // Draw message text left-aligned (no background for assistant)
                    for line_text in wrapped {
                        if y >= max_y { break; }
                        let line = Line::from(vec![Span::styled(line_text.as_str(), Style::default().fg(assistant_fg))]);
                        buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 10);
                        y += 1;
                    }

                    // Draw bookmark on right edge
                    let bookmark_text = model.as_deref().unwrap_or("Assistant");
                    let bookmark_width = (bookmark_text.len() as u16 + 4).max(6);
                    let bookmark_x = area.x + area.width - 1 - bookmark_width;
                    let bookmark_area = Rect::new(bookmark_x, area.y + y as u16 - msg_height as u16, bookmark_width, msg_area_height as u16);
                    render_bookmark(buf, bookmark_area, theme, bookmark_text);
                    y += 1; // spacing after assistant message
                }
                MessageItem::Thought { duration_secs } => {
                    // Italic, text.dim, shows duration
                    let thought_text = format!("thinking... {:.1}s", duration_secs);
                    let thought_fg: ratatui::style::Color = theme.color("text.dim").into();
                    let line = Line::from(vec![
                        Span::styled(&thought_text, Style::default().fg(thought_fg).add_modifier(Modifier::ITALIC)),
                    ]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 6);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::ToolCall { name, args } => {
                    // Collapsible header: ▼ Tool: name(args)
                    let tool_fg: ratatui::style::Color = theme.color("text.muted").into();
                    let header = format!("▼ Tool: {}({})", name, args);
                    let line = Line::from(vec![Span::styled(&header, Style::default().fg(tool_fg))]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 6);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::ToolResult { name, result, is_error } => {
                    // Icon + name + truncated result
                    let (icon, style) = if *is_error {
                        ("✗", Style::default().fg(theme.color("error").into()))
                    } else {
                        ("✓", Style::default().fg(theme.color("success").into()))
                    };
                    let result_fg: ratatui::style::Color = theme.color("text.muted").into();
                    let result_truncated = truncate(result, 60);
                    let line = Line::from(vec![
                        Span::styled(format!("{} {}: ", icon, name), style),
                        Span::styled(&result_truncated, Style::default().fg(result_fg)),
                    ]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 6);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::Edit { filename } => {
                    let edit_fg: ratatui::style::Color = theme.color("accent.secondary").into();
                    let line = Line::from(vec![
                        Span::styled("✎ ", Style::default().fg(edit_fg)),
                        Span::styled(filename.as_str(), Style::default().fg(edit_fg)),
                    ]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 6);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::System { text } => {
                    // Centered, warning color, subtle
                    let sys_fg: ratatui::style::Color = theme.color("warning").into();
                    let line = Line::from(vec![Span::styled(text.as_str(), Style::default().fg(sys_fg))]);
                    let text_width = text.len() as u16;
                    let start_x = area.x + (area.width.saturating_sub(text_width)) / 2;
                    buf.set_line(start_x, area.y + y as u16, &line, text_width);
                    y += 1;
                    y += 1; // spacing
                }
            }
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
        });
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
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

    lines
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
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
        });
        list.messages.push(MessageItem::Assistant {
            text: "Hi".to_string(),
            model: Some("gpt-4".to_string()),
        });

        list.update_last_assistant("Hi there");
        assert_eq!(
            list.messages.last(),
            Some(&MessageItem::Assistant {
                text: "Hi there".to_string(),
                model: Some("gpt-4".to_string())
            })
        );
    }

    #[test]
    fn test_add_or_update_assistant_updates_existing() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant {
            text: "Partial".to_string(),
            model: Some("gpt-4".to_string()),
        });

        list.add_or_update_assistant("Complete response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 1);
        assert_eq!(
            list.messages[0],
            MessageItem::Assistant {
                text: "Complete response".to_string(),
                model: Some("gpt-4".to_string())
            }
        );
    }

    #[test]
    fn test_add_or_update_assistant_adds_new() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
        });

        list.add_or_update_assistant("Response", Some("gpt-4".to_string()));
        assert_eq!(list.messages.len(), 2);
        assert_eq!(
            list.messages[1],
            MessageItem::Assistant {
                text: "Response".to_string(),
                model: Some("gpt-4".to_string())
            }
        );
    }

    #[test]
    fn test_has_assistant_in_progress_true() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::Assistant {
            text: "Thinking...".to_string(),
            model: None,
        });
        assert!(list.has_assistant_in_progress());
    }

    #[test]
    fn test_has_assistant_in_progress_false() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
        });
        assert!(!list.has_assistant_in_progress());
    }

    #[test]
    fn test_update_last_assistant_no_op_when_no_assistant() {
        let mut list = MessageList::default();
        list.messages.push(MessageItem::User {
            text: "Hello".to_string(),
            model: None,
        });
        list.update_last_assistant("This should not change anything");
        assert_eq!(
            list.messages[0],
            MessageItem::User {
                text: "Hello".to_string(),
                model: None
            }
        );
    }
}