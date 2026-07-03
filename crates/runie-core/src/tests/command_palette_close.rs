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
