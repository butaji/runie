//! Input history navigation.

use crate::model::AppState;

impl AppState {
    pub(crate) fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            self.input.input_flash = 3;
            return;
        }
        let pos = match self.input.history_pos {
            Some(p) if p > 0 => p - 1,
            Some(p) => p,
            None => self.input_history.len() - 1,
        };
        self.input.history_pos = Some(pos);
        self.input.input = self.input_history[pos].clone();
        self.input.cursor_pos = self.input.input.len();
        self.clamp_input_scroll();
        self.mark_dirty();
    }

    pub(crate) fn history_next(&mut self) {
        let pos = match self.input.history_pos {
            Some(p) => p + 1,
            None => {
                self.input.input_flash = 3;
                return;
            }
        };
        if pos >= self.input_history.len() {
            self.input.history_pos = None;
            self.input.input.clear();
            self.input.cursor_pos = 0;
        } else {
            self.input.history_pos = Some(pos);
            self.input.input = self.input_history[pos].clone();
            self.input.cursor_pos = self.input.input.len();
        }
        self.clamp_input_scroll();
        self.mark_dirty();
    }
}
