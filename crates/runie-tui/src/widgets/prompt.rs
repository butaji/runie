//! Prompt widget: simple text input with no external crate dependency.
//!
//! Avoids `tui-textarea` because its render path panics on certain
//! invariant states (empty buffers, no cursor-sync integration). The
//! trade-off: no advanced editing features (cursor movement, selection),
//! but the prompt is append-only + Enter-submits which is all this
//! minimal TUI needs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
#[cfg(test)]
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget, Wrap};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    /// Enter pressed; prompt was non-empty.
    Submitted(String),
    /// Key modified the buffer.
    Edited,
    /// Key had no effect.
    Ignored,
}

#[derive(Clone)]
pub struct PromptWidget {
    buffer: String,
    focused: bool,
}

impl PromptWidget {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            focused: true,
        }
    }

    pub fn text(&self) -> String {
        self.buffer.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn set_focused(&mut self, on: bool) {
        self.focused = on;
    }

    /// Return the terminal cursor position for the current prompt text.
    /// The prompt prefix occupies two terminal columns (`❯ `).
    pub fn cursor_position(&self, area: Rect) -> ratatui::layout::Position {
        let width = area.width.saturating_sub(5).max(1) as usize;
        let column = 3 + UnicodeWidthStr::width(self.buffer.as_str());
        ratatui::layout::Position::new(
            area.x + 1 + (column % width) as u16,
            area.y + 1 + (column / width).min(area.height.saturating_sub(3) as usize) as u16,
        )
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        match key.code {
            KeyCode::Enter if key.modifiers == KeyModifiers::NONE => {
                if self.buffer.trim().is_empty() {
                    PromptOutcome::Ignored
                } else {
                    let text = std::mem::take(&mut self.buffer);
                    PromptOutcome::Submitted(text)
                }
            }
            KeyCode::Backspace => {
                if !self.buffer.is_empty() {
                    self.buffer.pop();
                    PromptOutcome::Edited
                } else {
                    PromptOutcome::Ignored
                }
            }
            KeyCode::Char(c) => {
                self.buffer.push(c);
                PromptOutcome::Edited
            }
            _ => PromptOutcome::Ignored,
        }
    }
}

impl Widget for PromptWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 1 {
            return;
        }
        // Grok full mode uses a three-row composer: a top divider, one input
        // row, and a bottom divider with the model/mode caption.
        let border = Style::default();
        let top = area.y;
        let bottom = area.y + area.height.saturating_sub(1);
        let right = area.x + area.width.saturating_sub(1);
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, top)) {
                cell.set_char(if x == area.x {
                    '╭'
                } else if x == right {
                    '╮'
                } else {
                    '─'
                });
                cell.set_style(border);
            }
            if bottom != top {
                if let Some(cell) = buf.cell_mut((x, bottom)) {
                    cell.set_char(if x == area.x {
                        '╰'
                    } else if x == right {
                        '╯'
                    } else {
                        '─'
                    });
                    cell.set_style(border);
                }
            }
        }
        for y in top.saturating_add(1)..bottom {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_char('│');
                cell.set_style(border);
            }
            if let Some(cell) = buf.cell_mut((right, y)) {
                cell.set_char('│');
                cell.set_style(border);
            }
        }
        let caption = "Grok 4.5 (high)";
        let caption_width = UnicodeWidthStr::width(caption) as u16 + 2;
        if caption_width + 2 < area.width {
            let caption_x = right.saturating_sub(caption_width + 1);
            buf.set_string(caption_x, bottom, format!(" {caption} "), border);
        }
        let input_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2).max(1),
        };
        let glyph = " ❯ ";
        let content = Line::from(vec![
            ratatui::text::Span::styled(glyph, Style::default()),
            ratatui::text::Span::raw(&self.buffer),
        ]);
        let p = Paragraph::new(content).wrap(Wrap { trim: false });
        Widget::render(p, input_area, buf);
    }
}

impl Default for PromptWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn empty_enter_is_ignored() {
        let mut p = PromptWidget::new();
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(out, PromptOutcome::Ignored);
    }

    #[test]
    fn char_then_enter_submits() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('h'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Char('i'), KeyModifiers::NONE));
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        match out {
            PromptOutcome::Submitted(s) => assert_eq!(s, "hi"),
            other => panic!("expected Submitted, got {other:?}"),
        }
        assert!(p.is_empty());
    }

    #[test]
    fn backspace_pops_last_char() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(p.text(), "a");
    }

    #[test]
    fn empty_prompt_uses_bare_cursor_glyph() {
        let p = PromptWidget::new();
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 3,
        });
        p.clone().render(
            Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 3,
            },
            &mut buffer,
        );
        assert_eq!(buffer.cell((2, 1)).expect("cursor cell").symbol(), "❯");
        assert_eq!(buffer.cell((2, 1)).expect("cursor cell").fg, Color::Reset);
    }

    #[test]
    fn cursor_position_counts_unicode_display_width() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('界'), KeyModifiers::NONE));
        let pos = p.cursor_position(Rect {
            x: 4,
            y: 7,
            width: 20,
            height: 3,
        });
        assert_eq!(pos, ratatui::layout::Position::new(10, 8));
    }

    #[test]
    fn test_backend_receives_prompt_cursor_position() {
        use ratatui::backend::Backend;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut prompt = PromptWidget::new();
        prompt.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        let mut terminal = Terminal::new(TestBackend::new(20, 4)).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(prompt.clone(), area);
                frame.set_cursor_position(prompt.cursor_position(area));
            })
            .expect("draw prompt");
        assert_eq!(
            terminal
                .backend_mut()
                .get_cursor_position()
                .expect("cursor"),
            ratatui::layout::Position::new(5, 1)
        );
    }
}
