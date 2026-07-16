//! Merged input handling: dispatch, cursor/word/line nav, vim nav, text
//! editing, scroll, and history.
//!
//!   - `support`  — free helper functions (grapheme, word boundaries, hints)
//!   - `scroll`   — scroll event handler, page size, element jump
//!   - `text`     — text editing (insert/delete/paste/undo/redo)
//!   - `submit`   — submit, command dispatch, and history navigation
//!   - `nav`      — cursor move, vim nav mode, line nav, input scroll clamp

mod feed_nav;
mod nav;
mod scroll;
mod submit;
mod support;
mod text;

use crate::model::AppState;

// Re-export only what callers actually need.
pub use feed_nav::{next_response, next_turn, prev_response, prev_turn, sync_current_turn};
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
        crate::Event::TerminalSize { width, height } => {
            handle_terminal_resize(state, width, height);
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
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
        // Esc cancels plan mode, as advertised by the plan panel
        // ("[Esc] /plan off"). Plain Esc keypresses arrive as DialogBack and
        // are handled in handle_dialog_back_no_dialog; this covers the
        // Event::Escape path (tests, keystroke DSL).
        crate::Event::Escape => {
            state.update(crate::Event::PlanModeDisabled);
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
        | crate::Event::MouseScrollUp
        | crate::Event::MouseScrollDown
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

/// Navigation mode selected by history/preview key bindings.
///
/// Up/Down semantics follow grok's input model: history is only ever recalled
/// into an EMPTY input box, so drafts can never be clobbered by arrow keys.
/// The five input modes that affect Up/Down are:
/// - Path-completion suggestions open → navigate those
/// - An unmodified recalled history entry is showing → keep navigating history
/// - Multi-line draft → move the cursor vertically
/// - Empty input → recall history (latest first)
/// - Single-line draft → move the cursor to the start/end of the text
///
/// Feed scrolling uses PgUp/PgDn and Esc nav mode; earlier versions scrolled
/// the feed on Up/Down with an empty input so mouse wheels (which arrive as
/// arrow keys in alternate-scroll terminals) would scroll — that sacrificed
/// history recall from an empty box, the more familiar behavior.
pub(crate) enum HistoryNavMode {
    PathComplete,
    Recall,
    MultiLine,
    History,
    SingleLine,
}

/// Returns the navigation mode based on current input state.
///
/// The logic is shared by both history-prev and history-next handlers;
/// the caller maps the mode to up/down.
pub(crate) fn get_history_nav_mode(state: &mut AppState) -> HistoryNavMode {
    if state.completion().path_suggestions.is_some() {
        HistoryNavMode::PathComplete
    } else if state.input().history_pos.is_some() {
        // A recalled entry is showing unmodified (any edit clears
        // `history_pos` in `InputMsg::apply_to`): keep navigating history.
        HistoryNavMode::Recall
    } else if state.input().input.contains('\n') {
        HistoryNavMode::MultiLine
    } else if state.input().input.is_empty() {
        HistoryNavMode::History
    } else {
        HistoryNavMode::SingleLine
    }
}

fn handle_history_prev(state: &mut AppState) {
    match get_history_nav_mode(state) {
        HistoryNavMode::PathComplete => state.path_completion_up(),
        HistoryNavMode::Recall | HistoryNavMode::History => state.history_prev(),
        HistoryNavMode::MultiLine => state.move_cursor_up(),
        HistoryNavMode::SingleLine => state.move_cursor_to_line_start(),
    }
}

fn handle_history_next(state: &mut AppState) {
    match get_history_nav_mode(state) {
        HistoryNavMode::PathComplete => state.path_completion_down(),
        HistoryNavMode::Recall | HistoryNavMode::History => state.history_next(),
        HistoryNavMode::MultiLine => state.move_cursor_down(),
        HistoryNavMode::SingleLine => state.move_cursor_to_line_end(),
    }
}

fn handle_escape(state: &mut AppState) {
    // Close feed_element_detail overlay first
    if state.view().feed_element_detail.is_some() {
        state.view_mut().feed_element_detail = None;
        state.view_mut().dirty = true;
        return;
    }
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

#[cfg(test)]
mod tests;
