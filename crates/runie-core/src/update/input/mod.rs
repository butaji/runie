//! Merged input handling: dispatch, cursor/word/line nav, vim nav, text
//! editing, scroll, and history.
//!
//!   - `support`  — free helper functions (grapheme, word boundaries, hints)
//!   - `scroll`   — scroll event handler, page size, element jump
//!   - `text`     — text editing (insert/delete/paste/undo/redo)
//!   - `submit`   — submit, command dispatch, and history navigation
//!   - `nav`      — cursor move, vim nav mode, line nav, input scroll clamp

mod nav;
mod scroll;
mod submit;
mod support;
mod text;

use crate::model::AppState;

// Re-export only what callers actually need.
pub use scroll::{element_jump_down, element_jump_up, scroll_event};
pub use support::{
    at_suggestion_hints, empty_input_hints, find_word_boundary_left, find_word_boundary_right,
    input_active_hints, is_quit_command, modal_hints, next_grapheme_boundary,
    prev_grapheme_boundary, vim_nav_hints,
};

pub fn input_event(state: &mut AppState, event: crate::Event) {
    if state.view().plan_mode {
        return plan_mode_input_event(state, event);
    }
    apply_input_event(state, event);
}

fn apply_input_event(state: &mut AppState, event: crate::Event) {
    match event {
        crate::Event::Input(c) => state.push_input(c),
        crate::Event::Backspace => state.pop_input(),
        crate::Event::Newline => state.insert_newline(),
        crate::Event::CursorLeft => state.cursor_left(),
        crate::Event::CursorRight => state.cursor_right(),
        crate::Event::CursorStart => state.cursor_start(),
        crate::Event::CursorEnd => state.cursor_end(),
        crate::Event::DeleteWord => state.delete_word(),
        crate::Event::DeleteToEnd => state.delete_to_end(),
        crate::Event::DeleteToStart => state.delete_to_start(),
        crate::Event::KillChar => state.kill_char(),
        crate::Event::Undo => state.undo(),
        crate::Event::Redo => state.redo(),
        crate::Event::CursorWordLeft => state.cursor_word_left(),
        crate::Event::CursorWordRight => state.cursor_word_right(),
        crate::Event::Paste(text) => state.paste(&text),
        crate::Event::PasteImage => handle_paste_image(state),
        crate::Event::Submit => state.submit(),
        crate::Event::Escape => handle_escape(state),
        crate::Event::PageUp => state.page_up(),
        crate::Event::PageDown => state.page_down(),
        crate::Event::GoToTop => state.go_to_top(),
        crate::Event::GoToBottom => state.go_to_bottom(),
        crate::Event::HistoryPrev => handle_history_prev(state),
        crate::Event::HistoryNext => handle_history_next(state),
        crate::Event::MouseScrollUp => scroll_event(state, crate::Event::Up),
        crate::Event::MouseScrollDown => scroll_event(state, crate::Event::Down),
        crate::Event::MouseClick { row, col, button } => {
            handle_mouse_click_event(state, row, col, &button);
        }
        crate::Event::MouseMove { row, col } => handle_mouse_move(state, row, col),
        crate::Event::TerminalSize { width, height } => {
            handle_terminal_resize(state, width, height);
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
}

fn handle_mouse_move(state: &mut AppState, row: u16, col: u16) {
    state.view_mut().mouse_position = Some((row, col));
}

fn handle_terminal_resize(state: &mut AppState, width: u16, height: u16) {
    state.set_last_content_width(width);
    // Approximate message viewport height: full terminal minus input box,
    // status bar, and margins. This matches the legacy `view()` heuristic.
    let viewport_height = height.saturating_sub(8).max(3);
    state.set_last_visible_height(viewport_height);
}

/// Handle PasteImage event. Image paste was removed, so just flash.
fn handle_paste_image(state: &mut AppState) {
    state.input_mut().input_flash = 3;
}

/// Handle input events while plan mode is active.
///
/// - `Enter` approves the plan (disables plan mode)
/// - `Esc` cancels/disables plan mode
/// - Navigation keys are consumed silently (no-ops)
/// - All other keys are routed to the input box for plan editing
fn plan_mode_input_event(state: &mut AppState, event: crate::Event) {
    match event {
        // Approve plan — disable plan mode
        crate::Event::Submit | crate::Event::Newline => {
            state.view_mut().plan_mode = false;
            state.view_mut().active_plan_content.clear();
            state.view_mut().dirty = true;
            state.add_system_msg("Plan approved. Write tools unblocked.".to_string());
        }
        // Navigation and editing: consume silently (plan editing via panel)
        crate::Event::Backspace
        | crate::Event::DeleteWord
        | crate::Event::DeleteToEnd
        | crate::Event::DeleteToStart
        | crate::Event::KillChar
        | crate::Event::Undo
        | crate::Event::Redo
        | crate::Event::CursorWordLeft
        | crate::Event::CursorWordRight
        | crate::Event::CursorLeft
        | crate::Event::CursorRight
        | crate::Event::CursorStart
        | crate::Event::CursorEnd
        | crate::Event::HistoryPrev
        | crate::Event::HistoryNext
        | crate::Event::PageUp
        | crate::Event::PageDown
        | crate::Event::GoToTop
        | crate::Event::GoToBottom
        | crate::Event::MouseClick { .. }
        | crate::Event::MouseMove { .. }
        | crate::Event::MouseScrollUp
        | crate::Event::MouseScrollDown
        | crate::Event::MouseDrag { .. }
        | crate::Event::MouseRelease { .. }
        | crate::Event::FocusGained
        | crate::Event::FocusLost
        | crate::Event::TerminalSize { .. } => {
            // Consumed silently — do not route to input box
        }
        // Route all other input to the input box for plan content editing
        _ => {
            apply_input_event(state, event);
        }
    }
}

fn handle_mouse_click_event(state: &mut AppState, row: u16, col: u16, button: &str) {
    state.view_mut().mouse_position = Some((row, col));
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
pub(crate) fn get_history_nav_mode(state: &mut AppState) -> HistoryNavMode {
    if state.completion().path_suggestions.is_some() {
        HistoryNavMode::PathComplete
    } else if state.input().input.contains('\n') {
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
    if !state.config_mut().vim_mode {
        return;
    }
    if state.agent_state_mut().turn_active {
        state.stop_turn();
        state.view_mut().vim_nav_pending = true;
        state.view_mut().dirty = true;
        return;
    }
    if state.view().vim_nav_pending {
        state.view_mut().vim_nav_pending = false;
        state.view_mut().vim_nav_mode = true;
        state.view_mut().dirty = true;
        let selected = state.current_bottom_post_index();
        state.view_mut().selected_post = selected;
        return;
    }
    let entering = !state.view().vim_nav_mode;
    {
        let view = state.view_mut();
        view.vim_nav_mode = entering;
        if !entering {
            view.selected_post = None;
        }
        view.dirty = true;
    }
    if entering {
        let selected = state.current_bottom_post_index();
        state.view_mut().selected_post = selected;
    }
}

fn handle_mouse_click(state: &mut AppState, row: u16, col: u16, button: &str) {
    use crate::snapshot::compute_mouse_target;

    let last_content_width = state.view().last_content_width;
    let last_visible_height = state.view().last_visible_height;
    let has_models = state.has_models();
    let input = state.input().input.clone();

    let target = compute_mouse_target(
        Some((row, col)),
        last_content_width,
        last_visible_height,
        &input,
        has_models,
    );

    if button == "left" {
        match target {
            crate::snapshot::MouseTarget::Input => {
                // Left-click in input area: focus the prompt, exit vim nav mode.
                if state.view_mut().vim_nav_mode {
                    state.view_mut().vim_nav_mode = false;
                    state.view_mut().selected_post = None;
                }
                state.view_mut().dirty = true;
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
