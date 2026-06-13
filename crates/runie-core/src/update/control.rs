//! Control Event Handler

use crate::model::AppState;
use crate::Event;

pub fn control_event(state: &mut AppState, event: Event) {
    match event {
        Event::Quit => handle_quit(state),
        Event::Reset => handle_reset(state),
        Event::Abort => handle_abort(state),
        Event::ExternalEditorDone { content } => handle_editor_done(state, content),
        Event::ToggleExpand => state.toggle_expand_all(),
        Event::ToggleSessionTree => {
            state.toggle_session_tree_dialog();
            state.view.cached_session_tree_valid = false;
        }
        Event::SessionTreeFilterCycle => state.cycle_session_tree_filter(),
        Event::ForkSession { message_index } => {
            state.fork_session_at(message_index);
            state.view.cached_session_tree_valid = false;
        }
        Event::CloneSession => {
            state.clone_session();
            state.view.cached_session_tree_valid = false;
        }
        Event::SessionTreeSelect { id } => state.session_tree_select(&id),
        Event::SpawnAgent { .. } | Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => {}
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

fn handle_reset(state: &mut AppState) {
    *state = AppState::default();
}

fn handle_abort(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_close();
    } else {
        state.abort_queue();
    }
}

fn handle_editor_done(state: &mut AppState, content: String) {
    state.input.input = content;
    state.input.cursor_pos = state.input.input.len();
    state.mark_dirty();
}
