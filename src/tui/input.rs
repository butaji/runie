use ratatui::{
    widgets::Paragraph,
    style::{Style, Color},
    text::Line,
};

pub struct Input {
    text: String,
    cursor_position: usize,
}

impl Input {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor_position: 0,
        }
    }

    pub fn render(&self) -> Paragraph<'_> {
        let prompt = "> ";
        let display = format!("{}{}", prompt, self.text);
        
        Paragraph::new(Line::from(display))
            .style(Style::new().fg(Color::Green))
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_position = text.len();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Char(c) => {
                self.text.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            crossterm::event::KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.text.remove(self.cursor_position);
                }
            }
            crossterm::event::KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            crossterm::event::KeyCode::Right => {
                if self.cursor_position < self.text.len() {
                    self.cursor_position += 1;
                }
            }
            crossterm::event::KeyCode::Enter => {
                // Handled in app.rs via execute_input();
                // just clear locally so stale text doesn't persist
                self.text.clear();
                self.cursor_position = 0;
            }
            _ => {}
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
