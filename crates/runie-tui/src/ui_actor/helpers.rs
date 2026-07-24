//! Helper functions for UiActor.

use runie_core::Event;

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
            | Event::CycleThinkingLevel
            | Event::Submit
            | Event::Newline
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
        assert!(!is_dialog_input_event(&Event::InputChanged {
            state: Box::default()
        }));
        assert!(!is_dialog_input_event(&Event::CursorStart));
        assert!(!is_dialog_input_event(&Event::Undo));
    }

    #[test]
    fn dialog_input_event_includes_shift_tab_navigation() {
        assert!(is_dialog_input_event(&Event::CycleThinkingLevel));
    }

    /// Layer 1: Submit is a dialog input event so Enter activates the selected
    /// panel item (provider picker, command palette) and submits modal forms.
    #[test]
    fn dialog_input_event_includes_submit() {
        assert!(is_dialog_input_event(&Event::Submit));
    }
}
