//! Command form dialog handling and @-ref insertion.
//!
//! ## Borrow pattern
//! Form updates require `open_dialog.take()` to temporarily move the dialog out of
//! `AppState`. This is a legitimate borrow-conflict workaround: the form panel handler
//! needs `&mut AppState` to access input state, which conflicts with holding `&mut DialogState`.

use crate::commands::{DialogKind, DialogState};
use crate::model::AppState;

use super::form;

/// Handle command form dialog events. Entry point for `CommandForm*` events.
pub fn handle_form_dialog(state: &mut AppState, event: crate::Event) {
    let Some(mut dialog) = state.open_dialog_mut().take() else {
        return;
    };
    let DialogState::Active { kind: DialogKind::Generic, panels: ref mut stack } = dialog else {
        *state.open_dialog_mut() = Some(dialog);
        return;
    };
    let Some(panel) = stack.current_mut() else {
        *state.open_dialog_mut() = Some(dialog);
        return;
    };
    if !panel.is_form() {
        *state.open_dialog_mut() = Some(dialog);
        return;
    }
    let action = form::form_panel_action(state, panel, event);
    *state.open_dialog_mut() = Some(dialog);
    form::apply_form_action(state, action);
}

/// Insert filepath into input and close any dialog.
pub fn insert_at_ref(state: &mut AppState, path: &str) {
    // Record frecency for the selected file.
    if let Some(fff_state) = crate::actors::FffSearchState::get() {
        fff_state.record_file_access(std::path::Path::new(path));
    }
    let (final_text, mention_chip) = build_insert_text(state, path);
    // Merge pre-existing chips that sit before the insert point (chips at or
    // after it would need re-mapping through the replaced query token — rare
    // and safe to drop, grok-style).
    let insert_pos = mention_chip.as_ref().map_or(final_text.len(), |c| c.start);
    let mut chips: Vec<crate::model::InputChip> = state
        .input()
        .chips
        .iter()
        .filter(|c| c.start < insert_pos && c.end <= insert_pos)
        .cloned()
        .collect();
    if let Some(chip) = mention_chip {
        chips.push(chip);
    }
    state.input_mut().input = final_text.clone();
    state.input_mut().cursor_pos = state.input_mut().input.len();
    state.input_mut().chips = chips.clone();
    // Sync the authoritative InputActor: it was cleared when the picker
    // opened, so without this its next InputChanged echo clobbers the picked
    // text on the following keystroke.
    if let Some(handles) = state.actor_handles() {
        let _ = handles
            .input
            .send_message(crate::actors::InputMsg::SetText { text: final_text, chips });
    }
    *state.open_dialog_mut() = None;
    state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
    state.view_mut().dirty = true;
}

fn build_insert_text(state: &mut AppState, path: &str) -> (String, Option<crate::model::InputChip>) {
    let Some((original_input, insert_pos, cursor, _)) = state.input_mut().file_picker_backup.take() else {
        return (path.to_owned(), None);
    };
    let before = compute_before_text(&original_input, insert_pos, cursor);
    let after = extract_after(&original_input, cursor);
    // Append the range suffix if one was present (e.g. `:10-50`).
    let suffix = state
        .input
        .file_picker_range_suffix
        .take()
        .unwrap_or_default();
    // The mention chip covers the `@` + path + suffix (but NOT the trailing
    // space), so Backspace first removes the space, then the whole mention
    // atomically — grok parity.
    let chip_start = if before.ends_with('@') {
        before.len() - 1
    } else {
        before.len()
    };
    let chip_end = before.len() + path.len() + suffix.len();
    let chip = Some(crate::model::InputChip { start: chip_start, end: chip_end, label: None });
    // Trailing space after the mention so further typing appends cleanly.
    (format!("{}{}{} {}", before, path, suffix, after), chip)
}

fn compute_before_text(original_input: &str, insert_pos: usize, cursor: usize) -> String {
    let before = extract_before(original_input, insert_pos);
    let trimmed_before = before.trim_end();
    let prefix = extract_prefix(original_input, insert_pos, cursor);
    let has_word_boundary_space = insert_pos > 0 && original_input.chars().nth(insert_pos - 1) == Some(' ');

    if has_word_boundary_space {
        before
    } else if cursor >= original_input.len() && trimmed_before != prefix {
        trimmed_before.to_owned()
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
