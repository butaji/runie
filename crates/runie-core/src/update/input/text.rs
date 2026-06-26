//! Text editing (insert, delete, undo/redo, paste).
//!
//! Input state mutations go through two paths:
//! 1. `InputActor` — authoritative state owner, emits `InputChanged` facts.
//! 2. Direct `AppState` mutation — keeps `AppState` in sync for synchronous tests.
//!
//! In production the async actor processes messages and emits facts. In tests
//! (where the actor may not be spawned) the direct mutations keep `AppState`
//! current so assertions pass.

use crate::model::AppState;
use crate::update::input::find_word_boundary_left;
use crate::update::input::next_grapheme_boundary;
use crate::update::input::prev_grapheme_boundary;

impl AppState {
    pub(crate) fn hint_text(&self) -> String {
        let mut parts = vec!["ctrl+o expand/collapse".to_owned()];
        parts.extend(self.mode_hints());
        parts.push("ctrl+c quit".to_owned());
        parts.join(" · ")
    }

    fn mode_hints(&self) -> Vec<String> {
        if self.open_dialog().is_some() {
            return crate::update::input::modal_hints();
        }
        if self.view().vim_nav_mode {
            return crate::update::input::vim_nav_hints();
        }
        if self.completion().at_suggestions.is_some() {
            return crate::update::input::at_suggestion_hints();
        }
        if self.agent_state().turn_active {
            return self.active_turn_hints();
        }
        if !self.input().input.is_empty() {
            return crate::update::input::input_active_hints();
        }
        if self.config().vim_mode {
            return vec!["esc nav".to_owned()];
        }
        crate::update::input::empty_input_hints()
    }

    fn active_turn_hints(&self) -> Vec<String> {
        let esc = if self.config().vim_mode {
            "esc abort·nav"
        } else {
            "esc abort"
        };
        vec![
            "enter steer".to_owned(),
            "alt+enter follow-up".to_owned(),
            esc.to_owned(),
        ]
    }

    pub(crate) fn insert_char(&mut self, c: char) {
        try_send_input(self, crate::actors::InputMsg::InsertChar(c));
        self.do_insert_char(c);
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_before_cursor(&mut self) {
        let (cursor_pos, input_len, input_text) = {
            let input = self.input();
            (input.cursor_pos, input.input.len(), input.input.clone())
        };

        if cursor_pos > 0 {
            try_send_input(self, crate::actors::InputMsg::Backspace);
            self.do_backspace();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else if cursor_pos == 0 && input_len > 0 && input_text.starts_with('\n') {
            try_send_input(self, crate::actors::InputMsg::Backspace);
            self.do_backspace();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn delete_word(&mut self) {
        let (cursor_pos, input_text) = {
            let input = self.input();
            (input.cursor_pos, input.input.clone())
        };
        if cursor_pos == 0 {
            self.input_mut().input_flash = 3;
            return;
        }
        let start = find_word_boundary_left(&input_text, cursor_pos);
        try_send_input(self, crate::actors::InputMsg::DeleteWord);
        self.push_undo();
        let input = self.input_mut();
        input.input.drain(start..cursor_pos);
        input.cursor_pos = start;
        input.redo_stack.clear();
        let _ = input;
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_to_end(&mut self) {
        let cursor_pos = self.input().cursor_pos;
        if cursor_pos < self.input().input.len() {
            try_send_input(self, crate::actors::InputMsg::DeleteToEnd);
            self.push_undo();
            self.input_mut().input.truncate(cursor_pos);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn delete_to_start(&mut self) {
        if self.input().cursor_pos > 0 {
            let cursor = self.input().cursor_pos;
            try_send_input(self, crate::actors::InputMsg::DeleteToStart);
            self.push_undo();
            self.input_mut().input.drain(..cursor);
            self.input_mut().cursor_pos = 0;
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn kill_char(&mut self) {
        let (cursor_pos, input_len) = {
            let input = self.input();
            (input.cursor_pos, input.input.len())
        };
        if cursor_pos < input_len {
            let end = next_grapheme_boundary(&self.input().input, cursor_pos);
            try_send_input(self, crate::actors::InputMsg::KillChar);
            self.push_undo();
            self.input_mut().input.drain(cursor_pos..end);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn undo(&mut self) {
        let has_undo = !self.input().undo_stack.is_empty();
        if has_undo {
            try_send_input(self, crate::actors::InputMsg::Undo);
            self.do_undo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn redo(&mut self) {
        let has_redo = !self.input().redo_stack.is_empty();
        if has_redo {
            try_send_input(self, crate::actors::InputMsg::Redo);
            self.do_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn paste(&mut self, text: &str) {
        let clean = text
            .replace("\r\n", "")
            .replace(['\r', '\n'], "")
            .replace('\t', "    ");
        if clean.is_empty() {
            return;
        }
        let clean_owned = clean.clone();
        try_send_input(self, crate::actors::InputMsg::Paste(clean_owned));
        self.push_undo();
        let cursor = self.input().cursor_pos;
        self.input_mut().input.insert_str(cursor, &clean);
        self.input_mut().cursor_pos += clean.len();
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn insert_newline(&mut self) {
        try_send_input(self, crate::actors::InputMsg::Newline);
        self.push_undo();
        let input = self.input_mut();
        if input.cursor_pos == input.input.len() {
            input.input.push('\n');
        } else {
            input.input.insert(input.cursor_pos, '\n');
        }
        input.cursor_pos += 1;
        let _ = input;
        self.clear_redo();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn push_input(&mut self, c: char) {
        if c == '\t' {
            self.tab_complete();
            return;
        }
        if self.view().vim_nav_mode {
            self.handle_vim_nav_char(c);
            return;
        }
        let is_at_trigger_position = {
            let input = self.input();
            input.input.is_empty() || input.input.ends_with(' ')
        };
        if is_at_trigger_position && self.completion().path_suggestions.is_none() {
            if c == '/' {
                self.open_command_palette_from_input();
                return;
            }
            if c == '@' {
                self.open_file_picker_from_input();
                return;
            }
            if self.config().vim_mode {
                if let Some(evt) = self.vim_motion_event(c) {
                    self.update(evt);
                    return;
                }
            }
        }
        self.insert_char(c);
    }

    fn open_command_palette_from_input(&mut self) {
        let initial_filter = self.input().input.clone();
        try_send_input(self, crate::actors::InputMsg::SetText {
            text: String::new(),
        });
        self.input_mut().input.clear();
        self.input_mut().cursor_pos = 0;
        crate::update::dialog::open_command_palette_with_filter(self, &initial_filter);
        self.view_mut().dirty = true;
    }

    fn open_file_picker_from_input(&mut self) {
        let (input_text, cursor) = {
            let input = self.input();
            (input.input.clone(), input.cursor_pos)
        };
        try_send_input(self, crate::actors::InputMsg::SetText {
            text: String::new(),
        });
        self.input_mut().input.clear();
        self.input_mut().cursor_pos = 0;
        self.input_mut().file_picker_backup =
            Some((input_text, cursor, cursor, false));
        crate::update::dialog::open_at_file_picker_all(self);
    }

    // ── Pure state mutation helpers (mirrored in InputActor) ───────────────

    fn do_insert_char(&mut self, c: char) {
        self.push_undo();
        let input = self.input_mut();
        if input.cursor_pos == input.input.len() {
            input.input.push(c);
        } else {
            input.input.insert(input.cursor_pos, c);
        }
        input.cursor_pos += c.len_utf8();
        input.redo_stack.clear();
    }

    fn do_backspace(&mut self) {
        self.push_undo();
        let input = self.input_mut();
        if input.cursor_pos > 0 {
            let new_pos = prev_grapheme_boundary(&input.input, input.cursor_pos);
            input.input.drain(new_pos..input.cursor_pos);
            input.cursor_pos = new_pos;
            input.redo_stack.clear();
        }
    }

    fn do_undo(&mut self) {
        if let Some((text, pos)) = self.input_mut().undo_stack.pop() {
            let input = self.input_mut();
            input.redo_stack.push((input.input.clone(), input.cursor_pos));
            input.input = text;
            input.cursor_pos = pos;
        }
    }

    fn do_redo(&mut self) {
        if let Some((text, pos)) = self.input_mut().redo_stack.pop() {
            let input = self.input_mut();
            input.undo_stack.push((input.input.clone(), input.cursor_pos));
            input.input = text;
            input.cursor_pos = pos;
        }
    }

    fn push_undo(&mut self) {
        let input = self.input_mut();
        let input_clone = input.input.clone();
        let cursor_clone = input.cursor_pos;
        input.undo_stack.push((input_clone, cursor_clone));
    }

    fn clear_redo(&mut self) {
        self.input_mut().redo_stack.clear();
    }
}

/// Fire-and-forget send to InputActor (when spawned).
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(ref handles) = state.actor_handles() {
        handles.try_send_input(msg);
    }
}
