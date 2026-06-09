//! Line navigation helpers for multi-line input

use crate::model::AppState;

impl AppState {
    /// Get the current line boundaries (start and end positions, exclusive end)
    pub(crate) fn get_current_line_bounds(&self) -> (usize, usize) {
        if self.input.is_empty() {
            return (0, 0);
        }

        // Find which line the cursor is on
        let mut current_pos = 0;
        let mut line_start = 0;
        let mut line_end = self.input.len();

        for (i, c) in self.input.char_indices() {
            if c == '\n' {
                if i >= self.cursor_pos {
                    line_end = i;
                    break;
                }
                line_start = i + 1;
            }
        }

        // Check if cursor is past the last newline (last line)
        if line_end == self.input.len() && self.input.ends_with('\n') {
            // Input ends with newline, last line is empty
            if self.cursor_pos > line_start {
                line_end = self.input.len();
            }
        }

        (line_start, line_end)
    }

    /// Move cursor to the start of the current line
    pub(crate) fn move_cursor_to_line_start(&mut self) {
        let (line_start, _) = self.get_current_line_bounds();
        self.cursor_pos = line_start;
        self.mark_dirty();
    }

    /// Move cursor to the end of the current line
    pub(crate) fn move_cursor_to_line_end(&mut self) {
        let (_, line_end) = self.get_current_line_bounds();
        self.cursor_pos = line_end;
        self.mark_dirty();
    }

    /// Move cursor up one line (if in multi-line mode)
    pub(crate) fn move_cursor_up(&mut self) {
        if !self.input.contains('\n') {
            // Single line - do history navigation
            self.history_prev();
            return;
        }

        let (line_start, _) = self.get_current_line_bounds();

        // Find the previous line
        if line_start == 0 {
            // Already on first line - flash
            self.input_flash = 3;
            return;
        }

        // Find start of previous line
        let prev_line_start = self.input[..line_start - 1]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Calculate the column we're on
        let current_col = self.cursor_pos - line_start;

        // Find the previous line's end
        let prev_line_end = line_start - 1;
        let prev_line_len = prev_line_end - prev_line_start;

        // Position cursor at same column (or end of line if line is shorter)
        self.cursor_pos = prev_line_start + current_col.min(prev_line_len);
        self.mark_dirty();
    }

    /// Move cursor down one line (if in multi-line mode)
    pub(crate) fn move_cursor_down(&mut self) {
        if !self.input.contains('\n') {
            // Single line - do history navigation
            self.history_next();
            return;
        }

        let (_, line_end) = self.get_current_line_bounds();

        // Check if we're on the last line
        if line_end >= self.input.len() {
            // Already on last line - flash
            self.input_flash = 3;
            return;
        }

        // Skip the newline and find next line's start
        let next_line_start = line_end + 1;

        // Find the end of the next line
        let next_line_end = self.input[next_line_start..]
            .find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(self.input.len());

        // Calculate the column we're on
        let (current_line_start, _) = self.get_current_line_bounds();
        let current_col = self.cursor_pos - current_line_start;

        // Position cursor at same column (or end of line if line is shorter)
        let next_line_len = next_line_end - next_line_start;
        self.cursor_pos = next_line_start + current_col.min(next_line_len);
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use crate::model::AppState;

    #[test]
    fn single_line_cursor_start_end() {
        let mut state = AppState::default();
        state.input = "hello world".to_string();
        state.cursor_pos = 5;

        state.move_cursor_to_line_start();
        assert_eq!(state.cursor_pos, 0);

        state.move_cursor_to_line_end();
        assert_eq!(state.cursor_pos, 11);
    }

    #[test]
    fn multiline_line_bounds() {
        let mut state = AppState::default();
        // "line1\nline2\nline3"
        //  01234 5  6789 10 11  1213141516
        state.input = "line1\nline2\nline3".to_string();

        // Cursor on first line (column 3)
        state.cursor_pos = 3;
        let (start, end) = state.get_current_line_bounds();
        assert_eq!(start, 0, "first line start");
        assert_eq!(end, 5, "first line end (exclusive)");

        // Cursor on second line (column 2)
        state.cursor_pos = 8;
        let (start, end) = state.get_current_line_bounds();
        assert_eq!(start, 6, "second line start");
        assert_eq!(end, 11, "second line end (exclusive)");

        // Cursor on last line (column 3)
        state.cursor_pos = 15;
        let (start, end) = state.get_current_line_bounds();
        assert_eq!(start, 12, "last line start");
        assert_eq!(end, 17, "last line end (exclusive)");
    }

    #[test]
    fn move_cursor_up_first_line_flashes() {
        let mut state = AppState::default();
        state.input = "line1\nline2".to_string();
        state.cursor_pos = 2; // On first line

        state.move_cursor_up();
        assert_eq!(state.input_flash, 3); // Flash indicator
    }

    #[test]
    fn move_cursor_up_navigates() {
        let mut state = AppState::default();
        state.input = "line1\nline2".to_string();
        state.cursor_pos = 8; // On second line (column 2)

        state.move_cursor_up();
        assert_eq!(state.cursor_pos, 2); // First line, same column
    }

    #[test]
    fn move_cursor_down_last_line_flashes() {
        let mut state = AppState::default();
        state.input = "line1\nline2".to_string();
        state.cursor_pos = 8; // On last line

        state.move_cursor_down();
        assert_eq!(state.input_flash, 3); // Flash indicator
    }

    #[test]
    fn move_cursor_down_navigates() {
        let mut state = AppState::default();
        state.input = "line1\nline2\nline3".to_string();
        state.cursor_pos = 2; // On first line (column 2)

        state.move_cursor_down();
        assert_eq!(state.cursor_pos, 8); // Second line, same column

        state.move_cursor_down();
        assert_eq!(state.cursor_pos, 14); // Third line, same column
    }

    #[test]
    fn cursor_preserves_column_when_moving() {
        let mut state = AppState::default();
        // "ab\nabcdef" - Line 0 has 2 chars, Line 1 has 6 chars
        state.input = "ab\nabcdef".to_string();
        state.cursor_pos = 6; // On second line, column 3 ('d')

        state.move_cursor_up();
        // First line has 2 chars, column 3 clamps to 2
        assert_eq!(state.cursor_pos, 2); // End of first line

        state.move_cursor_down();
        // Back to second line, column 2 ('c')
        assert_eq!(state.cursor_pos, 5);
    }

    #[test]
    fn column_clamped_to_line_length() {
        let mut state = AppState::default();
        state.input = "ab\nabcdef".to_string();
        state.cursor_pos = 10; // On second line, column 6

        state.move_cursor_up();
        // First line is shorter (2 chars), column should be clamped
        assert_eq!(state.cursor_pos, 2); // End of first line
    }
}
