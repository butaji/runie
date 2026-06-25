//! Merged input handling: dispatch, cursor/word/line nav, vim nav, text
//! editing, scroll, and history.
//!
//!   - `support`  — free helper functions (grapheme, word boundaries, hints)
//!   - `scroll`   — scroll event handler, page size, element jump
//!   - `text`     — text editing (insert/delete/paste/submit/undo/redo) + history
//!   - `nav`      — cursor move, vim nav mode, line nav, input scroll clamp

mod nav;
mod scroll;
mod support;
mod text;

use crate::event::{InputEvent, ScrollEvent};
use crate::model::AppState;

// Re-export only what callers actually need.
pub use scroll::{element_jump_down, element_jump_up, scroll_event};
pub use support::{
    at_suggestion_hints, empty_input_hints, find_word_boundary_left, find_word_boundary_right,
    input_active_hints, is_quit_command, modal_hints, next_grapheme_boundary,
    prev_grapheme_boundary, vim_nav_hints,
};

#[cfg(test)]
pub use support::feed_focused_hints;

pub fn input_event(state: &mut AppState, event: InputEvent) {
    if state.permission_request.is_some() {
        return permission_input_event(state, event);
    }
    apply_input_event(state, event);
}

fn apply_input_event(state: &mut AppState, event: InputEvent) {
    match event {
        InputEvent::Input(c) => state.push_input(c),
        InputEvent::Backspace => state.pop_input(),
        InputEvent::Newline => state.insert_newline(),
        InputEvent::CursorLeft => state.cursor_left(),
        InputEvent::CursorRight => state.cursor_right(),
        InputEvent::CursorStart => state.cursor_start(),
        InputEvent::CursorEnd => state.cursor_end(),
        InputEvent::DeleteWord => state.delete_word(),
        InputEvent::DeleteToEnd => state.delete_to_end(),
        InputEvent::DeleteToStart => state.delete_to_start(),
        InputEvent::KillChar => state.kill_char(),
        InputEvent::Undo => state.undo(),
        InputEvent::Redo => state.redo(),
        InputEvent::CursorWordLeft => state.cursor_word_left(),
        InputEvent::CursorWordRight => state.cursor_word_right(),
        InputEvent::Paste(text) => state.paste(&text),
        InputEvent::PasteImage => state.paste_image(),
        InputEvent::Submit => state.submit(),
        InputEvent::Escape => handle_escape(state),
        InputEvent::PageUp => state.page_up(),
        InputEvent::PageDown => state.page_down(),
        InputEvent::GoToTop => state.go_to_top(),
        InputEvent::GoToBottom => state.go_to_bottom(),
        InputEvent::HistoryPrev => handle_history_prev(state),
        InputEvent::HistoryNext => handle_history_next(state),
        InputEvent::MouseScrollUp => scroll_event(state, ScrollEvent::Up),
        InputEvent::MouseScrollDown => scroll_event(state, ScrollEvent::Down),
        InputEvent::MouseClick { row, col, button } => {
            handle_mouse_click_event(state, row, col, &button);
        }
        InputEvent::MouseMove { row, col } => handle_mouse_move(state, row, col),
        InputEvent::TerminalSize { width, height } => {
            handle_terminal_resize(state, width, height);
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
}

fn handle_mouse_move(state: &mut AppState, row: u16, col: u16) {
    state.view.mouse_position = Some((row, col));
}

fn handle_terminal_resize(state: &mut AppState, width: u16, height: u16) {
    state.set_last_content_width(width);
    // Approximate message viewport height: full terminal minus input box,
    // status bar, and margins. This matches the legacy `view()` heuristic.
    let viewport_height = height.saturating_sub(8).max(3);
    state.set_last_visible_height(viewport_height);
}

fn permission_input_event(state: &mut AppState, event: InputEvent) {
    use crate::permissions::PermissionAction;

    let Some(req) = state.permission_request.take() else {
        return;
    };
    let action = match event {
        InputEvent::Input('y') | InputEvent::Input('Y') => PermissionAction::Allow,
        InputEvent::Input('a') | InputEvent::Input('A') => PermissionAction::Allow,
        _ => PermissionAction::Deny,
    };
    if let Ok(registry) = state.approval_registry.lock() {
        registry.resolve(&req.request_id, action);
    }
    state.view.dirty = true;
}

fn handle_mouse_click_event(state: &mut AppState, row: u16, col: u16, button: &str) {
    state.view.mouse_position = Some((row, col));
    handle_mouse_click(state, row, col, button);
}

/// Navigation mode selected by history/preview key bindings.
///
/// The three input modes that affect history navigation are:
/// - Path-completion suggestions open → navigate those
/// - Multi-line input active → move cursor vertically
/// - Otherwise → navigate session history
pub(crate) enum HistoryNavMode {
    PathComplete,
    Cursor,
    History,
}

/// Returns the navigation mode based on current input state.
///
/// The logic is shared by both history-prev and history-next handlers;
/// the caller maps the mode to up/down.
pub(crate) fn get_history_nav_mode(state: &AppState) -> HistoryNavMode {
    if state.completion.path_suggestions.is_some() {
        HistoryNavMode::PathComplete
    } else if state.input.input.contains('\n') {
        HistoryNavMode::Cursor
    } else {
        HistoryNavMode::History
    }
}

fn handle_history_prev(state: &mut AppState) {
    match get_history_nav_mode(state) {
        HistoryNavMode::PathComplete => state.path_completion_up(),
        HistoryNavMode::Cursor => state.move_cursor_up(),
        HistoryNavMode::History => state.history_prev(),
    }
}

fn handle_history_next(state: &mut AppState) {
    match get_history_nav_mode(state) {
        HistoryNavMode::PathComplete => state.path_completion_down(),
        HistoryNavMode::Cursor => state.move_cursor_down(),
        HistoryNavMode::History => state.history_next(),
    }
}

fn handle_escape(state: &mut AppState) {
    if !state.config.vim_mode {
        return;
    }
    if state.agent.turn_active {
        state.stop_turn();
        state.view.vim_nav_pending = true;
        state.view.dirty = true;
        return;
    }
    if state.view.vim_nav_pending {
        state.view.vim_nav_pending = false;
        state.view.vim_nav_mode = true;
        state.view.selected_post = state.current_bottom_post_index();
        state.view.dirty = true;
        return;
    }
    if !state.view.vim_nav_mode {
        state.view.vim_nav_mode = true;
        state.view.selected_post = state.current_bottom_post_index();
        state.view.dirty = true;
    }
}

fn handle_mouse_click(state: &mut AppState, row: u16, col: u16, button: &str) {
    use crate::snapshot::compute_mouse_target;

    let target = compute_mouse_target(
        Some((row, col)),
        state.view.last_content_width,
        state.view.last_visible_height,
        &state.input.input,
        state.has_models(),
    );

    if button == "left" {
        match target {
            crate::snapshot::MouseTarget::Input => {
                // Left-click in input area: focus the prompt, exit vim nav mode.
                if state.view.vim_nav_mode {
                    state.view.vim_nav_mode = false;
                }
                state.view.dirty = true;
            }
            crate::snapshot::MouseTarget::Feed => {
                // Left-click in feed: toggle collapse-all (same as Ctrl+O).
                state.toggle_expand_all();
            }
            // intentionally ignored: other targets fall through
            _ => {}
        }
    }
    // Middle-click and right-click are intentionally ignored here;
    // middle-click paste is future work.
}

#[cfg(test)]
mod tests;
