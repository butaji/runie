use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct MessageList {
    pub messages: Vec<MessageItem>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub enum MessageItem {
    User { text: String },
    Assistant { text: String },
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
                MessageItem::User { text } => {
                    // Right-aligned, accent.tertiary (coral), subtle bg.panel background
                    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(8));
                    let user_bg: ratatui::style::Color = theme.color("bg.panel").into();
                    let user_fg: ratatui::style::Color = theme.color("accent.tertiary").into();

                    for line_text in wrapped {
                        if y >= max_y { break; }
                        // Draw background for user message row
                        for x in area.x..area.x + area.width {
                            buf.get_mut(x, area.y + y as u16).set_style(Style::default().bg(user_bg));
                        }

                        let line = Line::from(vec![Span::styled(line_text.as_str(), Style::default().fg(user_fg))]);
                        // Right-align: calculate start x
                        let line_width = line_text.len() as u16;
                        let start_x = area.x + area.width.saturating_sub(line_width + 2);
                        buf.set_line(start_x, area.y + y as u16, &line, line_width);
                        y += 1;
                    }
                    y += 1; // spacing after user message
                }
                MessageItem::Assistant { text } => {
                    // Left-aligned, text.primary, full width
                    let wrapped = wrap_text(text, (area.width as usize).saturating_sub(4));
                    let assistant_fg: ratatui::style::Color = theme.color("text.primary").into();

                    for line_text in wrapped {
                        if y >= max_y { break; }
                        let line = Line::from(vec![Span::styled(line_text.as_str(), Style::default().fg(assistant_fg))]);
                        buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 4);
                        y += 1;
                    }
                    y += 1; // spacing after assistant message
                }
                MessageItem::Thought { duration_secs } => {
                    // Italic, text.dim, shows duration
                    let thought_text = format!("thinking... {:.1}s", duration_secs);
                    let thought_fg: ratatui::style::Color = theme.color("text.dim").into();
                    let line = Line::from(vec![
                        Span::styled(&thought_text, Style::default().fg(thought_fg).add_modifier(Modifier::ITALIC)),
                    ]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 4);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::ToolCall { name, args } => {
                    // Collapsible header: ▼ Tool: name(args)
                    let tool_fg: ratatui::style::Color = theme.color("text.muted").into();
                    let header = format!("▼ Tool: {}({})", name, args);
                    let line = Line::from(vec![Span::styled(&header, Style::default().fg(tool_fg))]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 4);
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
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 4);
                    y += 1;
                    y += 1; // spacing
                }
                MessageItem::Edit { filename } => {
                    let edit_fg: ratatui::style::Color = theme.color("accent.secondary").into();
                    let line = Line::from(vec![
                        Span::styled("✎ ", Style::default().fg(edit_fg)),
                        Span::styled(filename.as_str(), Style::default().fg(edit_fg)),
                    ]);
                    buf.set_line(area.x + 2, area.y + y as u16, &line, area.width - 4);
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