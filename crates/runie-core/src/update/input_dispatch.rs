//! Input event dispatcher.

use crate::model::AppState;
use crate::Event;

pub(crate) fn input_event(state: &mut AppState, event: Event) {
    match event {
        Event::Input(c) => state.push_input(c),
        Event::Backspace => state.pop_input(),
        Event::Newline => state.insert_newline(),
        Event::CursorLeft => state.cursor_left(),
        Event::CursorRight => state.cursor_right(),
        Event::CursorStart => state.cursor_start(),
        Event::CursorEnd => state.cursor_end(),
        Event::DeleteWord => state.delete_word(),
        Event::DeleteToEnd => state.delete_to_end(),
        Event::DeleteToStart => state.delete_to_start(),
        Event::KillChar => state.kill_char(),
        Event::Undo => state.undo(),
        Event::Redo => state.redo(),
        Event::CursorWordLeft => state.cursor_word_left(),
        Event::CursorWordRight => state.cursor_word_right(),
        Event::Paste(text) => state.paste(&text),
        Event::PasteImage => state.paste_image(),
        Event::Submit => state.submit(),
        Event::HistoryPrev => handle_history_prev(state),
        Event::HistoryNext => handle_history_next(state),
        Event::InsertAtRef(path) => super::dialog::insert_at_ref(state, &path),
        _ => {}
    }
}

fn handle_history_prev(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_up();
    } else if state.input.input.contains('\n') {
        state.move_cursor_up();
    } else {
        state.history_prev();
    }
}

fn handle_history_next(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_down();
    } else if state.input.input.contains('\n') {
        state.move_cursor_down();
    } else {
        state.history_next();
    }
}
