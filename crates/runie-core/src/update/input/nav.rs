//! Cursor & vim navigation (merged from input_nav.rs).

use crate::model::{AppState, InputState};
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

/// Compute line bounds from raw input state (pure helper — no borrow of AppState).
fn compute_line_bounds(input: &InputState) -> (usize, usize) {
    if input.input.is_empty() {
        return (0, 0);
    }
    let mut line_start = 0;
    let mut line_end = input.input.len();
    let cursor_pos = input.cursor_pos;

    for (i, c) in input.input.char_indices() {
        if c == '\n' {
            if i >= cursor_pos {
                line_end = i;
                break;
            }
            line_start = i + 1;
        }
    }
    if line_end == input.input.len() && input.input.ends_with('\n') && cursor_pos > line_start {
        line_end = input.input.len();
    }
    (line_start, line_end)
}

impl AppState {
    pub(crate) fn get_current_line_bounds(&mut self) -> (usize, usize) {
        let input = self.input();
        compute_line_bounds(input)
    }

    pub(crate) fn move_cursor_to_line_start(&mut self) {
        let (line_start, _) = self.get_current_line_bounds();
        self.input_mut().cursor_pos = line_start;
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_to_line_end(&mut self) {
        let (_, line_end) = self.get_current_line_bounds();
        self.input_mut().cursor_pos = line_end;
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_up(&mut self) {
        {
            let input = self.input();
            if !input.input.contains('\n') {
                drop(input);
                self.history_prev();
                return;
            }
        }
        let (line_start, _) = {
            let input = self.input();
            compute_line_bounds(input)
        };
        if line_start == 0 {
            self.input_mut().input_flash = 3;
            return;
        }
        let prev_line_start = {
            let input = self.input();
            input.input[..line_start - 1]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0)
        };
        let current_col = {
            let input = self.input();
            input.cursor_pos - line_start
        };
        let prev_line_end = line_start - 1;
        let prev_line_len = prev_line_end - prev_line_start;
        self.input_mut().cursor_pos = prev_line_start + current_col.min(prev_line_len);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_down(&mut self) {
        let (line_end, input_len, cursor_pos) = {
            let input = self.input();
            if !input.input.contains('\n') {
                drop(input);
                self.history_next();
                return;
            }
            let (_, end) = compute_line_bounds(input);
            (end, input.input.len(), input.cursor_pos)
        };
        if line_end >= input_len {
            self.input_mut().input_flash = 3;
            return;
        }
        let next_line_start = line_end + 1;
        let next_line_end = self.input().input[next_line_start..]
            .find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(self.input().input.len());
        let (current_line_start, _) = compute_line_bounds(&self.input());
        let current_col = cursor_pos - current_line_start;
        let next_line_len = next_line_end - next_line_start;
        self.input_mut().cursor_pos = next_line_start + current_col.min(next_line_len);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn clamp_input_scroll(&mut self) {
        let input = self.input_mut();
        let total_lines = count_input_lines(&input.input);
        if total_lines <= 1 {
            input.input_scroll = 0;
            return;
        }
        const MAX_INPUT_HEIGHT: usize = 10;
        const BORDER_ROWS: usize = 2;
        let visible_height = MAX_INPUT_HEIGHT.saturating_sub(BORDER_ROWS);
        if total_lines <= visible_height {
            input.input_scroll = 0;
            return;
        }
        let pos = input.cursor_pos.min(input.input.len());
        let cursor_line = input.input[..pos].chars().filter(|&c| c == '\n').count();
        if cursor_line < input.input_scroll {
            input.input_scroll = cursor_line;
        } else if cursor_line >= input.input_scroll + visible_height {
            input.input_scroll = cursor_line.saturating_sub(visible_height - 1);
        }
        let max_scroll = total_lines.saturating_sub(visible_height);
        input.input_scroll = input.input_scroll.min(max_scroll);
    }

    pub(crate) fn cursor_left(&mut self) {
        let input = self.input_mut();
        if input.cursor_pos > 0 {
            let pos = input.cursor_pos;
            let text = input.input.clone();
            input.cursor_pos = crate::update::input::prev_grapheme_boundary(&text, pos);
            drop(input);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.input().ghost_completion.is_some() {
            self.accept_ghost();
            return;
        }
        let input = self.input_mut();
        if input.cursor_pos < input.input.len() {
            let pos = input.cursor_pos;
            let text = input.input.clone();
            input.cursor_pos = crate::update::input::next_grapheme_boundary(&text, pos);
            drop(input);
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input().input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.input().cursor_pos != 0 {
            self.input_mut().cursor_pos = 0;
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn cursor_end(&mut self) {
        if self.input().input.contains('\n') {
            self.move_cursor_to_line_end();
        } else if self.input().cursor_pos != self.input().input.len() {
            let input = self.input_mut();
            input.cursor_pos = input.input.len();
            drop(input);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_left(&mut self) {
        let input = self.input_mut();
        if input.cursor_pos > 0 {
            let pos = input.cursor_pos;
            let text = input.input.clone();
            input.cursor_pos = crate::update::input::find_word_boundary_left(&text, pos);
            drop(input);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            input.input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_right(&mut self) {
        let input = self.input_mut();
        if input.cursor_pos < input.input.len() {
            let pos = input.cursor_pos;
            let text = input.input.clone();
            input.cursor_pos = crate::update::input::find_word_boundary_right(&text, pos);
            drop(input);
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            input.input_flash = 3;
        }
    }

    pub(crate) fn handle_vim_nav_char(&mut self, c: char) {
        if c == ' ' {
            self.view_mut().vim_nav_mode = false;
            self.insert_char(' ');
            return;
        }
        if c == 'i' {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().dirty = true;
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
        self.view_mut().vim_nav_mode = false;
        self.insert_char(c);
    }

    pub(crate) fn try_vim_nav_motion(&mut self, c: char) -> Option<bool> {
        let last = self.view_mut().posts.len().saturating_sub(1);
        match c {
            'j' => Some(self.handle_vim_jump_down(last)),
            'k' => Some(self.handle_vim_jump_up()),
            'g' => {
                self.update(crate::Event::GoToTop);
                Some(true)
            }
            'G' => {
                self.update(crate::Event::GoToBottom);
                Some(true)
            }
            'y' => Some(self.handle_vim_copy(crate::Event::CopySelectedBlock)),
            'Y' => Some(self.handle_vim_copy(crate::Event::CopyBlockMetadata)),
            _ => None,
        }
    }

    fn handle_vim_jump_down(&mut self, last: usize) -> bool {
        if self.view_mut().selected_post.unwrap_or(0) >= last {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().dirty = true;
            true
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    fn handle_vim_jump_up(&mut self) -> bool {
        if self.view_mut().selected_post.unwrap_or(0) == 0 {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
            true
        } else {
            crate::update::input::element_jump_up(self);
            true
        }
    }

    fn handle_vim_copy(&mut self, evt: crate::Event) -> bool {
        self.update(evt);
        self.view_mut().vim_nav_mode = false;
        self.view_mut().dirty = true;
        true
    }

    pub(crate) fn handle_vim_nav_event(&mut self, event: &Event) -> Option<bool> {
        match event {
            crate::Event::Up => {
                self.vim_nav_up();
                Some(false)
            }
            crate::Event::Down => {
                self.vim_nav_down();
                Some(false)
            }
            crate::Event::PageUp
            | crate::Event::PageDown
            | crate::Event::GoToTop
            | crate::Event::GoToBottom => {
                crate::update::input::scroll_event(self, event.clone());
                Some(false)
            }
            crate::Event::ToggleCommandPalette => {
                crate::update::dialog::dialog_toggle_event(
                    self,
                    crate::Event::ToggleCommandPalette,
                );
                Some(false)
            }
            _ => Some(true),
        }
    }

    pub(crate) fn vim_nav_up(&mut self) {
        if self.view_mut().selected_post.unwrap_or(0) == 0 {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
        } else {
            crate::update::input::element_jump_up(self);
        }
    }

    pub(crate) fn vim_nav_down(&mut self) -> bool {
        let last = self.view_mut().posts.len().saturating_sub(1);
        if self.view_mut().selected_post.unwrap_or(0) >= last {
            self.view_mut().vim_nav_mode = false;
            self.view_mut().dirty = true;
            false
        } else {
            crate::update::input::element_jump_down(self);
            true
        }
    }

    pub(crate) fn vim_motion_event(&self, c: char) -> Option<Event> {
        match c {
            'j' => Some(crate::Event::Up),
            'k' => Some(crate::Event::Down),
            'g' => Some(crate::Event::GoToTop),
            'G' => Some(crate::Event::GoToBottom),
            '/' => Some(crate::Event::ToggleCommandPalette),
            _ => None,
        }
    }
}
