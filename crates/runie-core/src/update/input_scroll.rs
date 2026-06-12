//! Input box scroll logic — keeps cursor line visible.

use crate::model::AppState;

impl AppState {
    /// Recompute input_scroll so the cursor line is always visible.
    /// Called after any cursor movement or text change.
    pub(crate) fn clamp_input_scroll(&mut self) {
        let total_lines = count_input_lines(&self.input.input);
        if total_lines <= 1 {
            self.input.input_scroll = 0;
            return;
        }
        // Visible height: max 10 rows for input box, minus 2 for borders = 8
        const MAX_INPUT_HEIGHT: usize = 10;
        const BORDER_ROWS: usize = 2;
        let visible_height = MAX_INPUT_HEIGHT.saturating_sub(BORDER_ROWS);
        if total_lines <= visible_height {
            self.input.input_scroll = 0;
            return;
        }
        // Cursor line index
        let pos = self.input.cursor_pos.min(self.input.input.len());
        let cursor_line = self.input.input[..pos].chars().filter(|&c| c == '\n').count();
        // Ensure cursor_line is within [scroll, scroll + visible_height - 1]
        if cursor_line < self.input.input_scroll {
            self.input.input_scroll = cursor_line;
        } else if cursor_line >= self.input.input_scroll + visible_height {
            self.input.input_scroll = cursor_line.saturating_sub(visible_height - 1);
        }
        // Clamp scroll to valid range
        let max_scroll = total_lines.saturating_sub(visible_height);
        self.input.input_scroll = self.input.input_scroll.min(max_scroll);
    }
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
