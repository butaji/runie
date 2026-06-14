//! Cursor movement and vim navigation for the input field.

use crate::model::AppState;
use crate::Event;

use super::input_text_support::{
    find_word_boundary_left, find_word_boundary_right, next_grapheme_boundary,
    prev_grapheme_boundary,
};

impl AppState {
    pub(crate) fn cursor_left(&mut self) {
        if self.input.cursor_pos > 0 {
            self.input.cursor_pos =
                prev_grapheme_boundary(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.input.ghost_completion.is_some() {
            self.accept_ghost();
            return;
        }
        if self.input.cursor_pos < self.input.input.len() {
            self.input.cursor_pos =
                next_grapheme_boundary(&self.input.input, self.input.cursor_pos);
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input.input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.input.cursor_pos != 0 {
            self.input.cursor_pos = 0;
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.input.input.contains('\n') {
            self.move_cursor_to_line_end();
        } else if self.input.cursor_pos != self.input.input.len() {
            self.input.cursor_pos = self.input.input.len();
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_left(&mut self) {
        if self.input.cursor_pos > 0 {
            self.input.cursor_pos =
                find_word_boundary_left(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_right(&mut self) {
        if self.input.cursor_pos < self.input.input.len() {
            self.input.cursor_pos =
                find_word_boundary_right(&self.input.input, self.input.cursor_pos);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    /// Handle a character while in vim nav mode. Returns true if the char
    /// was consumed (motion or space-to-leave), false if it should be
    /// treated as a normal typed character (which also leaves nav mode).
    pub(crate) fn handle_vim_nav_char(&mut self, c: char) {
        if c == ' ' {
            self.vim_nav_mode = false;
            self.insert_char(' ');
            return;
        }
        if c == 'i' {
            self.vim_nav_mode = false;
            self.mark_dirty();
            return;
        }
        if let Some(handled) = self.try_vim_nav_motion(c) {
            if handled {
                return;
            }
        }
        if let Some(evt) = self.vim_motion_event(c) {
            self.update(evt);
            return;
        }
        self.vim_nav_mode = false;
        self.insert_char(c);
    }

    fn try_vim_nav_motion(&mut self, c: char) -> Option<bool> {
        let last = self.view.posts.len().saturating_sub(1);
        match c {
            'j' => {
                if self.view.selected_post.unwrap_or(0) >= last {
                    self.vim_nav_mode = false;
                    self.mark_dirty();
                    Some(true)
                } else {
                    super::scroll::element_jump_down(self);
                    Some(true)
                }
            }
            'k' => {
                if self.view.selected_post.unwrap_or(0) == 0 {
                    self.input.input_flash = 3;
                    self.mark_dirty();
                    Some(true)
                } else {
                    super::scroll::element_jump_up(self);
                    Some(true)
                }
            }
            'g' => {
                self.update(Event::GoToTop);
                Some(true)
            }
            'G' => {
                self.update(Event::GoToBottom);
                Some(true)
            }
            _ => None,
        }
    }

    /// Called by `update` for non-char events while in nav mode. Returns
    /// `Some(false)` to fully consume the event, `Some(true)` to fall
    /// through.
    pub(crate) fn handle_vim_nav_event(&mut self, event: &Event) -> Option<bool> {
        match event {
            Event::HistoryPrev | Event::ScrollUp => {
                if self.view.selected_post.unwrap_or(0) == 0 {
                    self.input.input_flash = 3;
                    self.mark_dirty();
                } else {
                    super::scroll::element_jump_up(self);
                }
                Some(false)
            }
            Event::HistoryNext | Event::ScrollDown => {
                let last = self.view.posts.len().saturating_sub(1);
                if self.view.selected_post.unwrap_or(0) >= last {
                    self.vim_nav_mode = false;
                    self.mark_dirty();
                } else {
                    super::scroll::element_jump_down(self);
                }
                Some(false)
            }
            Event::PageUp | Event::PageDown | Event::GoToTop | Event::GoToBottom => {
                super::scroll::scroll_event(self, event.clone());
                Some(false)
            }
            Event::ToggleCommandPalette => {
                super::dialog_toggle::dialog_toggle_event(self, event.clone());
                Some(false)
            }
            Event::CopyLastResponse => {
                super::control::control_event(self, event.clone());
                Some(false)
            }
            _ => Some(true),
        }
    }

    /// Map a single character to a vim motion event when vim_mode is on and
    /// the input field is empty. Returns None for characters that should be
    /// inserted normally.
    pub(crate) fn vim_motion_event(&self, c: char) -> Option<Event> {
        match c {
            'j' => Some(Event::ScrollUp),
            'k' => Some(Event::ScrollDown),
            'g' => Some(Event::GoToTop),
            'G' => Some(Event::GoToBottom),
            '/' => Some(Event::ToggleCommandPalette),
            _ => None,
        }
    }
}
