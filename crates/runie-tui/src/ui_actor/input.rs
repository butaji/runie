//! Input handling, autocomplete detection, and form detection.

use crate::ui_actor::UiActor;

impl UiActor {
    /// Detect autocomplete trigger characters ('@' or '/') typed at end of input.
    /// Opens the command palette or file picker accordingly.
    pub(crate) fn detect_autocomplete_trigger(
        &mut self,
        prev_input: &str,
        _prev_cursor: usize,
        new_input: &str,
        new_cursor: usize,
    ) {
        // Detect '@' or '/' typed at end of input (not inside existing autocomplete).
        let was_empty_or_space =
            prev_input.is_empty() || prev_input.ends_with(' ') || prev_input.ends_with('\n');

        if was_empty_or_space
            && !new_input.is_empty()
            && new_cursor == new_input.len()
            && self.state.completion().at_suggestions.is_none()
        {
            let last_char = new_input.chars().last().unwrap();
            if last_char == '@' {
                // Open file picker via event.
                // UiActor-specific: save input state before picker opens (projection state).
                let (input_text, cursor) = (new_input.to_owned(), new_cursor);
                self.state.input_mut().file_picker_backup =
                    Some((input_text, cursor, cursor, false));
                // Route through event: UiActor's apply_event will call
                // dialog_toggle_event which calls open_at_file_picker_all.
                self.apply_event(runie_core::Event::AtFilePicker);
            } else if last_char == '/' && !Self::is_quit_command(new_input) {
                // Open command palette via event.
                // UiActor-specific: clear input projection before palette opens.
                self.state.input_mut().input = String::new();
                self.state.input_mut().cursor_pos = 0;
                // Route through event: UiActor's apply_event will call
                // dialog_toggle_event which calls open_command_palette.
                self.apply_event(runie_core::Event::ToggleCommandPalette);
            }
        }
    }

    /// Handle autocomplete trigger at current cursor position.
    pub(crate) fn handle_at_trigger(&mut self) {
        let input = self.state.input();
        let is_empty_or_space =
            input.input.is_empty() || input.input.ends_with(' ') || input.input.ends_with('\n');
        if is_empty_or_space
            || self.state.completion().at_suggestions.is_some()
            || input.input.ends_with('@')
        {
            return;
        }

        let last_char = input.input.chars().last().unwrap();
        if last_char == '@' && input.cursor_pos == input.input.len() {
            // File picker: already opened in detect_autocomplete_trigger.
            return;
        }

        if last_char == '/' && !Self::is_quit_command(&input.input) {
            // Command palette: already opened in detect_autocomplete_trigger.
        }
    }
}
