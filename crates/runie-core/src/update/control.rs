//! Control event handling — navigation, session tree, and @-file picker.

use crate::model::AppState;
use crate::Event;

pub(crate) fn update(state: &mut AppState, event: Event) {
    match event {
        Event::Quit => handle_quit(state),
        Event::Reset => *state = AppState::default(),
        Event::Abort => handle_abort(state),
        Event::SpawnAgent { .. } | Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => {}
        Event::ExternalEditorDone { content } => handle_external_editor_done(state, content),
        Event::ToggleExpand => state.toggle_expand_all(),
        Event::ForkSession { message_index } => state.fork_session_at(message_index),
        Event::CloneSession => state.clone_session(),
        Event::ToggleSessionTree => state.toggle_session_tree_dialog(),
        Event::SessionTreeFilterCycle => state.cycle_session_tree_filter(),
        Event::SessionTreeSelect { id } => state.session_tree_select(&id),
        Event::AtFilePicker => state.open_at_file_picker(),
        Event::InsertAtRef(path) => state.insert_at_ref(&path),
        _ => {}
    }
}

fn handle_quit(state: &mut AppState) {
    if !state.input.input.is_empty() {
        state.input.input.clear();
        state.input.cursor_pos = 0;
        state.input.input_scroll = 0;
        state.input.undo_stack.clear();
        state.input.redo_stack.clear();
        state.mark_dirty();
    } else {
        state.should_quit = true;
    }
}

fn handle_abort(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_close();
    } else {
        state.abort_queue();
    }
}

fn handle_external_editor_done(state: &mut AppState, content: String) {
    state.input.input = content;
    state.input.cursor_pos = state.input.input.len();
    state.mark_dirty();
}
