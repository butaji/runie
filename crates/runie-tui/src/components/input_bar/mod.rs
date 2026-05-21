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
    text_body: Style,
    accent_chevron: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_primary: Style::default().fg(theme.color("text.primary").into()),
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
            text_body: Style::default().fg(theme.color("text.secondary").into()),
            accent_chevron: Style::default().fg(theme.color("accent.secondary").into()),
        }
    }
    fn primary(&self) -> Style {
        self.text_primary
    }
    fn dim(&self) -> Style {
        self.text_tertiary
    }
    fn body(&self) -> Style {
        self.text_body
    }
    fn chevron(&self) -> Style {
        self.accent_chevron
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    fn setup_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    // ──── Cursor Rendering Tests ────

    #[test]
    fn test_cursor_empty_first_line() {
        let theme = setup_theme();
        let input = InputBar {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            ..Default::default()
        };
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn test_cursor_after_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "i");
    }

    #[test]
    fn test_cursor_in_middle_of_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "i");
    }

    #[test]
    fn test_cursor_color_matches_chevron() {
        let theme = setup_theme();
        let input = InputBar::default();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        let chevron_cell = buf.cell((1, 1)).unwrap();
        assert_eq!(chevron_cell.symbol(), "❯");
    }

    #[test]
    fn test_cursor_on_second_line() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_char('x');
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 2)).unwrap().symbol(), "x");
    }

    #[test]
    fn test_cursor_at_end_of_long_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        for _ in 0..10 {
            input.insert_char('a');
        }
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "a");
    }

    // ──── Input Editing Tests ────

    #[test]
    fn test_insert_char() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_insert_newline() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.lines[1], "");
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_in_middle() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.backspace();
        assert_eq!(input.lines[0], "i");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_at_line_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.backspace();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut input = InputBar::default();
        for ch in "hello world".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "hello ");
        assert_eq!(input.cursor_col, 6);
    }

    #[test]
    fn test_delete_to_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        input.delete_to_start();
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_end() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.delete_to_end();
        assert_eq!(input.lines[0], "h");
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_delete_forward() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.delete_forward();
        assert_eq!(input.lines[0], "h");
    }

    // ──── Cursor Navigation Tests ────

    #[test]
    fn test_move_cursor_left() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_left_across_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.move_cursor_left();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_right() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.move_cursor_to_start();
        input.move_cursor_right();
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_right_across_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.move_cursor_right();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_up() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.insert_char('i');
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_down() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.insert_char('i');
        input.move_cursor_up();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_to_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_to_end() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        input.move_cursor_to_end();
        assert_eq!(input.cursor_col, 2);
    }

    // ──── Submit/Clear Tests ────

    #[test]
    fn test_submit() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('b');
        let result = input.submit();
        assert_eq!(result, "hi\nb");
        assert_eq!(input.lines, vec![String::new()]);
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_clear() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.clear();
        assert_eq!(input.lines, vec![String::new()]);
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    // ──── Multi-line Tests ────

    #[test]
    fn test_multi_line_render() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('l');
        input.insert_char('i');
        input.insert_char('n');
        input.insert_char('e');
        input.insert_char('1');
        input.insert_newline();
        input.insert_char('l');
        input.insert_char('i');
        input.insert_char('n');
        input.insert_char('e');
        input.insert_char('2');

        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "l");

        assert_eq!(buf.cell((1, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 2)).unwrap().symbol(), "l");
    }

    #[test]
    fn test_three_lines() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_newline();

        let area = Rect::new(0, 0, 40, 7);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 3)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 4)).unwrap().symbol(), "╰");
    }

    // ──── Cursor Screen Position Tests ────

    #[test]
    fn test_cursor_screen_pos_empty() {
        let input = InputBar::default();
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 3);
        assert_eq!(pos.y, 1);
    }

    #[test]
    fn test_cursor_screen_pos_with_text() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 1);
    }

    #[test]
    fn test_cursor_screen_pos_second_line() {
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_char('x');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 4);
        assert_eq!(pos.y, 2);
    }

    // ──── delete_word_backward edge cases ────

    #[test]
    fn test_delete_word_backward_with_punctuation() {
        let mut input = InputBar::default();
        for ch in "hello world!".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "hello ");
        assert_eq!(input.cursor_col, 6);
    }

    #[test]
    fn test_delete_word_backward_punctuation_only() {
        let mut input = InputBar::default();
        for ch in "!!!".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_mixed_punctuation_words() {
        let mut input = InputBar::default();
        for ch in "foo-bar baz".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "foo-bar ");
        assert_eq!(input.cursor_col, 8);
    }

    #[test]
    fn test_delete_word_backward_at_start_of_line_joins() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_to_start();
        input.delete_word_backward();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hithere");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_word_backward_multiple_whitespace() {
        let mut input = InputBar::default();
        for ch in "hello   ".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_single_word() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_empty() {
        let mut input = InputBar::default();
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── delete_to_start edge cases ────

    #[test]
    fn test_delete_to_start_in_middle() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_left();
        input.move_cursor_left();
        input.delete_to_start();
        assert_eq!(input.lines[0], "lo");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_start_at_end() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_to_start();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── delete_to_end edge cases ────

    #[test]
    fn test_delete_to_end_at_start() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_to_start();
        input.delete_to_end();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_end_at_end() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_to_end();
        assert_eq!(input.lines[0], "hello");
        assert_eq!(input.cursor_col, 5);
    }

    #[test]
    fn test_delete_to_end_multi_line_no_join() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_to_end();
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.lines[1], "there");
    }

    // ──── delete_forward edge cases ────

    #[test]
    fn test_delete_forward_at_end_joins_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_forward();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hithere");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_forward_at_end_of_last_line() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_forward();
        assert_eq!(input.lines[0], "hello");
        assert_eq!(input.cursor_col, 5);
    }

    #[test]
    fn test_delete_forward_in_middle() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_to_start();
        input.move_cursor_right();
        input.delete_forward();
        assert_eq!(input.lines[0], "hllo");
        assert_eq!(input.cursor_col, 1);
    }

    // ──── Cursor boundary tests ────

    #[test]
    fn test_move_cursor_left_at_start() {
        let mut input = InputBar::default();
        input.move_cursor_left();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_right_at_end() {
        let mut input = InputBar::default();
        for ch in "hi".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_right();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_move_cursor_up_at_top() {
        let mut input = InputBar::default();
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_down_at_bottom() {
        let mut input = InputBar::default();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_up_clamping() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.insert_newline();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_move_cursor_down_clamping() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_up();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 2);
    }

    // ──── Multi-line backspace tests ────

    #[test]
    fn test_backspace_joins_multiple_lines() {
        let mut input = InputBar::default();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        input.insert_newline();
        input.insert_char('c');
        input.backspace();
        assert_eq!(input.lines.len(), 3);
        assert_eq!(input.lines[0], "a");
        assert_eq!(input.lines[1], "b");
        assert_eq!(input.lines[2], "");
        assert_eq!(input.cursor_line, 2);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_at_start_of_line_joins() {
        let mut input = InputBar::default();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        input.move_cursor_to_start();
        input.backspace();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "ab");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_backspace_at_start_of_first_line() {
        let mut input = InputBar::default();
        input.backspace();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── Ctrl+A/E on multi-line ────

    #[test]
    fn test_move_cursor_to_start_multi_line() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_to_start();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_to_end_multi_line() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.move_cursor_to_start();
        input.move_cursor_to_end();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 1);
    }
}
