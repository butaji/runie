//! Key → Action mapping.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Submit(String),
    Abort,
    Quit,
    ClearPrompt,
    FocusPrompt,
    /// Cycle the input mode (grok Shift+Tab).
    ModeCycle,
    /// Open the shortcut help (grok Ctrl+x).
    OpenShortcuts,
    OpenCommandPalette,
    /// Enter the file-search prompt (grok Ctrl+l).
    OpenFileSearch,
    /// Toggle the selected scrollback fold (Grok's scrollback `e` action).
    ToggleFold,
    SelectNextTool,
    SelectPreviousTool,
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
#[allow(
    clippy::cognitive_complexity,
    reason = "the key map keeps the declarative normal-mode bindings together"
)]
pub fn map_key(key: KeyEvent, prompt_non_empty: bool, streaming: bool) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => ctrl_c_action(prompt_non_empty, streaming),
            KeyCode::Char('d' | 'q') => Action::Quit,
            // Reserved for Grok's file-search line viewer. The minimal TUI
            // has no file-search target yet, so do not repurpose it as a
            // destructive scrollback action.
            KeyCode::Char('l') => Action::OpenFileSearch,
            KeyCode::Char('x') => Action::OpenShortcuts,
            KeyCode::Char('p') => Action::OpenCommandPalette,
            _ => Action::Noop,
        };
    }
    match key.code {
        KeyCode::Up if !prompt_non_empty => Action::SelectPreviousTool,
        KeyCode::Down if !prompt_non_empty => Action::SelectNextTool,
        KeyCode::Char('e') if !prompt_non_empty => Action::ToggleFold,
        KeyCode::Char('?') if !prompt_non_empty => Action::OpenCommandPalette,
        KeyCode::Enter if key.modifiers == KeyModifiers::NONE => {
            if prompt_non_empty {
                Action::FocusPrompt
            } else {
                Action::Noop
            }
        }
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Action::ModeCycle,
        KeyCode::Esc => {
            if prompt_non_empty {
                Action::ClearPrompt
            } else {
                Action::Noop
            }
        }
        _ => Action::Noop,
    }
}

fn ctrl_c_action(prompt_non_empty: bool, streaming: bool) -> Action {
    match (prompt_non_empty, streaming) {
        (true, _) => Action::ClearPrompt,
        (false, true) => Action::Abort,
        (false, false) => Action::Quit,
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
    fn ctrl_c_quits_when_idle_and_prompt_empty() {
        let a = map_key(k(KeyCode::Char('c'), KeyModifiers::CONTROL), false, false);
        assert_eq!(a, Action::Quit);
    }

    #[test]
    fn ctrl_c_clears_prompt_before_abort_or_quit() {
        assert_eq!(
            map_key(k(KeyCode::Char('c'), KeyModifiers::CONTROL), true, true),
            Action::ClearPrompt
        );
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
    fn ctrl_l_is_reserved_for_file_search() {
        assert_eq!(
            map_key(k(KeyCode::Char('l'), KeyModifiers::CONTROL), false, false),
            Action::OpenFileSearch
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
    fn shift_tab_cycles_input_mode() {
        assert_eq!(
            map_key(k(KeyCode::Tab, KeyModifiers::SHIFT), false, false),
            Action::ModeCycle
        );
    }

    #[test]
    fn ctrl_x_opens_shortcuts() {
        assert_eq!(
            map_key(k(KeyCode::Char('x'), KeyModifiers::CONTROL), false, false),
            Action::OpenShortcuts
        );
    }

    #[test]
    fn ctrl_p_and_question_open_command_palette() {
        assert_eq!(
            map_key(k(KeyCode::Char('p'), KeyModifiers::CONTROL), false, false),
            Action::OpenCommandPalette
        );
        assert_eq!(
            map_key(k(KeyCode::Char('?'), KeyModifiers::NONE), false, false),
            Action::OpenCommandPalette
        );
    }

    #[test]
    fn e_toggles_scrollback_fold_when_prompt_is_empty() {
        assert_eq!(
            map_key(k(KeyCode::Char('e'), KeyModifiers::NONE), false, false),
            Action::ToggleFold
        );
        assert_eq!(
            map_key(k(KeyCode::Char('e'), KeyModifiers::NONE), true, false),
            Action::Noop
        );
    }

    #[test]
    fn arrows_select_tools_only_when_prompt_is_empty() {
        assert_eq!(
            map_key(k(KeyCode::Up, KeyModifiers::NONE), false, false),
            Action::SelectPreviousTool
        );
        assert_eq!(
            map_key(k(KeyCode::Down, KeyModifiers::NONE), false, false),
            Action::SelectNextTool
        );
        assert_eq!(
            map_key(k(KeyCode::Up, KeyModifiers::NONE), true, false),
            Action::Noop
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
