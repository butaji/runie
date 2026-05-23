use crate::tui::state::{AppState, Msg};

pub fn handle_move_left(state: &mut AppState) {
    if state.cursor_col > 0 {
        state.cursor_col -= 1;
    } else if state.cursor_row > 0 {
        state.cursor_row -= 1;
        state.cursor_col = state.input_lines[state.cursor_row].chars().count();
    }
}

pub fn handle_move_right(state: &mut AppState) {
    let char_len = state.input_lines[state.cursor_row].chars().count();
    if state.cursor_col < char_len {
        state.cursor_col += 1;
    } else if state.cursor_row + 1 < state.input_lines.len() {
        state.cursor_row += 1;
        state.cursor_col = 0;
    }
}

pub fn handle_move_up(state: &mut AppState) {
    if state.cursor_row > 0 {
        state.cursor_row -= 1;
        let char_len = state.input_lines[state.cursor_row].chars().count();
        state.cursor_col = state.cursor_col.min(char_len);
    }
}

pub fn handle_move_down(state: &mut AppState) {
    if state.cursor_row + 1 < state.input_lines.len() {
        state.cursor_row += 1;
        let char_len = state.input_lines[state.cursor_row].chars().count();
        state.cursor_col = state.cursor_col.min(char_len);
    }
}

pub fn handle_cursor_edge(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::MoveCursorToStart => state.cursor_col = 0,
        Msg::MoveCursorToEnd => state.cursor_col = state.input_lines[state.cursor_row].chars().count(),
        _ => {}
    }
}

pub fn handle_cursor_move(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::MoveCursorLeft => handle_move_left(state),
        Msg::MoveCursorRight => handle_move_right(state),
        Msg::MoveCursorUp => handle_move_up(state),
        Msg::MoveCursorDown => handle_move_down(state),
        _ => {}
    }
}
