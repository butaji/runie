//! Tests for command palette closing after command execution.
//!
//! When a slash command executes (e.g., `/history`, `/session`, `/sessions`),
//! the command palette should close automatically so the result is visible
//! in the main area without an overlay.

use crate::commands::{CommandResult, DialogKind, DialogState};
use crate::dialog::{Panel, PanelStack};
use crate::model::AppState;
use crate::update::dialog::open_command_palette;
use crate::update::dialog::process_command_result;

/// Helper: open the command palette and verify it's open.
fn state_with_open_palette() -> AppState {
    let mut state = AppState::default();
    open_command_palette(&mut state);
    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::CommandPalette,
                ..
            })
        ),
        "command palette should be open"
    );
    state
}

/// Layer 1: CommandResult::Message closes the command palette.
#[test]
fn message_result_closes_command_palette() {
    let mut state = state_with_open_palette();

    process_command_result(&mut state, CommandResult::Message("Test message".into()));

    assert!(
        state.open_dialog().is_none(),
        "command palette should be closed after Message result"
    );
    assert!(
        state
            .session()
            .messages
            .iter()
            .any(|m| m.content().contains("Test message")),
        "message should be added to session"
    );
}

/// Layer 1: CommandResult::Warning closes the command palette.
#[test]
fn warning_result_closes_command_palette() {
    let mut state = state_with_open_palette();

    process_command_result(&mut state, CommandResult::Warning("Test warning".into()));

    assert!(
        state.open_dialog().is_none(),
        "command palette should be closed after Warning result"
    );
}

/// Layer 1: CommandResult::Event closes the command palette.
#[test]
fn event_result_closes_command_palette() {
    let mut state = state_with_open_palette();

    process_command_result(&mut state, CommandResult::Event(crate::Event::Reset));

    assert!(
        state.open_dialog().is_none(),
        "command palette should be closed after Event result"
    );
}

/// Layer 1: CommandResult::None closes the command palette.
#[test]
fn none_result_closes_command_palette() {
    let mut state = state_with_open_palette();

    process_command_result(&mut state, CommandResult::None);

    assert!(
        state.open_dialog().is_none(),
        "command palette should be closed after None result"
    );
}

/// Layer 1: CommandResult::OpenDialog does NOT close the palette (opens a new dialog).
#[test]
fn open_dialog_result_pushes_to_back_stack() {
    let mut state = state_with_open_palette();

    // Simulate opening settings from the palette
    process_command_result(
        &mut state,
        CommandResult::OpenDialog(crate::commands::DialogType::Settings),
    );

    // Palette should be on the back stack, settings dialog open
    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Settings,
                ..
            })
        ),
        "settings dialog should be open"
    );
    assert_eq!(
        state.dialog_back_stack().len(),
        1,
        "command palette should be on back stack"
    );
}

/// Layer 1: CommandResult::OpenPanelStack does NOT close the palette (opens a new panel stack).
#[test]
fn open_panel_stack_result_pushes_to_back_stack() {
    use crate::commands::DialogKind;

    let mut state = state_with_open_palette();
    let panels = Box::new(PanelStack::new(Panel::new("test", "Test")));

    process_command_result(&mut state, CommandResult::OpenPanelStack(panels));

    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                ..
            })
        ),
        "generic panel stack should be open"
    );
    assert_eq!(
        state.dialog_back_stack().len(),
        1,
        "command palette should be on back stack"
    );
}

/// Layer 1: Non-palette dialog is NOT closed by process_command_result.
#[test]
fn non_palette_dialog_unchanged_by_message_result() {
    use crate::update::dialog::open_settings_dialog;

    let mut state = AppState::default();
    open_settings_dialog(&mut state);

    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Settings,
                ..
            })
        ),
        "settings dialog should be open"
    );

    // Message result should NOT close settings dialog
    process_command_result(&mut state, CommandResult::Message("test".into()));

    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Settings,
                ..
            })
        ),
        "settings dialog should remain open after Message result"
    );
}

/// Layer 1: Input receiver returns to ChatInput after palette closes.
#[test]
fn input_receiver_returns_to_chat_after_palette_closes() {
    use crate::model::InputReceiver;

    let mut state = state_with_open_palette();
    assert_eq!(
        state.view.input_receiver,
        InputReceiver::Dialog,
        "input receiver should be Dialog while palette is open"
    );

    process_command_result(&mut state, CommandResult::Message("test".into()));

    assert_eq!(
        state.view.input_receiver,
        InputReceiver::ChatInput,
        "input receiver should return to ChatInput after palette closes"
    );
}

/// Layer 1: Scroll is reset when palette closes.
#[test]
fn scroll_resets_when_palette_closes() {
    let mut state = state_with_open_palette();
    state.view.scroll = 100; // Set non-zero scroll

    process_command_result(&mut state, CommandResult::Message("test".into()));

    assert_eq!(
        state.view.scroll, 0,
        "scroll should be reset when palette closes"
    );
}

/// Layer 1: View dirty flag is set when palette closes.
#[test]
fn view_marked_dirty_when_palette_closes() {
    let mut state = state_with_open_palette();
    state.view.dirty = false;

    process_command_result(&mut state, CommandResult::Message("test".into()));

    assert!(
        state.view.dirty,
        "view should be marked dirty when palette closes"
    );
}

/// Regression (live-test #3): a command palette opened from the chat-input "/"
/// autocomplete (`command_palette_from_input = true`) is ephemeral: activating
/// a command must NOT push the palette onto the back stack, so dismissing the
/// command's panel returns to the chat input. The persistent Ctrl+P command
/// bar (`command_palette_from_input = false`) keeps the menu behavior (Esc
/// returns to the palette).
fn palette_with_status_selected(from_input: bool) -> AppState {
    let mut state = AppState::default();
    state.update(crate::Event::ToggleCommandPalette);
    state.command_palette_from_input = from_input;
    // Filter down to the "status" command so the selected row is deterministic.
    if let Some(DialogState::Active { panels, .. }) = &mut state.open_dialog {
        if let Some(panel) = panels.current_mut() {
            panel.set_filter("status");
        }
    }
    state
}

#[test]
fn autocomplete_palette_returns_to_chat_after_command() {
    let mut state = palette_with_status_selected(true);

    // Activate the selected "status" command from the autocomplete palette.
    state.update(crate::Event::Submit);

    assert!(
        state.dialog_back_stack().is_empty(),
        "autocomplete palette must NOT be pushed to the back stack, len={}",
        state.dialog_back_stack().len()
    );
    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                ..
            })
        ),
        "the /status result panel should be open"
    );

    // Esc on the result panel -> back to the chat input (nothing to restore).
    state.update(crate::Event::DialogBack);
    assert!(
        state.open_dialog().is_none(),
        "autocomplete palette must return to chat after the command panel closes"
    );
}

#[test]
fn command_bar_palette_returns_to_palette_after_command() {
    let mut state = palette_with_status_selected(false);

    state.update(crate::Event::Submit);

    assert_eq!(
        state.dialog_back_stack().len(),
        1,
        "command bar palette must be pushed so Esc returns to the menu"
    );

    // Esc on the result panel -> back to the palette (menu behavior).
    state.update(crate::Event::DialogBack);
    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::CommandPalette,
                ..
            })
        ),
        "command bar must restore the palette after the command panel closes"
    );
}
