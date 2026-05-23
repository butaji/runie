pub mod cursor;
pub mod render;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct InputBar {
    pub prompt: String,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub mode: InputMode,
    pub right_info: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
}

impl Default for InputBar {
    fn default() -> Self {
        Self {
            prompt: "\u{276F} ".to_string(),
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            mode: InputMode::Normal,
            right_info: String::new(),
        }
    }
}

pub struct StyleHelpers {
    text_primary: Style,
    text_tertiary: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_primary: Style::default().fg(theme.color("text.primary").into()),
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
        }
    }
    fn primary(&self) -> Style {
        self.text_primary
    }
    fn dim(&self) -> Style {
        self.text_tertiary
    }
}

impl InputBar {
    /// Height in visual lines (each logical line = 1 visual line, no wrapping)
    pub fn visual_height(&self) -> usize {
        self.lines.len().max(1)
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        render::render_ref(self, area, buf, theme);
    }

    pub fn cursor_screen_pos(&self, area: Rect) -> ratatui::layout::Position {
        cursor::cursor_screen_pos(self, area)
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_line];
        let pos = self.cursor_col.min(line.len());
        line.insert(pos, ch);
        self.cursor_col += 1;
    }

    pub fn insert_newline(&mut self) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_line];
        let pos = self.cursor_col.min(line.len());
        let new_line = line[pos..].to_string();
        line.truncate(pos);
        self.lines.insert(self.cursor_line + 1, new_line);
        self.cursor_line += 1;
        self.cursor_col = 0;
    }

    pub fn backspace(&mut self) {
        if self.cursor_line >= self.lines.len() {
            self.cursor_line = self.lines.len().saturating_sub(1);
        }
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.cursor_col = 0;
            return;
        }
        if self.cursor_col > self.lines[self.cursor_line].len() {
            self.cursor_col = self.lines[self.cursor_line].len();
        }
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_line];
            line.remove(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            let prev_len = self.lines[self.cursor_line - 1].len();
            let current = self.lines.remove(self.cursor_line);
            self.lines[self.cursor_line - 1].push_str(&current);
            self.cursor_line -= 1;
            self.cursor_col = prev_len;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_col < self.lines[self.cursor_line].len() {
            self.cursor_col += 1;
        } else if self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        if self.cursor_line < self.lines.len() {
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col == 0 {
            if self.cursor_line > 0 {
                let prev_len = self.lines[self.cursor_line - 1].len();
                let current = self.lines.remove(self.cursor_line);
                self.lines[self.cursor_line - 1].push_str(&current);
                self.cursor_line -= 1;
                self.cursor_col = prev_len;
            }
            return;
        }
        let text = line.as_str();
        let mut pos = self.cursor_col;
        while pos > 0 && text.chars().nth(pos - 1).unwrap_or(' ').is_whitespace() {
            pos -= 1;
        }
        while pos > 0 {
            let ch = text.chars().nth(pos - 1).unwrap_or(' ');
            if ch.is_whitespace() {
                break;
            }
            pos -= 1;
        }
        line.drain(pos..self.cursor_col);
        self.cursor_col = pos;
    }

    pub fn delete_to_start(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[self.cursor_line];
        line.drain(0..self.cursor_col);
        self.cursor_col = 0;
    }

    pub fn delete_to_end(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[self.cursor_line];
        line.truncate(self.cursor_col);
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col < line.len() {
            line.remove(self.cursor_col);
        } else if self.cursor_line < self.lines.len() - 1 {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
        }
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
    }

    pub fn submit(&mut self) -> String {
        let text = self.lines.join("\n");
        self.clear();
        text
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests;
#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests_extra;
