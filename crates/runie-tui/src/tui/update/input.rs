use crate::tui::state::{AppState, Msg};

/// Convert character index to byte index for String operations.
pub fn byte_idx(line: &str, char_idx: usize) -> usize {
    line.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(line.len())
}

pub fn handle_insert_char(state: &mut AppState, c: char) {
    if state.cursor_row >= state.input_lines.len() {
        state.cursor_row = state.input_lines.len().saturating_sub(1);
    }
    if state.input_lines.is_empty() {
        state.input_lines.push(String::new());
    }
    let line = &state.input_lines[state.cursor_row];
    let char_len = line.chars().count();
    if state.cursor_col > char_len {
        state.cursor_col = char_len;
    }
    let byte_pos = byte_idx(line, state.cursor_col);
    state.input_lines[state.cursor_row].insert(byte_pos, c);
    state.cursor_col += 1;
}

pub fn handle_backspace(state: &mut AppState) {
    if state.cursor_row >= state.input_lines.len() {
        state.cursor_row = state.input_lines.len().saturating_sub(1);
    }
    if state.input_lines.is_empty() {
        state.input_lines.push(String::new());
        state.cursor_col = 0;
        return;
    }
    let line = &state.input_lines[state.cursor_row];
    let char_len = line.chars().count();
    if state.cursor_col > char_len {
        state.cursor_col = char_len;
    }
    if state.cursor_col > 0 {
        let byte_pos = byte_idx(line, state.cursor_col);
        let prev_byte_pos = byte_idx(line, state.cursor_col - 1);
        state.input_lines[state.cursor_row].drain(prev_byte_pos..byte_pos);
        state.cursor_col -= 1;
    } else if state.cursor_row > 0 {
        let line = state.input_lines.remove(state.cursor_row);
        state.cursor_row -= 1;
        state.cursor_col = state.input_lines[state.cursor_row].chars().count();
        state.input_lines[state.cursor_row].push_str(&line);
    }
}

pub fn handle_insert_newline(state: &mut AppState) {
    if state.cursor_row < state.input_lines.len() {
        let byte_pos = byte_idx(&state.input_lines[state.cursor_row], state.cursor_col);
        let remainder = state.input_lines[state.cursor_row].split_off(byte_pos);
        state.cursor_row += 1;
        state.cursor_col = 0;
        state.input_lines.insert(state.cursor_row, remainder);
    }
}

pub fn handle_delete_forward(state: &mut AppState) {
    if state.cursor_row >= state.input_lines.len() {
        state.cursor_row = state.input_lines.len().saturating_sub(1);
    }
    if state.input_lines.is_empty() {
        state.input_lines.push(String::new());
        return;
    }
    let line = &state.input_lines[state.cursor_row];
    let char_len = line.chars().count();
    if state.cursor_col > char_len {
        state.cursor_col = char_len;
    }
    if state.cursor_col < char_len {
        let byte_pos = byte_idx(line, state.cursor_col);
        let next_byte_pos = byte_idx(line, state.cursor_col + 1);
        state.input_lines[state.cursor_row].drain(byte_pos..next_byte_pos);
    }
}

pub fn handle_delete_word_backward(state: &mut AppState) {
    if state.cursor_row >= state.input_lines.len() {
        state.cursor_row = state.input_lines.len().saturating_sub(1);
    }
    if state.input_lines.is_empty() {
        state.input_lines.push(String::new());
        state.cursor_col = 0;
        return;
    }
    let line = &state.input_lines[state.cursor_row];
    let char_len = line.chars().count();
    if state.cursor_col > char_len {
        state.cursor_col = char_len;
    }
    let byte_pos = byte_idx(line, state.cursor_col);
    let before = line[..byte_pos].to_string();
    if let Some(byte_idx_pos) = before.rfind(|c: char| c.is_whitespace()) {
        state.input_lines[state.cursor_row].drain(byte_idx_pos..byte_pos);
        let char_pos = before[..byte_idx_pos].chars().count();
        state.cursor_col = char_pos;
    } else {
        state.input_lines[state.cursor_row].clear();
        state.cursor_col = 0;
    }
}

pub fn handle_delete_to_start(state: &mut AppState) {
    if state.cursor_row >= state.input_lines.len() {
        state.cursor_row = state.input_lines.len().saturating_sub(1);
    }
    if state.input_lines.is_empty() {
        state.input_lines.push(String::new());
        state.cursor_col = 0;
        return;
    }
    let line = &state.input_lines[state.cursor_row];
    let char_len = line.chars().count();
    if state.cursor_col > char_len {
        state.cursor_col = char_len;
    }
    let byte_pos = byte_idx(line, state.cursor_col);
    state.input_lines[state.cursor_row].drain(..byte_pos);
    state.cursor_col = 0;
}
