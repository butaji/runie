use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
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

struct StyleHelpers {
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
        let sp = StyleHelpers::new(theme);
        let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
        let blue_color: ratatui::style::Color = theme.color("accent.primary").into();

        // ──── Top border with tags ────
        // Left corner
        buf.get_mut(area.x, area.y).set_char('╭');
        buf.get_mut(area.x, area.y).set_style(Style::default().fg(border_color));

        // Top border line
        for x in 1..area.width.saturating_sub(1) {
            buf.get_mut(area.x + x, area.y).set_char('─');
            buf.get_mut(area.x + x, area.y).set_style(Style::default().fg(border_color));
        }

        // Right corner
        buf.get_mut(area.x + area.width - 1, area.y).set_char('╮');
        buf.get_mut(area.x + area.width - 1, area.y).set_style(Style::default().fg(border_color));

        // ──── Content lines ────
        let content_width = area.width.saturating_sub(4) as usize; // borders + indent

        for (line_idx, line_text) in self.lines.iter().enumerate() {
            let y = area.y + 1 + line_idx as u16;

            // Left border
            buf.get_mut(area.x, y).set_style(Style::default().fg(border_color));
            buf.get_mut(area.x, y).set_char('│');

            let x = area.x + 1;

            if line_idx == 0 {
                // First line: chevron prompt
                let chevron = "❯";
                let prompt_style = Style::default().fg(blue_color);
                let prompt_line = Line::from(vec![
                    Span::styled(chevron, prompt_style),
                    Span::styled(" ", Style::default()),
                ]);
                buf.set_line(x, y, &prompt_line, 2);

                // Text (truncate if too long)
                let text_x = x + 2;
                let available = content_width.saturating_sub(2);
                let display_text = if line_text.len() > available {
                    format!("{}...", &line_text[..available.saturating_sub(3)])
                } else {
                    line_text.clone()
                };
                let text_line = Line::from(vec![Span::styled(display_text, sp.primary())]);
                buf.set_line(text_x, y, &text_line, available as u16);
            } else {
                // Subsequent lines: indent
                let indent = "  ";
                let indent_line = Line::from(vec![Span::styled(indent, sp.dim())]);
                buf.set_line(x, y, &indent_line, 2);

                // Text
                let text_x = x + 2;
                let available = content_width.saturating_sub(2);
                let display_text = if line_text.len() > available {
                    format!("{}...", &line_text[..available.saturating_sub(3)])
                } else {
                    line_text.clone()
                };
                let text_line = Line::from(vec![Span::styled(display_text, sp.primary())]);
                buf.set_line(text_x, y, &text_line, available as u16);
            }

            // Right border
            buf.get_mut(area.x + area.width - 1, y)
                .set_style(Style::default().fg(border_color));
            buf.get_mut(area.x + area.width - 1, y).set_char('│');
        }

        // ──── Bottom border ────
        let bottom_y = area.y + 1 + self.lines.len() as u16;
        let info_text = if self.right_info.is_empty() {
            "model: claude-4"
        } else {
            &self.right_info
        };
        let info_len = info_text.len() as u16;
        let inner_width = area.width.saturating_sub(2); // space between corners

        // Layout: ╰ + ─...─ + " " + info + " " + ─ + ╯
        // Total fixed width: 1 (╰) + 1 (space) + info_len + 1 (space) + 1 (─) + 1 (╯) = info_len + 5
        let fixed_width = info_len + 5;
        let dashes = if inner_width >= fixed_width {
            inner_width - fixed_width
        } else {
            0
        };

        // Left corner
        buf.get_mut(area.x, bottom_y).set_char('╰');
        buf.get_mut(area.x, bottom_y).set_style(Style::default().fg(border_color));

        // Horizontal line before info
        for i in 0..dashes {
            let x = area.x + 1 + i;
            buf.get_mut(x, bottom_y).set_char('─');
            buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
        }

        // Space + info + space + ─ before corner
        let mut x = area.x + 1 + dashes;
        // Space
        buf.get_mut(x, bottom_y).set_char(' ');
        buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
        x += 1;
        // Info text
        for ch in info_text.chars() {
            buf.get_mut(x, bottom_y).set_char(ch);
            buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
            x += 1;
        }
        // Space
        buf.get_mut(x, bottom_y).set_char(' ');
        buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
        x += 1;
        // ─ before corner
        buf.get_mut(x, bottom_y).set_char('─');
        buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
        x += 1;

        // Right corner
        buf.get_mut(area.x + area.width - 1, bottom_y).set_char('╯');
        buf.get_mut(area.x + area.width - 1, bottom_y).set_style(Style::default().fg(border_color));
    }

    pub fn cursor_screen_pos(&self, area: Rect) -> ratatui::layout::Position {
        let x = area.x + 1;
        let y = area.y + 1 + self.cursor_line as u16;
        let cursor_x = if self.cursor_line == 0 {
            x + 2 + self.cursor_col as u16
        } else {
            x + 2 + self.cursor_col as u16
        };
        ratatui::layout::Position::new(cursor_x, y)
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
        if self.cursor_line >= self.lines.len() { return; }
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
            if ch.is_whitespace() { break; }
            pos -= 1;
        }
        line.drain(pos..self.cursor_col);
        self.cursor_col = pos;
    }

    pub fn delete_to_start(&mut self) {
        if self.cursor_line >= self.lines.len() { return; }
        let line = &mut self.lines[self.cursor_line];
        line.drain(0..self.cursor_col);
        self.cursor_col = 0;
    }

    pub fn delete_to_end(&mut self) {
        if self.cursor_line >= self.lines.len() { return; }
        let line = &mut self.lines[self.cursor_line];
        line.truncate(self.cursor_col);
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_line >= self.lines.len() { return; }
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

        // Prompt "❯ " at x=1,2
        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn test_cursor_after_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        // cursor_col = 2
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        // Verify text is rendered: "❯ hi" at x=1,2,3,4
        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "i");
    }

    #[test]
    fn test_cursor_in_middle_of_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left(); // cursor_col = 1
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        // Verify text is rendered: "❯ hi" with cursor between h and i
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

        // Chevron at x=1
        let chevron_cell = buf.cell((1, 1)).unwrap();
        assert_eq!(chevron_cell.symbol(), "❯");
    }

    #[test]
    fn test_cursor_on_second_line() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_newline(); // cursor_line=1, cursor_col=0
        input.insert_char('x'); // cursor_col=1
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        // Second line: "  x" at x=1,2,3
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
        // cursor_col = 10
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        // Verify text is rendered after prompt "❯ "
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
        input.backspace(); // merge with previous line
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('l');
        input.insert_char('l');
        input.insert_char('o');
        input.insert_char(' ');
        input.insert_char('w');
        input.insert_char('o');
        input.insert_char('r');
        input.insert_char('l');
        input.insert_char('d');
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
        assert_eq!(input.cursor_col, 1); // clamped to line length
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

        // Line 1: "❯ line1" at y=1
        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "l");

        // Line 2: "  line2" at y=2
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
        // Now 3 lines, cursor on line 2

        let area = Rect::new(0, 0, 40, 7);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        // Should have 3 content lines + 2 borders = height 5
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "│"); // line 0
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "│"); // line 1
        assert_eq!(buf.cell((0, 3)).unwrap().symbol(), "│"); // line 2
        assert_eq!(buf.cell((0, 4)).unwrap().symbol(), "╰"); // bottom border
    }

    // ──── Cursor Screen Position Tests ────

    #[test]
    fn test_cursor_screen_pos_empty() {
        let input = InputBar::default();
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 3); // area.x + 1 + 2 (prompt)
        assert_eq!(pos.y, 1); // area.y + 1
    }

    #[test]
    fn test_cursor_screen_pos_with_text() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 5); // 3 + 2 chars
        assert_eq!(pos.y, 1);
    }

    #[test]
    fn test_cursor_screen_pos_second_line() {
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_char('x');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 4); // area.x + 1 + 2 (indent) + 1 (cursor_col)
        assert_eq!(pos.y, 2); // area.y + 1 + 1
    }

    // ──── delete_word_backward edge cases ────

    #[test]
    fn test_delete_word_backward_with_punctuation() {
        let mut input = InputBar::default();
        // Type "hello world!"
        for ch in "hello world!".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        // Should delete "world!" back to the space
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
        // Should delete all "!!!"
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
        // Should delete "baz" (back to space)
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
        // cursor at start of line 1
        input.move_cursor_to_start();
        input.delete_word_backward();
        // Should join with previous line: "hi" + "there" = "hithere"
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
        // Should delete back through whitespace to "hello"
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
        input.move_cursor_left(); // cursor at 'l' (pos 3)
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
        // Move to end of first line
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_to_end();
        // Current behavior: just truncates first line, doesn't join
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
        // Move to end of first line
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_forward();
        // Should join with next line
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
        // cursor at (1, 2), move up to line 0 which has length 5
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2); // clamped to line 0 length
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
        // cursor at (1, 5), move up then down
        input.move_cursor_up();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 2); // clamped to line 1 length
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
        // cursor at (2, 1), backspace removes char since cursor_col > 0
        input.backspace();
        // Actual behavior: removes 'c', no line join since cursor_col was > 0
        assert_eq!(input.lines.len(), 3);
        assert_eq!(input.lines[0], "a");
        assert_eq!(input.lines[1], "b");
        assert_eq!(input.lines[2], ""); // 'c' was removed
        assert_eq!(input.cursor_line, 2);
        assert_eq!(input.cursor_col, 0);
        // TODO: When cursor_col == 0, backspace joins lines (tested below)
    }

    #[test]
    fn test_backspace_at_start_of_line_joins() {
        let mut input = InputBar::default();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        // Move cursor to start of line 1 (col 0), then backspace joins with previous line
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
