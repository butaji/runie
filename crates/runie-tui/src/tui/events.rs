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
    // P0-3/P0-4 FIX: Blocking modes intercept ALL keys (no global hotkeys)
    if let Some(msg) = blocking_mode_handler(&key, &state.mode) {
        return msg;
    }
    
    // Global hotkeys: active in all non-blocking modes
    if let Some(msg) = global_hotkey_handler(&key, state) {
        return msg;
    }

    // Route to mode-specific handler (non-blocking modes only)
    route_non_blocking_mode(&key, state)
}

/// Handles keys in blocking modes (Permission, Overlay).
/// These intercept ALL keys, preventing accidental Ctrl+ shortcuts from quitting the app.
fn blocking_mode_handler(key: &crossterm::event::KeyEvent, mode: &TuiMode) -> Option<Option<Msg>> {
    match mode {
        TuiMode::Permission => Some(key_to_permission_msg(*key)),
        TuiMode::Overlay => Some(key_to_overlay_msg(*key)),
        _ => None,
    }
}

/// Handles global hotkeys (Ctrl+C, Ctrl+Q) in non-blocking modes.
fn global_hotkey_handler(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Option<Msg>> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('c') => {
            let is_empty = state.textarea.lines() == [""];
            if is_empty {
                // Empty textarea: quit immediately
                Some(Some(Msg::Quit))
            } else {
                // Has text: require double-tap Ctrl+C to clear (P1-REMAINING-1 FIX)
                // The actual check happens in update() via clear_input_confirm
                Some(Some(Msg::ClearInputConfirm)) // Signal that user wants to clear
            }
        }
        KeyCode::Char('q') => Some(Some(Msg::Quit)),
        _ => None,
    }
}

/// Routes key to the appropriate mode-specific handler (non-blocking modes only).
fn route_non_blocking_mode(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    match state.mode {
        TuiMode::Chat | TuiMode::Select => key_to_chat_msg(*key),
        TuiMode::CommandPalette => key_to_palette_msg(*key),
        TuiMode::DiffViewer => key_to_diff_msg(*key),
        TuiMode::SessionTree => key_to_tree_msg(*key),
        TuiMode::Onboarding => key_to_onboarding_msg(*key, state),
        // Permission and Overlay handled by blocking_mode_handler above
        #[allow(unreachable_patterns)]
        _ => unreachable!(),
    }
}

fn key_to_overlay_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // P0-4 FIX: Esc closes overlay; Ctrl+Q also closes
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q')) {
        return Some(Msg::CloseModal);
    }
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
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
        // P1-3 FIX: ? key for help (opens help overlay if available)
        KeyCode::Char('?') => Some(Msg::OpenCommandPalette), // Temporary: open palette which lists commands
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
    // Permission modal intercepts ALL keys — blocking mode
    // P0-3 FIX: Ctrl+C and ^q in Permission mode both cancel permission
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::PermissionCancel),
            _ => {}
        }
    }
    
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') => Some(Msg::PermissionConfirm),
        KeyCode::Esc | KeyCode::Char('n') => Some(Msg::PermissionCancel),
        KeyCode::Char('a') => Some(Msg::PermissionAlways),
        KeyCode::Char('s') => Some(Msg::PermissionSkip),
        _ => None,
    }
}

fn key_to_palette_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // P1-1 FIX: Esc cancels argument mode if active, otherwise closes palette
    if matches!(key.code, KeyCode::Esc) {
        // The actual check for is_argument_mode happens in the command_palette module
        // Here we send CancelArgument which will be handled appropriately
        return Some(Msg::CommandPaletteCancelArgument);
    }
    match key.code {
        KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
        KeyCode::Up => Some(Msg::CommandPaletteUp),
        KeyCode::Down => Some(Msg::CommandPaletteDown),
        KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
        KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
        _ => None,
    }
}

fn key_to_diff_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // P0-4 FIX: Accept more close triggers for accessibility
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::CloseModal),
            _ => {}
        }
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('x') => Some(Msg::CloseModal),
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
