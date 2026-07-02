//! Helper functions for UiActor.

use runie_core::commands::{DialogKind, DialogState};
use runie_core::{AppState, Event};

/// Returns `true` for events that should be silently consumed (no-op) while
/// a permission dialog is open. These keys must NOT deny the request and must
/// NOT be routed to the input box.
///
/// This intentionally does NOT include `Event::Input` — those are handled
/// separately in `handle_input_event` to resolve the permission.
pub fn is_navigation_or_editing_event(evt: &Event) -> bool {
    matches!(
        evt,
        Event::Escape
            | Event::Backspace
            | Event::Newline
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::Undo
            | Event::Redo
            | Event::Paste(_)
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom
            | Event::Submit
            | Event::MouseScrollUp
            | Event::MouseScrollDown
            | Event::MouseClick { .. }
            | Event::MouseMove { .. }
            | Event::TerminalSize { .. }
    )
}

/// Returns `true` if a Generic dialog with a form panel is currently open
/// and no login flow is active.
///
/// Used to route Enter/Tab to the command form handler instead of the input box.
/// Login flow dialogs also use Generic + Form panels but have their own submission
/// mechanism (button actions emit `Event::Save`), so they are excluded.
pub fn is_form_dialog_open(state: &AppState) -> bool {
    // Exclude login flow: it uses Generic+Form panels but its submit button
    // emits Event::Save (handled by login_flow_event), not CommandFormSubmit.
    if state.login_flow().is_some() {
        return false;
    }
    state.open_dialog().is_some_and(|d| {
        if let DialogState::Active {
            kind: DialogKind::Generic,
            panels,
        } = d
        {
            panels.current().is_some_and(|p| p.is_form())
        } else {
            false
        }
    })
}
