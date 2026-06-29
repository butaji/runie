//! Text editing (insert, delete, undo/redo, paste).
//!
//! All authoritative input mutations go through `InputActor` via `InputMsg`.
//! `InputActor` emits `Event::InputChanged` which updates `AppState` projection.
//! UI side effects (flash, dirty flag, completion triggers) are handled here.

use crate::model::AppState;

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
        self.handle_at_trigger();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_before_cursor(&mut self) {
        let (cursor_pos, input_len, input_text) = {
            let input = self.input();
            (input.cursor_pos, input.input.len(), input.input.clone())
        };

        let should_backspace = cursor_pos > 0
            || (cursor_pos == 0 && input_len > 0 && input_text.starts_with('\n'));
        if should_backspace {
            try_send_input(self, crate::actors::InputMsg::Backspace);
            self.handle_at_trigger();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn delete_word(&mut self) {
        let cursor_pos = self.input().cursor_pos;
        if cursor_pos == 0 {
            self.input_mut().input_flash = 3;
            return;
        }
        try_send_input(self, crate::actors::InputMsg::DeleteWord);
        self.handle_at_trigger();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_to_end(&mut self) {
        let cursor_pos = self.input().cursor_pos;
        if cursor_pos < self.input().input.len() {
            try_send_input(self, crate::actors::InputMsg::DeleteToEnd);
            self.handle_at_trigger();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn delete_to_start(&mut self) {
        if self.input().cursor_pos > 0 {
            try_send_input(self, crate::actors::InputMsg::DeleteToStart);
            self.handle_at_trigger();
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
            try_send_input(self, crate::actors::InputMsg::KillChar);
            self.handle_at_trigger();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn undo(&mut self) {
        let has_undo = !self.input().undo_stack.is_empty();
        if has_undo {
            try_send_input(self, crate::actors::InputMsg::Undo);
            self.handle_at_trigger();
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn redo(&mut self) {
        let has_redo = !self.input().redo_stack.is_empty();
        if has_redo {
            try_send_input(self, crate::actors::InputMsg::Redo);
            self.handle_at_trigger();
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
        try_send_input(self, crate::actors::InputMsg::Paste(clean));
        self.handle_at_trigger();
        self.view_mut().dirty = true;
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn insert_newline(&mut self) {
        try_send_input(self, crate::actors::InputMsg::Newline);
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
        self.input_mut().file_picker_backup = Some((input_text, cursor, cursor, false));
        crate::update::dialog::open_at_file_picker_all(self);
    }
}

/// Fire-and-forget send to InputActor (when spawned).
/// In test mode (no actor handles), applies the mutation synchronously so that
/// synchronous tests can assert on the updated state without awaiting the actor.
fn try_send_input(state: &mut AppState, msg: crate::actors::InputMsg) {
    if let Some(handles) = state.actor_handles() {
        let _ = handles.input.try_send(msg);
    } else {
        // Test mode: apply synchronously to AppState projection.
        msg.apply_to(state.input_mut());
    }
}
