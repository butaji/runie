use std::collections::HashMap;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
};
use runie_core::session::Session;
use crate::theme::ThemeWrapper;
use crate::components::panel::render_gradient_border;

#[derive(Debug, Clone)]
pub struct SessionTreeEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub preview: String,
    pub timestamp: String,
    pub depth: usize,
}

#[derive(Clone)]
pub struct SessionTreeNavigator {
    pub visible: bool,
    pub entries: Vec<SessionTreeEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl Default for SessionTreeNavigator {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTreeNavigator {
    pub fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
        }
    }

    pub fn load_session(&mut self, session: &Session) {
        self.entries = session.messages.iter().map(|node| {
            let preview = match &node.message {
                runie_core::Message::User { content, .. } => {
                    let cut = content.len().min(40);
                    format!("User: {}", &content[..cut])
                }
                runie_core::Message::Assistant { content, .. } => {
                    let cut = content.len().min(40);
                    format!("Assistant: {}", &content[..cut])
                }
                _ => "System".to_string(),
            };
            SessionTreeEntry {
                id: node.id.clone(),
                parent_id: node.parent_id.clone(),
                preview,
                timestamp: node.timestamp.format("%H:%M").to_string(),
                depth: 0,
            }
        }).collect();
        self.compute_depths();
    }

    fn compute_depths(&mut self) {
        // Build id -> depth map for O(1) parent lookups
        let mut id_to_depth: HashMap<String, usize> = HashMap::new();
        
        for entry in self.entries.iter() {
            let depth = if let Some(ref parent_id) = entry.parent_id {
                *id_to_depth.get(parent_id).unwrap_or(&0) + 1
            } else {
                0
            };
            id_to_depth.insert(entry.id.clone(), depth);
        }
        
        for entry in self.entries.iter_mut() {
            if let Some(depth) = id_to_depth.get(&entry.id) {
                entry.depth = *depth;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll_offset = self.scroll_offset.min(self.selected);
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < self.entries.len().saturating_sub(1) {
            self.selected += 1;
            if self.selected >= self.scroll_offset + 20 {
                self.scroll_offset = self.selected - 19;
            }
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected = 0;
            self.scroll_offset = 0;
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn get_selected_id(&self) -> Option<String> {
        self.entries.get(self.selected).map(|e| e.id.clone())
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        if !self.visible {
            return;
        }

        let border: Color = theme.color("border.unfocused").into();
        let accent: Color = theme.color("syntax.phase").into();
        render_gradient_border(area, buf, border, accent);
        self.render_title(area, buf, theme);

        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        self.render_content(inner, buf, theme);
    }

    fn render_title(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let title = " Session Tree ";
        let title_len = title.len() as u16;
        let title_x = area.x + (area.width.saturating_sub(title_len)) / 2;
        let title_y = area.y;
        let title_color: Color = theme.color("syntax.phase").into();

        for (i, ch) in title.chars().enumerate() {
            if let Some(cell) = buf.cell_mut((title_x + i as u16, title_y)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(title_color));
            }
        }
    }

    fn render_content(&self, inner: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let text_secondary: Color = theme.color("text.secondary").into();
        let accent: Color = theme.color("accent.primary").into();
        let bg_panel: Color = theme.color("bg.panel").into();

        let max_rows = inner.height as usize;
        for (i, entry) in self.entries.iter()
            .skip(self.scroll_offset)
            .take(max_rows)
            .enumerate()
        {
            let row = i + self.scroll_offset;
            let y = inner.y + i as u16;

            let indent = "  ".repeat(entry.depth.min(5));
            let marker = if row == self.selected { "▸ " } else { "  " };
            let text = format!("{}{}{} {}", marker, indent, entry.timestamp, entry.preview);
            let color = if row == self.selected { accent } else { text_secondary };

            for x in 0..inner.width {
                if let Some(cell) = buf.cell_mut((inner.x + x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_panel).fg(color));
                }
            }

            let line = Line::raw(text).style(Style::default().fg(color));
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::Message;

    fn make_test_session() -> Session {
        let mut session = Session::new("test".to_string());
        let id1 = session.add_message(None, Message::User { content: "Hello".to_string(), attachments: vec![] });
        let _id2 = session.add_message(Some(id1.clone()), Message::Assistant { content: "Hi there".to_string(), tool_calls: vec![], thinking: None });
        session
    }

    #[test]
    fn test_load_session() {
        let mut nav = SessionTreeNavigator::new();
        let session = make_test_session();
        nav.load_session(&session);
        assert_eq!(nav.entries.len(), 2);
        assert_eq!(nav.entries[0].preview, "User: Hello");
        assert_eq!(nav.entries[1].preview, "Assistant: Hi there");
    }

    #[test]
    fn test_depth_computation() {
        let mut nav = SessionTreeNavigator::new();
        let session = make_test_session();
        nav.load_session(&session);
        assert_eq!(nav.entries[0].depth, 0);
        assert_eq!(nav.entries[1].depth, 1);
    }

    #[test]
    fn test_move_selection() {
        let mut nav = SessionTreeNavigator::new();
        let session = make_test_session();
        nav.load_session(&session);
        assert_eq!(nav.selected, 0);
        nav.move_down();
        assert_eq!(nav.selected, 1);
        nav.move_up();
        assert_eq!(nav.selected, 0);
    }

    #[test]
    fn test_toggle() {
        let mut nav = SessionTreeNavigator::new();
        assert!(!nav.visible);
        nav.toggle();
        assert!(nav.visible);
        nav.toggle();
        assert!(!nav.visible);
    }

    #[test]
    fn test_get_selected_id() {
        let mut nav = SessionTreeNavigator::new();
        let session = make_test_session();
        nav.load_session(&session);
        assert_eq!(nav.get_selected_id(), Some(session.messages[0].id.clone()));
        nav.move_down();
        assert_eq!(nav.get_selected_id(), Some(session.messages[1].id.clone()));
    }
}