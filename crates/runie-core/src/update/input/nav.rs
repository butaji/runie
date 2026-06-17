//! Cursor & vim navigation (merged from input_nav.rs).

use crate::event::{DialogEvent, InputEvent, ScrollEvent};
use crate::model::AppState;
use crate::Event;

pub const PAGE_SIZE: usize = 5;

fn count_input_lines(input: &str) -> usize {
    if input.is_empty() {
        return 1;
    }
    let mut lines = input.lines().count().max(1);
    if input.ends_with('\n') {
        lines += 1;
    }
    lines
}

impl AppState {
    pub(crate) fn get_current_line_bounds(&self) -> (usize, usize) {
        if self.input.input.is_empty() {
            return (0, 0);
        }
        let mut line_start = 0;
        let mut line_end = self.input.input.len();

        for (i, c) in self.input.input.char_indices() {
            if c == '\n' {
                if i >= self.input.cursor_pos {
                    line_end = i;
                    break;
                }
                line_start = i + 1;
            }
        }
        if line_end == self.input.input.len()
            && self.input.input.ends_with('\n')
            && self.input.cursor_pos > line_start
        {
            line_end = self.input.input.len();
        }
        (line_start, line_end)
    }

    pub(crate) fn move_cursor_to_line_start(&mut self) {
        let (line_start, _) = self.get_current_line_bounds();
        self.input.cursor_pos = line_start;
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn move_cursor_to_line_end(&mut self) {
        let (_, line_end) = self.get_current_line_bounds();
        self.input.cursor_pos = line_end;
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn move_cursor_up(&mut self) {
        if !self.input.input.contains('\n') {
            self.history_prev();
            return;
        }
        let (line_start, _) = self.get_current_line_bounds();
        if line_start == 0 {
            self.input.input_flash = 3;
            return;
        }
        let prev_line_start = self.input.input[..line_start - 1]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let current_col = self.input.cursor_pos - line_start;
        let prev_line_end = line_start - 1;
        let prev_line_len = prev_line_end - prev_line_start;
        self.input.cursor_pos = prev_line_start + current_col.min(prev_line_len);
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn move_cursor_down(&mut self) {
        if !self.input.input.contains('\n') {
            self.history_next();
            return;
        }
        let (_, line_end) = self.get_current_line_bounds();
        if line_end >= self.input.input.len() {
            self.input.input_flash = 3;
            return;
        }
        let next_line_start = line_end + 1;
        let next_line_end = self.input.input[next_line_start..]
            .find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(self.input.input.len());
        let (current_line_start, _) = self.get_current_line_bounds();
        let current_col = self.input.cursor_pos - current_line_start;
        let next_line_len = next_line_end - next_line_start;
        self.input.cursor_pos = next_line_start + current_col.min(next_line_len);
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn clamp_input_scroll(&mut self) {
        let total_lines = count_input_lines(&self.input.input);
        if total_lines <= 1 {
            self.input.input_scroll = 0;
            return;
        }
        const MAX_INPUT_HEIGHT: usize = 10;
        const BORDER_ROWS: usize = 2;
        let visible_height = MAX_INPUT_HEIGHT.saturating_sub(BORDER_ROWS);
        if total_lines <= visible_height {
            self.input.input_scroll = 0;
            return;
        }
        let pos = self.input.cursor_pos.min(self.input.input.len());
        let cursor_line = self.input.input[..pos]
            .chars()
            .filter(|&c| c == '\n')
            .count();
        if cursor_line < self.input.input_scroll {
            self.input.input_scroll = cursor_line;
        } else if cursor_line >= self.input.input_scroll + visible_height {
            self.input.input_scroll = cursor_line.saturating_sub(visible_height - 1);
        }
        let max_scroll = total_lines.saturating_sub(visible_height);
        self.input.input_scroll = self.input.input_scroll.min(max_scroll);
    }

    pub(crate) fn cursor_left(&mut self) {
        if self.input.cursor_pos > 0 {
            self.input.cursor_pos = crate::update::input::prev_grapheme_boundary(
                &self.input.input,
                self.input.cursor_pos,
            );
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
            self.input.cursor_pos = crate::update::input::next_grapheme_boundary(
                &self.input.input,
                self.input.cursor_pos,
            );
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
            self.input.cursor_pos = crate::update::input::find_word_boundary_left(
                &self.input.input,
                self.input.cursor_pos,
            );
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_right(&mut self) {
        if self.input.cursor_pos < self.input.input.len() {
            self.input.cursor_pos = crate::update::input::find_word_boundary_right(
                &self.input.input,
                self.input.cursor_pos,
            );
            self.clear_ghost();
            self.clamp_input_scroll();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
        }
    }

    pub(crate) fn handle_vim_nav_char(&mut self, c: char) {
        if c == ' ' {
            self.view.vim_nav_mode = false;
            self.insert_char(' ');
            return;
        }
        if c == 'i' {
            self.view.vim_nav_mode = false;
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
        self.view.vim_nav_mode = false;
        self.insert_char(c);
    }

    pub(crate) fn try_vim_nav_motion(&mut self, c: char) -> Option<bool> {
        let last = self.view.posts.len().saturating_sub(1);
        match c {
            'j' => Some(self.handle_vim_jump_down(last)),
            'k' => Some(self.handle_vim_jump_up()),
            'g' => {
                self.update(ScrollEvent::GoToTop);
                Some(true)
            }
            'G' => {
                self.update(ScrollEvent::GoToBottom);
                Some(true)
            }
            'y' => Some(self.handle_vim_copy(DialogEvent::CopySelectedBlock)),
            'Y' => Some(self.handle_vim_copy(DialogEvent::CopyBlockMetadata)),
            _ => None,
        }
    }

    fn handle_vim_jump_down(&mut self, last: usize) -> bool {
        if self.view.selected_post.unwrap_or(0) >= last {
            self.view.vim_nav_mode = false;
            self.mark_dirty();
            true
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    fn handle_vim_jump_up(&mut self) -> bool {
        if self.view.selected_post.unwrap_or(0) == 0 {
            self.input.input_flash = 3;
            self.mark_dirty();
            true
        } else {
            crate::update::input::element_jump_up(self);
            true
        }
    }

    fn handle_vim_copy(&mut self, evt: DialogEvent) -> bool {
        self.update(evt);
        self.view.vim_nav_mode = false;
        self.mark_dirty();
        true
    }

    pub(crate) fn handle_vim_nav_event(&mut self, event: &Event) -> Option<bool> {
        match event {
            InputEvent::HistoryPrev | ScrollEvent::Up => {
                self.vim_nav_up();
                Some(false)
            }
            InputEvent::HistoryNext | ScrollEvent::Down => {
                self.vim_nav_down();
                Some(false)
            }
            ScrollEvent::PageUp
            | ScrollEvent::PageDown
            | ScrollEvent::GoToTop
            | ScrollEvent::GoToBottom => {
                crate::update::input::scroll_event(self, event.clone());
                Some(false)
            }
            DialogEvent::ToggleCommandPalette => {
                crate::update::dialog::dialog_toggle_event(self, DialogEvent::ToggleCommandPalette);
                Some(false)
            }
            _ => Some(true),
        }
    }

    pub(crate) fn vim_nav_up(&mut self) {
        if self.view.selected_post.unwrap_or(0) == 0 {
            self.input.input_flash = 3;
            self.mark_dirty();
        } else {
            crate::update::input::element_jump_up(self);
        }
    }

    pub(crate) fn vim_nav_down(&mut self) -> bool {
        let last = self.view.posts.len().saturating_sub(1);
        if self.view.selected_post.unwrap_or(0) >= last {
            self.view.vim_nav_mode = false;
            self.mark_dirty();
            false
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    pub(crate) fn vim_motion_event(&self, c: char) -> Option<Event> {
        match c {
            'j' => Some(ScrollEvent::Up),
            'k' => Some(ScrollEvent::Down),
            'g' => Some(ScrollEvent::GoToTop),
            'G' => Some(ScrollEvent::GoToBottom),
            '/' => Some(DialogEvent::ToggleCommandPalette),
            _ => None,
        }
    }
}
