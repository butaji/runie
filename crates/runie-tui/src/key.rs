//! Key → Action mapping.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Submit(String),
    Abort,
    Quit,
    ClearScrollback,
    ClearPrompt,
    FocusPrompt,
    Noop,
}

/// Whether a submitted prompt is an immediate quit command.
pub fn is_quit_command(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "exit" | "quit" | ":q"
    )
}

/// Map a key event to actions.
///
/// `prompt_non_empty` reflects whether the user has typed anything. `streaming`
/// tells us whether the loop is currently processing.
pub fn map_key(key: KeyEvent, prompt_non_empty: bool, streaming: bool) -> Action {
    match (key.code, key.modifiers) {
        (KeyCode::Enter, m) if m == KeyModifiers::NONE => {
            if prompt_non_empty {
                // Submit is handled in App via PromptOutcome; this branch is
                // reserved for key-only submit (we don't carry the text here).
                Action::FocusPrompt
            } else {
                Action::Noop
            }
        }
        (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
            if streaming {
                Action::Abort
            } else {
                Action::Quit
            }
        }
        (KeyCode::Char('d'), m) if m.contains(KeyModifiers::CONTROL) => Action::Quit,
        // Ctrl+Q is the unconditional quit chord in full mode. It must not
        // depend on prompt contents or whether a turn is streaming.
        (KeyCode::Char('q'), m) if m.contains(KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Char('l'), m) if m.contains(KeyModifiers::CONTROL) => Action::ClearScrollback,
        (KeyCode::Esc, _) => {
            if prompt_non_empty {
                Action::ClearPrompt
            } else {
                Action::Noop
            }
        }
        _ => Action::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn k(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn ctrl_c_aborts_when_streaming() {
        let a = map_key(k(KeyCode::Char('c'), KeyModifiers::CONTROL), false, true);
        assert_eq!(a, Action::Abort);
    }

    #[test]
    fn ctrl_c_quits_when_idle() {
        let a = map_key(k(KeyCode::Char('c'), KeyModifiers::CONTROL), false, false);
        assert_eq!(a, Action::Quit);
    }

    #[test]
    fn ctrl_d_quits() {
        assert_eq!(
            map_key(k(KeyCode::Char('d'), KeyModifiers::CONTROL), false, false),
            Action::Quit
        );
    }

    #[test]
    fn ctrl_q_quits_when_idle() {
        assert_eq!(
            map_key(k(KeyCode::Char('q'), KeyModifiers::CONTROL), false, false),
            Action::Quit
        );
    }

    #[test]
    fn ctrl_q_quits_immediately_while_streaming() {
        assert_eq!(
            map_key(k(KeyCode::Char('q'), KeyModifiers::CONTROL), true, true),
            Action::Quit
        );
    }

    #[test]
    fn quit_commands_are_case_insensitive_and_trimmed() {
        for command in ["exit", " quit ", ":q", "EXIT", "QuIt"] {
            assert!(
                is_quit_command(command),
                "expected quit command: {command:?}"
            );
        }
    }

    #[test]
    fn ordinary_prompt_is_not_a_quit_command() {
        assert!(!is_quit_command("quit please"));
        assert!(!is_quit_command("/quit"));
        assert!(!is_quit_command(""));
    }

    #[test]
    fn ctrl_l_clears_scrollback() {
        assert_eq!(
            map_key(k(KeyCode::Char('l'), KeyModifiers::CONTROL), false, false),
            Action::ClearScrollback
        );
    }

    #[test]
    fn esc_with_text_clears_prompt() {
        assert_eq!(
            map_key(k(KeyCode::Esc, KeyModifiers::NONE), true, false),
            Action::ClearPrompt
        );
    }

    #[test]
    fn esc_without_text_is_noop() {
        assert_eq!(
            map_key(k(KeyCode::Esc, KeyModifiers::NONE), false, false),
            Action::Noop
        );
    }
}
