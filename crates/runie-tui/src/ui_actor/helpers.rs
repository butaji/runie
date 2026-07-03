//! Helper functions for UiActor.

use runie_core::Event;

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

/// Returns `true` for input/navigation events that should be applied to
/// `AppState` when any dialog is open, instead of being routed to the main
/// chat `InputActor`.
///
/// Without this guard the canonical router sends everything to `InputActor`,
/// which only mutates the chat input box. Modal forms, palettes, and the
/// onboarding login flow need these events on state so their fields receive
/// typing, backspace, paste, and arrow navigation.
pub fn is_dialog_input_event(evt: &Event) -> bool {
    matches!(
        evt,
        Event::Input(_)
            | Event::Backspace
            | Event::Paste(_)
            | Event::CursorLeft
            | Event::CursorRight
            | Event::HistoryPrev
            | Event::HistoryNext
            | Event::Submit
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Layer 1: dialog input events are the typing/navigation keys that modal
    /// forms and palettes need when a dialog is open.
    #[test]
    fn dialog_input_event_matches_typing_and_arrows() {
        assert!(is_dialog_input_event(&Event::Input('x')));
        assert!(is_dialog_input_event(&Event::Backspace));
        assert!(is_dialog_input_event(&Event::Paste("abc".into())));
        assert!(is_dialog_input_event(&Event::CursorLeft));
        assert!(is_dialog_input_event(&Event::CursorRight));
        assert!(is_dialog_input_event(&Event::HistoryPrev));
        assert!(is_dialog_input_event(&Event::HistoryNext));
    }

    /// Layer 1: non-dialog input events keep routing to InputActor.
    #[test]
    fn dialog_input_event_excludes_non_dialog_keys() {
        assert!(!is_dialog_input_event(&Event::Newline));
        assert!(!is_dialog_input_event(&Event::InputChanged {
            state: Box::default()
        }));
        assert!(!is_dialog_input_event(&Event::CursorStart));
        assert!(!is_dialog_input_event(&Event::Undo));
    }

    /// Layer 1: Submit is a dialog input event so Enter activates the selected
    /// panel item (provider picker, command palette) and submits modal forms.
    #[test]
    fn dialog_input_event_includes_submit() {
        assert!(is_dialog_input_event(&Event::Submit));
    }
}
