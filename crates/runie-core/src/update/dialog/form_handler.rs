//! Command form dialog handling and @-ref insertion.

use crate::commands::DialogState;
use crate::event::DialogEvent;
use crate::model::AppState;

use super::form;

/// Handle command form dialog events. Entry point for `CommandForm*` events.
pub fn handle_form_dialog(state: &mut AppState, event: DialogEvent) {
    let action = {
        let Some(d) = &mut state.open_dialog else {
            return;
        };
        let DialogState::PanelStack(stack) = d else {
            return;
        };
        let Some(panel) = stack.current_mut() else {
            return;
        };
        if !panel.is_form() {
            return;
        }
        form::form_panel_action(panel, event)
    };
    form::apply_form_action(state, action);
}

/// Insert filepath into input and close any dialog.
pub fn insert_at_ref(state: &mut AppState, path: &str) {
    // Record frecency for the selected file.
    if let Some(fff_state) = crate::actors::FffSearchState::get() {
        fff_state.record_file_access(std::path::Path::new(path));
    }
    state.input.input = build_insert_text(state, path);
    state.input.cursor_pos = state.input.input.len();
    state.open_dialog = None;
    state.mark_dirty();
}

fn build_insert_text(state: &mut AppState, path: &str) -> String {
    let Some((original_input, insert_pos, cursor, _)) = state.input.file_picker_backup.take()
    else {
        return path.to_string();
    };
    let before = compute_before_text(&original_input, insert_pos, cursor);
    let after = extract_after(&original_input, cursor);
    // Append the range suffix if one was present (e.g. `:10-50`).
    let suffix = state
        .input
        .file_picker_range_suffix
        .take()
        .unwrap_or_default();
    format!("{}{}{}{}", before, path, suffix, after)
}

fn compute_before_text(original_input: &str, insert_pos: usize, cursor: usize) -> String {
    let before = extract_before(original_input, insert_pos);
    let trimmed_before = before.trim_end();
    let prefix = extract_prefix(original_input, insert_pos, cursor);
    let has_word_boundary_space =
        insert_pos > 0 && original_input.chars().nth(insert_pos - 1) == Some(' ');

    if has_word_boundary_space {
        before
    } else if cursor >= original_input.len() && trimmed_before != prefix {
        trimmed_before.to_string()
    } else {
        before
    }
}

fn extract_before(original_input: &str, insert_pos: usize) -> String {
    if insert_pos > 0 {
        original_input[..insert_pos].to_string()
    } else {
        String::new()
    }
}

fn extract_prefix(original_input: &str, insert_pos: usize, cursor: usize) -> &str {
    if cursor > insert_pos && insert_pos < original_input.len() {
        original_input[insert_pos..cursor].trim()
    } else {
        ""
    }
}

fn extract_after(original_input: &str, cursor: usize) -> String {
    if cursor < original_input.len() {
        original_input[cursor..].to_string()
    } else {
        String::new()
    }
}
