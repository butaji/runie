use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg, OnboardingStep};

// --- Key classification helpers ---

fn is_ctrl_combo(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::SHIFT)
}

pub fn event_to_msg(event: Event, state: &AppState) -> Vec<Msg> {
    match event {
        Event::Key(key) => key_to_msg(key, state).map_or_else(Vec::new, |m| vec![m]),
        // BUG-03 FIX: Check mode before emitting Paste — block in Permission/Overlay
        Event::Paste(text) => {
            if matches!(state.mode, TuiMode::Permission | TuiMode::Overlay) {
                vec![]
            } else {
                vec![Msg::Paste(text)]
            }
        }
        Event::Resize(w, h) => vec![Msg::Resize(w, h)],
        _ => Vec::new(),
    }
}

pub fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // P0-3/P0-4 FIX: Blocking modes intercept ALL keys (no global hotkeys)
    if let Some(blocking_result) = blocking_mode_handler(&key, &state.mode, state) {
        // blocking_result is Option<Msg>:
        // Some(msg) = blocking mode handled the key
        // None = blocking mode exists but didn't handle this key
        if let Some(msg) = blocking_result {
            return Some(msg);
        }
        // Blocking mode exists but didn't handle -> return None
        return None;
    }
    
    // Global hotkeys: active in all non-blocking modes
    if let Some(global_result) = global_hotkey_handler(&key, state) {
        // Some(msg) = global handler handled the key
        // None = no global hotkey matched
        if let Some(msg) = global_result {
            return Some(msg);
        }
        // No global hotkey matched -> continue to mode-specific
        return None;
    }

    // Route to mode-specific handler (non-blocking modes only)
    route_non_blocking_mode(&key, state)
}

/// Handles keys in blocking modes (Permission, Overlay).
/// These intercept ALL keys, preventing accidental Ctrl+ shortcuts from quitting the app.
fn blocking_mode_handler(key: &crossterm::event::KeyEvent, mode: &TuiMode, state: &AppState) -> Option<Option<Msg>> {
    match mode {
        TuiMode::Permission => Some(key_to_permission_msg(*key)),
        TuiMode::Overlay => Some(key_to_overlay_msg(*key, state)),
        _ => None,
    }
}

/// Handles global hotkeys (Ctrl+C, Ctrl+Q) in non-blocking modes.
fn global_hotkey_handler(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Option<Msg>> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // DiffViewer intercepts Ctrl+Q to close the viewer — the global quit
    // handler must not fire here (test_ctrl_q_quits_in_diff_viewer).
    if matches!(state.mode, TuiMode::DiffViewer) && matches!(key.code, KeyCode::Char('q')) {
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
        KeyCode::Char('m') => Some(Some(Msg::SwitchModel)),
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
        _ => {
            tracing::warn!("Unhandled TuiMode in route_non_blocking_mode");
            None
        }
    }
}

fn key_to_overlay_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // P0-4 FIX: Esc closes overlay; Ctrl+Q also closes
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q')) {
        return Some(Msg::CloseModal);
    }
    // Plain Ctrl + letter combos (other than Ctrl+Q above) are NOT overlay
    // navigation — block them so the global hotkey handler can't fire
    // through the overlay (test_no_global_hotkeys_in_overlay).
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // Route to model picker specific messages when model_picker is active
    if state.model_picker.is_some() {
        return key_to_model_picker_msg(key);
    }
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
        _ => None,
    }
}

fn key_to_model_picker_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
        KeyCode::Char('d') => Some(Msg::SelectToggleDetails),
        _ => None,
    }
}

fn key_to_chat_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // Handle Ctrl+Shift+E before the normal ctrl combo check
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('e')) {
        return Some(Msg::ToggleThoughts);
    }
    if is_ctrl_combo(key) {
        return ctrl_chat_key(key);
    }
    if matches!(key.code, KeyCode::Enter) {
        return if key.modifiers.contains(KeyModifiers::SHIFT) { Some(Msg::InsertNewline) } else { Some(Msg::Submit) };
    }
    // Simple navigation
    if matches!(key.code, KeyCode::Up) { return Some(Msg::HistoryUp); }
    if matches!(key.code, KeyCode::Down) { return Some(Msg::HistoryDown); }
    if matches!(key.code, KeyCode::PageUp) { return Some(Msg::ScrollPageUp); }
    if matches!(key.code, KeyCode::PageDown) { return Some(Msg::ScrollPageDown); }
    Some(Msg::TextareaKey(key))
}

fn ctrl_chat_key(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Enter => Some(Msg::InsertNewline),
        KeyCode::Char('k') | KeyCode::Char('n') | KeyCode::Char('p') | KeyCode::Char('s') => Some(Msg::OpenCommandPalette),
        KeyCode::Char('b') => Some(Msg::ToggleSidebar),
        KeyCode::Char('o') => Some(Msg::CopyLastResponse),
        KeyCode::Char('l') => Some(Msg::ClearChat),
        KeyCode::Char('q') => Some(Msg::Quit),
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Msg::ToggleThoughts),
        _ => Some(Msg::TextareaKey(key)),
    }
}

fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // Permission modal intercepts ALL keys — blocking mode
    // P0-3 FIX: Ctrl+C/Ctrl+Q cancel permission
    if is_ctrl_combo(key) && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
        return Some(Msg::PermissionCancel);
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
    // P0-4 FIX: Ctrl+C/Ctrl+Q close modal
    if is_ctrl_combo(key) && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
        return Some(Msg::CloseModal);
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('x') => Some(Msg::CloseModal),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => Some(Msg::ScrollDown),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => Some(Msg::ScrollUp),
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

fn key_to_onboarding_navigation(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Up) { return Some(Msg::OnboardingNavigateUp); }
    if matches!(key.code, KeyCode::Down) { return Some(Msg::OnboardingNavigateDown); }
    if matches!(key.code, KeyCode::Enter) { return Some(Msg::OnboardingNext); }
    None
}

fn key_to_onboarding_esc(is_welcome: bool) -> Option<Msg> {
    Some(if is_welcome { Msg::OnboardingSkip } else { Msg::OnboardingBack })
}

fn key_to_onboarding_space(state: &AppState) -> Option<Msg> {
    let step = state.onboarding.as_ref().map(|o| o.step.clone());
    if matches!(step, Some(OnboardingStep::ModelSelect)) {
        let idx = state.onboarding.as_ref().map(|o| o.selected_item).unwrap_or(0);
        return Some(Msg::OnboardingSelectModel(idx));
    }
    None
}

fn key_to_onboarding_char(key: crossterm::event::KeyEvent, is_picker_step: bool) -> Option<Msg> {
    if let KeyCode::Char(c) = key.code {
        return Some(if is_picker_step { Msg::OnboardingSearchInput(c) } else { Msg::OnboardingKeyInput(c) });
    }
    None
}

fn key_to_onboarding_backspace(is_picker_step: bool) -> Option<Msg> {
    Some(if is_picker_step { Msg::OnboardingSearchBackspace } else { Msg::OnboardingKeyBackspace })
}

fn key_to_onboarding_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    let step = state.onboarding.as_ref().map(|o| o.step.clone());
    let is_welcome = matches!(step, Some(OnboardingStep::Welcome));
    let is_picker_step = matches!(step, Some(OnboardingStep::ProviderSelect | OnboardingStep::ModelSelect));

    if matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Enter) {
        return key_to_onboarding_navigation(key);
    }
    if matches!(key.code, KeyCode::Esc) {
        return key_to_onboarding_esc(is_welcome);
    }
    if matches!(key.code, KeyCode::Char(' ')) {
        return key_to_onboarding_space(state);
    }
    if let Some(c) = key_to_onboarding_char(key, is_picker_step) {
        return Some(c);
    }
    if matches!(key.code, KeyCode::Backspace | KeyCode::Delete) {
        return key_to_onboarding_backspace(is_picker_step);
    }
    None
}
