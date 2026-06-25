//! Text editing (insert, delete, undo/redo, paste).

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
        self.push_undo();
        let input = self.input_mut();
        if input.cursor_pos == input.input.len() {
            input.input.push(c);
        } else {
            input.input.insert(input.cursor_pos, c);
        }
        input.cursor_pos += c.len_utf8();
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_before_cursor(&mut self) {
        let input = self.input();
        if input.cursor_pos > 0 {
            drop(input);
            self.delete_before_cursor_with_content();
        } else if input.cursor_pos == 0 && !input.input.is_empty() {
            drop(input);
            self.delete_leading_newline();
        } else {
            self.input_mut().input_flash = 3;
        }
    }

    fn delete_before_cursor_with_content(&mut self) {
        let input = self.input();
        let char_before_cursor = input.input[..input.cursor_pos].chars().last();
        drop(input);
        if char_before_cursor == Some('\n') {
            self.remove_newline_before_cursor();
        } else {
            self.remove_grapheme_before_cursor();
        }
    }

    fn remove_newline_before_cursor(&mut self) {
        self.push_undo();
        let input = self.input_mut();
        let new_pos = input.cursor_pos - 1;
        input.input.remove(input.cursor_pos - 1);
        input.cursor_pos = new_pos;
        drop(input);
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    fn remove_grapheme_before_cursor(&mut self) {
        self.push_undo();
        let input = self.input();
        let new_pos = crate::update::input::prev_grapheme_boundary(&input.input, input.cursor_pos);
        drop(input);
        let input = self.input_mut();
        input.input.drain(new_pos..input.cursor_pos);
        input.cursor_pos = new_pos;
        drop(input);
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    fn delete_leading_newline(&mut self) {
        let input = self.input();
        if input.input.starts_with('\n') {
            drop(input);
            self.push_undo();
            self.input_mut().input.remove(0);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            drop(input);
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn delete_word(&mut self) {
        let input = self.input();
        if input.cursor_pos == 0 {
            drop(input);
            self.input_mut().input_flash = 3;
            return;
        }
        let start = crate::update::input::find_word_boundary_left(&input.input, input.cursor_pos);
        drop(input);
        self.push_undo();
        let input = self.input_mut();
        input.input.drain(start..input.cursor_pos);
        input.cursor_pos = start;
        drop(input);
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn delete_to_end(&mut self) {
        let cursor_pos = self.input().cursor_pos;
        if cursor_pos < self.input().input.len() {
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
        let input = self.input();
        if input.cursor_pos > 0 {
            drop(input);
            self.push_undo();
            let cursor = self.input().cursor_pos;
            self.input_mut().input.drain(..cursor);
            self.input_mut().cursor_pos = 0;
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            drop(input);
            self.input_mut().input_flash = 3;
        }
    }

    pub(crate) fn kill_char(&mut self) {
        let input = self.input();
        if input.cursor_pos < input.input.len() {
            let end = crate::update::input::next_grapheme_boundary(&input.input, input.cursor_pos);
            drop(input);
            self.push_undo();
            let input = self.input_mut();
            input.input.drain(input.cursor_pos..end);
            drop(input);
            self.clear_redo();
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        } else {
            drop(input);
            self.input_mut().input_flash = 3;
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

    pub(crate) fn undo(&mut self) {
        if let Some((text, pos)) = self.input_mut().undo_stack.pop() {
            let input = self.input_mut();
            input
                .redo_stack
                .push((input.input.clone(), input.cursor_pos));
            input.input = text;
            input.cursor_pos = pos;
            drop(input);
            self.handle_at_trigger();
            self.clamp_input_scroll();
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some((text, pos)) = self.input_mut().redo_stack.pop() {
            let input = self.input_mut();
            input
                .undo_stack
                .push((input.input.clone(), input.cursor_pos));
            input.input = text;
            input.cursor_pos = pos;
            drop(input);
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
        self.push_undo();
        let cursor = self.input().cursor_pos;
        self.input_mut().input.insert_str(cursor, &clean);
        self.input_mut().cursor_pos += clean.len();
        self.clear_redo();
        self.handle_at_trigger();
        self.clamp_input_scroll();
        self.view_mut().dirty = true;
    }

    pub(crate) fn paste_image(&mut self) {
        match crate::clipboard_image::read_clipboard_image() {
            Some(bytes) => {
                let uri = crate::clipboard_image::to_data_uri(&bytes);
                self.session_mut().image_attachments.push(uri);
                self.view_mut().dirty = true;
            }
            None => {
                self.input_mut().input_flash = 3;
            }
        }
    }

    pub(crate) fn pop_input(&mut self) {
        self.delete_before_cursor();
    }

    pub(crate) fn insert_newline(&mut self) {
        self.push_undo();
        let input = self.input_mut();
        if input.cursor_pos == input.input.len() {
            input.input.push('\n');
        } else {
            input.input.insert(input.cursor_pos, '\n');
        }
        input.cursor_pos += 1;
        drop(input);
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
        let input = self.input();
        let is_at_trigger_position = input.input.is_empty() || input.input.ends_with(' ');
        drop(input);
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
        {
            let input = self.input_mut();
            input.input.clear();
            input.cursor_pos = 0;
        }
        crate::update::dialog::open_command_palette_with_filter(self, &initial_filter);
        self.view_mut().dirty = true;
    }

    fn open_file_picker_from_input(&mut self) {
        let input = self.input();
        let needs_brackets = false;
        let cursor = input.cursor_pos;
        let file_picker_backup = Some((input.input.clone(), cursor, cursor, needs_brackets));
        drop(input);
        self.input_mut().file_picker_backup = file_picker_backup;
        crate::update::dialog::open_at_file_picker_all(self);
    }
}
