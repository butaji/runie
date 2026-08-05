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
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget, Wrap};

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
        Self { buffer: String::new(), focused: true }
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
        // Match grok-build's minimal-mode prompt: bare `❯` glyph, no border,
        // bright accent when the buffer has content, dim otherwise.
        let (glyph, color) = if self.buffer.is_empty() {
            ("❯ ", Color::DarkGray)
        } else {
            ("❯ ", Color::Cyan)
        };
        let content = Line::from(vec![
            ratatui::text::Span::styled(glyph, Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD)),
            ratatui::text::Span::raw(&self.buffer),
        ]);
        let p = Paragraph::new(content).wrap(Wrap { trim: false });
        Widget::render(p, area, buf);
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
        KeyEvent { code, modifiers: mods, kind: KeyEventKind::Press, state: crossterm::event::KeyEventState::NONE }
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
}