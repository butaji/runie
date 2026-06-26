//! Cursor & vim navigation.
//!
//! Cursor mutations are delegated to `InputActor` via `InputMsg`.
//! View-side concerns (scroll clamp, ghost, dirty) are handled here.

use crate::model::AppState;
use crate::Event;

pub const PAGE_SIZE: usize = 5;

/// Pure helper: compute the byte offset of the start of the line containing `cursor_pos`.
fn current_line_start(input: &str, cursor_pos: usize) -> usize {
    let mut pos = 0;
    for (i, c) in input.char_indices() {
        if c == '\n' {
            if i >= cursor_pos {
                return pos;
            }
            pos = i + 1;
        }
    }
    pos
}

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
    pub(crate) fn move_cursor_to_line_start(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorStart);
        // Direct mutation for tests (when InputActor is not spawned)
        let input_text = self.input().input.clone();
        let cursor_pos = self.input().cursor_pos;
        self.input_mut().cursor_pos = current_line_start(&input_text, cursor_pos);
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_to_line_end(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorEnd);
        // Direct mutation for tests (when InputActor is not spawned)
        let input_text = self.input().input.clone();
        let cursor_pos = self.input().cursor_pos;
        let line_start = current_line_start(&input_text, cursor_pos);
        let line_end = input_text[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(input_text.len());
        self.input_mut().cursor_pos = line_end;
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_up(&mut self) {
        let input = self.input();
        if !input.input.contains('\n') {
            self.history_prev();
            return;
        }
        let input_text = input.input.clone();
        let cursor_pos = input.cursor_pos;

        let cur_start = current_line_start(&input_text, cursor_pos);
        if cur_start == 0 {
            self.input_mut().input_flash = 3;
            return;
        }
        let prev_ls = input_text[..cur_start - 1].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_le = cur_start - 1;
        let current_col = cursor_pos - cur_start;
        let new_pos = prev_ls + current_col.min(prev_le - prev_ls);
        try_send_input(self, crate::actors::InputMsg::MoveCursor { pos: new_pos });
        // Direct mutation for tests (when InputActor is not spawned)
        self.input_mut().cursor_pos = new_pos;
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn move_cursor_down(&mut self) {
        let input = self.input();
        if !input.input.contains('\n') {
            self.history_next();
            return;
        }
        let input_text = input.input.clone();
        let cursor_pos = input.cursor_pos;
        let input_len = input.input.len();
        let cur_start = current_line_start(&input_text, cursor_pos);

        // Find end of current line (cursor is at line_end).
        let line_end = input_text[cur_start..]
            .find('\n')
            .map(|i| cur_start + i)
            .unwrap_or(input_text.len());
        if line_end >= input_len {
            self.input_mut().input_flash = 3;
            return;
        }
        let next_ls = line_end + 1;
        let next_le = input_text[next_ls..]
            .find('\n')
            .map(|i| next_ls + i)
            .unwrap_or(input_text.len());
        let current_col = cursor_pos - cur_start;
        let new_pos = next_ls + current_col.min(next_le - next_ls);
        try_send_input(self, crate::actors::InputMsg::MoveCursor { pos: new_pos });
        // Direct mutation for tests (when InputActor is not spawned)
        self.input_mut().cursor_pos = new_pos;
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn clamp_input_scroll(&mut self) {
        let input = self.input();
        let total_lines = count_input_lines(&input.input);
        if total_lines <= 1 {
            let input = self.input_mut();
            input.input_scroll = 0;
            return;
        }
        const MAX_INPUT_HEIGHT: usize = 10;
        const BORDER_ROWS: usize = 2;
        let visible_height = MAX_INPUT_HEIGHT.saturating_sub(BORDER_ROWS);
        if total_lines <= visible_height {
            let input = self.input_mut();
            input.input_scroll = 0;
            return;
        }
        let pos = input.cursor_pos.min(input.input.len());
        let cursor_line = input.input[..pos].chars().filter(|&c| c == '\n').count();
        let input = self.input_mut();
        if cursor_line < input.input_scroll {
            input.input_scroll = cursor_line;
        } else if cursor_line >= input.input_scroll + visible_height {
            input.input_scroll = cursor_line.saturating_sub(visible_height - 1);
        }
        let max_scroll = total_lines.saturating_sub(visible_height);
        input.input_scroll = input.input_scroll.min(max_scroll);
    }

    pub(crate) fn cursor_left(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorLeft);
        // Direct mutation for tests (when InputActor is not spawned)
        if self.input().cursor_pos > 0 {
            let pos = self.input().cursor_pos;
            self.input_mut().cursor_pos =
                crate::update::input::prev_grapheme_boundary(&self.input().input, pos);
        } else {
            self.input_mut().input_flash = 3;
        }
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_right(&mut self) {
        if self.input().ghost_completion.is_some() {
            self.accept_ghost();
            return;
        }
        try_send_input(self, crate::actors::InputMsg::CursorRight);
        // Direct mutation for tests (when InputActor is not spawned)
        if self.input().cursor_pos < self.input().input.len() {
            let pos = self.input().cursor_pos;
            self.input_mut().cursor_pos =
                crate::update::input::next_grapheme_boundary(&self.input().input, pos);
        }
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_start(&mut self) {
        if self.input().input.contains('\n') {
            self.move_cursor_to_line_start();
        } else if self.input().cursor_pos != 0 {
            try_send_input(self, crate::actors::InputMsg::CursorStart);
            // Direct mutation for tests (when InputActor is not spawned)
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
            try_send_input(self, crate::actors::InputMsg::CursorEnd);
            // Direct mutation for tests (when InputActor is not spawned)
            self.input_mut().cursor_pos = self.input().input.len();
            self.clear_ghost();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn cursor_word_left(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorWordLeft);
        // Direct mutation for tests (when InputActor is not spawned)
        if self.input().cursor_pos > 0 {
            let pos = self.input().cursor_pos;
            self.input_mut().cursor_pos =
                crate::update::input::find_word_boundary_left(&self.input().input, pos);
        }
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn cursor_word_right(&mut self) {
        try_send_input(self, crate::actors::InputMsg::CursorWordRight);
        // Direct mutation for tests (when InputActor is not spawned)
        if self.input().cursor_pos < self.input().input.len() {
            let pos = self.input().cursor_pos;
            self.input_mut().cursor_pos =
                crate::update::input::find_word_boundary_right(&self.input().input, pos);
        }
        self.clear_ghost();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
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

/// Fire-and-forget send to InputActor.
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(ref handles) = state.actor_handles() {
        handles.try_send_input(msg);
    }
}
