use crossterm::event::{Event, KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg, OnboardingStep};

pub fn event_to_msg(event: Event, _state: &AppState) -> Vec<Msg> {
    match event {
        Event::Key(key) => key_to_msg(key, _state).map_or_else(Vec::new, |m| vec![m]),
        Event::Paste(text) => vec![Msg::Paste(text)],
        Event::Resize(w, h) => vec![Msg::Resize(w, h)],
        _ => Vec::new(),
    }
}

pub fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // Global hotkeys: always active regardless of mode
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => {
                // If textarea is empty (only has one empty line), quit; otherwise clear input
                let is_empty = state.textarea.lines() == [""];
                if is_empty {
                    return Some(Msg::Quit);
                } else {
                    return Some(Msg::ClearInput);
                }
            }
            KeyCode::Char('q') => return Some(Msg::Quit),
            _ => {}
        }
    }

    match state.mode {
        TuiMode::Chat => key_to_chat_msg(key),
        TuiMode::Permission => key_to_permission_msg(key),
        TuiMode::CommandPalette => key_to_palette_msg(key),
        TuiMode::DiffViewer => key_to_diff_msg(key),
        TuiMode::SessionTree => key_to_tree_msg(key),
        TuiMode::Onboarding => key_to_onboarding_msg(key, state),
        _ => None,
    }
}

fn key_to_chat_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return ctrl_chat_key(key);
    }
    match key.code {
        KeyCode::Enter => {
            // Shift+Enter → newline, plain Enter → submit
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(Msg::InsertNewline)
            } else {
                Some(Msg::Submit)
            }
        }
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        _ => Some(Msg::TextareaKey(key)),
    }
}

fn ctrl_chat_key(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('j') => Some(Msg::InsertNewline), // Ctrl+J = insert newline
        KeyCode::Char('k') | KeyCode::Char('p') => Some(Msg::OpenCommandPalette),
        KeyCode::Char('b') => Some(Msg::ToggleSidebar),
        KeyCode::Char('n') => Some(Msg::OpenCommandPalette), // Ctrl+N = new session via palette
        KeyCode::Char('o') => Some(Msg::OpenCommandPalette), // Ctrl+O = load session via palette
        KeyCode::Char('s') => Some(Msg::OpenCommandPalette), // Ctrl+S = save session via palette
        KeyCode::Char('l') => Some(Msg::ClearChat),
        KeyCode::Char('q') => Some(Msg::Quit),
        KeyCode::Enter => Some(Msg::InsertNewline), // Ctrl+Enter = insert newline
        _ => Some(Msg::TextareaKey(key)), // Let textarea handle Ctrl+A/E/D/W/U/etc.
    }
}

fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') => Some(Msg::PermissionConfirm),
        KeyCode::Esc | KeyCode::Char('n') => Some(Msg::PermissionCancel),
        KeyCode::Char('a') => Some(Msg::PermissionAlways),
        KeyCode::Char('s') => Some(Msg::PermissionSkip),
        _ => None,
    }
}

fn key_to_palette_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
        KeyCode::Up => Some(Msg::CommandPaletteUp),
        KeyCode::Down => Some(Msg::CommandPaletteDown),
        KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
        KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
        _ => None,
    }
}

fn key_to_diff_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseModal),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::ScrollDown),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::ScrollUp),
        KeyCode::PageDown => Some(Msg::ScrollDown),
        KeyCode::PageUp => Some(Msg::ScrollUp),
        _ => None,
    }
}

fn key_to_tree_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SessionTreeUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SessionTreeDown),
        KeyCode::Enter => Some(Msg::SessionTreeConfirm),
        _ => None,
    }
}

fn key_to_onboarding_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    let is_picker_step = state
        .onboarding
        .as_ref()
        .map(|o| matches!(o.step, OnboardingStep::ProviderSelect | OnboardingStep::ModelSelect))
        .unwrap_or(false);

    match key.code {
        KeyCode::Enter => Some(Msg::OnboardingNext),
        KeyCode::Esc => Some(Msg::OnboardingBack),
        KeyCode::Up => Some(Msg::OnboardingNavigateUp),
        KeyCode::Down => Some(Msg::OnboardingNavigateDown),
        KeyCode::Char(c) => {
            if is_picker_step {
                Some(Msg::OnboardingSearchInput(c))
            } else {
                Some(Msg::OnboardingKeyInput(c))
            }
        }
        KeyCode::Backspace | KeyCode::Delete => {
            if is_picker_step {
                Some(Msg::OnboardingSearchBackspace)
            } else {
                Some(Msg::OnboardingKeyBackspace)
            }
        }
        _ => None,
    }
}
