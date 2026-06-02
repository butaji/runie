use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
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
        Event::Mouse(mouse_event) => {
            match mouse_event.kind {
                MouseEventKind::ScrollUp => vec![Msg::ScrollUp],
                MouseEventKind::ScrollDown => vec![Msg::ScrollDown],
                MouseEventKind::Down(button) => vec![Msg::MouseClick { x: mouse_event.column, y: mouse_event.row, button: 0 }],
                _ => vec![],
            }
        }
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
        TuiMode::HomeScreen => Some(key_to_home_screen_msg(*key)),
        TuiMode::Plan => Some(key_to_plan_modal_msg(*key, state)),
        _ => None,
    }
}

/// Handles global hotkeys (Ctrl+C, Ctrl+Q) in non-blocking modes.
fn global_hotkey_handler(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Option<Msg>> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // Ctrl+Shift+Q toggles questionnaire panel
    if key.modifiers.contains(KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('q')) {
        return Some(Some(Msg::ToggleQuestionnaire));
    }
    // DiffViewer intercepts Ctrl+Q to close the viewer — the global quit
    // handler must not fire here (test_ctrl_q_quits_in_diff_viewer).
    if matches!(state.mode, TuiMode::DiffViewer) && matches!(key.code, KeyCode::Char('q')) {
        return None;
    }
    match key.code {
        KeyCode::Char('c') => {
            if state.agent_running {
                Some(Some(Msg::Stop))
            } else {
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
        }
        KeyCode::Char('q') => Some(Some(Msg::Quit)),
        KeyCode::Char('d') => Some(Some(Msg::Quit)),
        KeyCode::Char('m') => Some(Some(Msg::SwitchModel)),
        KeyCode::Char('h') => Some(Some(Msg::GoHome)),
        _ => None,
    }
}

/// Routes key to the appropriate mode-specific handler (non-blocking modes only).
fn check_modal_precedence(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if matches!(state.mode, TuiMode::Chat) && state.slash_menu.is_open() {
        return key_to_slash_menu_msg(*key);
    }
    if state.shortcuts_panel.is_open() {
        return key_to_shortcuts_panel_msg(*key, state);
    }
    if state.settings_modal.is_open() {
        return key_to_settings_modal_msg(*key);
    }
    if state.file_picker.is_open() {
        return key_to_file_picker_msg(*key);
    }
    if !state.history_search_matches.is_empty() {
        return key_to_history_search_msg(*key);
    }
    if state.context_usage_modal.is_open() {
        return key_to_context_usage_msg(*key);
    }
    None
}

fn route_non_blocking_mode(key: &crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if let Some(msg) = check_modal_precedence(key, state) {
        return Some(msg);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Enter) && state.agent_running {
        return Some(Msg::Interject);
    }
    match state.mode {
        TuiMode::Chat | TuiMode::Select => key_to_chat_msg(*key, state),
        TuiMode::CommandPalette => key_to_palette_msg(*key),
        TuiMode::DiffViewer => key_to_diff_msg(*key),
        TuiMode::SessionTree => key_to_tree_msg(*key),
        TuiMode::Onboarding => key_to_onboarding_msg(*key, state),
        TuiMode::Questionnaire => key_to_questionnaire_msg(*key),
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
    // Route to extensions modal when active
    if state.extensions_modal.is_some() {
        return key_to_extensions_modal_msg(key);
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

fn key_to_extensions_modal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::ExtensionsModalUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::ExtensionsModalDown),
        KeyCode::Enter => Some(Msg::ExtensionsModalSelect),
        KeyCode::Left | KeyCode::Char('h') => Some(Msg::ExtensionsModalLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Msg::ExtensionsModalRight),
        KeyCode::Backspace => Some(Msg::ExtensionsModalSearchBackspace),
        KeyCode::Char(c) => Some(Msg::ExtensionsModalSearchInput(c)),
        _ => None,
    }
}

fn key_to_slash_menu_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseSlashMenu),
        KeyCode::Up => Some(Msg::SlashMenuUp),
        KeyCode::Down => Some(Msg::SlashMenuDown),
        KeyCode::Enter => Some(Msg::SlashMenuConfirm),
        _ => Some(Msg::TextareaKey(key)),
    }
}

fn shortcuts_panel_filter_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseShortcutsPanel),
        KeyCode::Backspace => Some(Msg::ShortcutsPanelFilterBackspace),
        KeyCode::Char(c) => Some(Msg::ShortcutsPanelFilterInput(c)),
        KeyCode::Up => Some(Msg::ShortcutsPanelUp),
        KeyCode::Down => Some(Msg::ShortcutsPanelDown),
        KeyCode::Enter => Some(Msg::ShortcutsPanelToggleSection),
        _ => None,
    }
}

fn shortcuts_panel_normal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseShortcutsPanel),
        KeyCode::Char('f') | KeyCode::Char('/') => Some(Msg::ShortcutsPanelToggleFilter),
        KeyCode::Char('e') | KeyCode::Enter | KeyCode::Char(' ') => Some(Msg::ShortcutsPanelToggleSection),
        KeyCode::Up => Some(Msg::ShortcutsPanelUp),
        KeyCode::Down => Some(Msg::ShortcutsPanelDown),
        _ => None,
    }
}

fn key_to_shortcuts_panel_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if state.shortcuts_panel.filter_mode {
        shortcuts_panel_filter_msg(key)
    } else {
        shortcuts_panel_normal_msg(key)
    }
}

fn key_to_home_screen_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // Handle Ctrl+ shortcuts first
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => return Some(Msg::CloseHomeScreen), // Resume session
            KeyCode::Char('w') => return Some(Msg::ToggleWorktreeMode),
            KeyCode::Char('i') => return Some(Msg::ImportClaudeSettings),
            _ => {}
        }
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::Quit),
        KeyCode::Up => Some(Msg::HomeScreenUp),
        KeyCode::Down => Some(Msg::HomeScreenDown),
        KeyCode::Enter => Some(Msg::HomeScreenSelect),
        KeyCode::Char('n') => Some(Msg::CloseHomeScreen),
        KeyCode::Char('r') => Some(Msg::CloseHomeScreen),
        KeyCode::Char('s') => Some(Msg::OpenSettingsModal),
        KeyCode::Char('h') => Some(Msg::OpenShortcutsPanel),
        _ => None,
    }
}

fn key_to_settings_modal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseSettingsModal),
        KeyCode::Tab => Some(Msg::SettingsModalNextTab),
        KeyCode::BackTab => Some(Msg::SettingsModalPrevTab),
        KeyCode::Up => Some(Msg::SettingsModalUp),
        KeyCode::Down => Some(Msg::SettingsModalDown),
        KeyCode::Enter => Some(Msg::SettingsModalSelect),
        _ => None,
    }
}

fn chat_navigation_msg(key: crossterm::event::KeyEvent, scroll_focused: bool) -> Option<Msg> {
    if scroll_focused {
        // Vim-style scroll keys + arrow navigation when feed has focus
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(Msg::ScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Msg::ScrollUp),
            KeyCode::Char('g') => Some(Msg::ScrollToTop), // gg - handled as 'g' first press
            KeyCode::Char('G') => Some(Msg::ScrollToBottom),
            KeyCode::Char('H') => Some(Msg::ScrollToPrevUserTurn),
            KeyCode::Char('L') => Some(Msg::ScrollToNextUserTurn),
            KeyCode::Left | KeyCode::Char('h') => Some(Msg::CollapseEntry),
            KeyCode::Right | KeyCode::Char('l') => Some(Msg::ExpandEntry),
            KeyCode::Char('e') => Some(Msg::ToggleFoldEntry),
            KeyCode::Char('E') => Some(Msg::ToggleAllEntries),
            KeyCode::Char('y') => Some(Msg::CopyBlockContent),
            KeyCode::Char('Y') => Some(Msg::CopyBlockMetadata),
            KeyCode::Char('r') => Some(Msg::ToggleRawMarkdown),
            KeyCode::Char(' ') => Some(Msg::FocusPrompt),
            KeyCode::Char('i') => Some(Msg::FocusPrompt),
            KeyCode::PageUp => Some(Msg::ScrollPageUp),
            KeyCode::PageDown => Some(Msg::ScrollPageDown),
            // Shift + Left/Right for scrollback navigation
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Msg::ScrollToPrevUserTurn),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Msg::ScrollToNextUserTurn),
            // o/O for open entry
            KeyCode::Char('o') => Some(Msg::OpenEntry),
            KeyCode::Char('O') => Some(Msg::OpenEntryOptions),
            _ => None,
        }
    } else {
        match key.code {
            KeyCode::Up => Some(Msg::HistoryUp),
            KeyCode::Down => Some(Msg::HistoryDown),
            KeyCode::PageUp => Some(Msg::ScrollPageUp),
            KeyCode::PageDown => Some(Msg::ScrollPageDown),
            _ => None,
        }
    }
}

fn key_to_chat_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('e')) {
        return Some(Msg::ToggleThoughts);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('a')) {
        return Some(Msg::ToggleSubagentPanel);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Char('n')) {
        return Some(Msg::NewSessionWorktree);
    }
    if is_ctrl_combo(key) {
        return ctrl_chat_key(key);
    }
    if matches!(key.code, KeyCode::Enter) {
        return if key.modifiers.contains(KeyModifiers::SHIFT) { Some(Msg::InsertNewline) } else { Some(Msg::Submit) };
    }
    if matches!(key.code, KeyCode::Esc) {
        return Some(Msg::ToggleScrollFocus);
    }
    // Shift+Tab cycles session modes
    if key.modifiers.contains(KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Tab) {
        return Some(Msg::TogglePermissionMode);
    }
    if matches!(key.code, KeyCode::Tab) {
        return Some(Msg::ToggleScrollFocus);
    }
    // i and Space focus the prompt (vim mode / scrollback)
    if matches!(key.code, KeyCode::Char('i') | KeyCode::Char(' ')) {
        return Some(Msg::FocusPrompt);
    }
    if let Some(msg) = chat_navigation_msg(key, state.scroll.scroll_focused) {
        return Some(msg);
    }
    if matches!(key.code, KeyCode::Char('?')) { return Some(Msg::ShowHelp); }
    Some(Msg::TextareaKey(key))
}

fn ctrl_chat_key(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Enter) {
        return Some(Msg::Interject);
    }
    ctrl_chat_key_match(key)
}

fn ctrl_chat_key_match(key: crossterm::event::KeyEvent) -> Option<Msg> {
    let c = match key.code {
        KeyCode::Char(c) => c,
        KeyCode::Enter => return Some(Msg::Interject),
        _ => return Some(Msg::TextareaKey(key)),
    };
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        if c == 'a' { return Some(Msg::ClearAlwaysApprove); }
        if c == 'e' { return Some(Msg::ToggleThoughts); }
        return Some(Msg::TextareaKey(key));
    }
    const CTRL_MAP: &[(char, Msg)] = &[
        ('k', Msg::ScrollUp),
        ('j', Msg::ScrollDown),
        ('n', Msg::OpenCommandPalette),
        ('p', Msg::OpenCommandPalette),
        ('s', Msg::ToggleSessionTree),
        ('.', Msg::OpenShortcutsPanel),
        (',', Msg::OpenSettingsModal),
        ('b', Msg::ToggleSidebar),
        ('o', Msg::TogglePermissionMode),
        ('l', Msg::ClearChat),
        ('r', Msg::HistorySearchStart),
        ('u', Msg::ScrollHalfPageUp),
        ('d', Msg::ScrollHalfPageDown),
        ('a', Msg::TogglePermissionMode),
        ('q', Msg::Quit),
        (';', Msg::TogglePromptQueue),
    ];
    for &(ch, ref msg) in CTRL_MAP {
        if c == ch { return Some(msg.clone()); }
    }
    Some(Msg::TextareaKey(key))
}

fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    // Permission modal intercepts ALL keys — blocking mode
    // P0-3 FIX: Ctrl+C/Ctrl+Q cancel permission
    if is_ctrl_combo(key) && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
        return Some(Msg::PermissionCancel);
    }
    match key.code {
        // BUG-13 FIX: Handle uppercase Y for tmux compatibility
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => Some(Msg::PermissionConfirm),
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => Some(Msg::PermissionCancel),
        // BUG-13 FIX: Handle uppercase A for tmux compatibility
        KeyCode::Char('a') | KeyCode::Char('A') => Some(Msg::PermissionAlways),
        KeyCode::Char('s') => Some(Msg::PermissionSkip),
        _ => None,
    }
}

fn key_to_plan_modal_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    // Esc closes the plan modal without applying
    if matches!(key.code, KeyCode::Esc) {
        return Some(Msg::CloseModal);
    }
    // Enter or y/Y approves the plan
    if matches!(key.code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')) {
        return Some(Msg::PlanModeApprove);
    }
    // n/N denies the plan
    if matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')) {
        return Some(Msg::PlanModeDeny);
    }
    // Up/k or Down/j scrolls the plan
    if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
        return Some(Msg::PlanModeViewPrev);
    }
    if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
        return Some(Msg::PlanModeViewNext);
    }
    // Any character adds to user comment
    if let KeyCode::Char(c) = key.code {
        if state.plan_modal.is_open() {
            return Some(Msg::TextareaKey(key));
        }
    }
    None
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

fn key_to_context_usage_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseContextUsageModal),
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

fn key_to_file_picker_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseFilePicker),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::FilePickerUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::FilePickerDown),
        KeyCode::Enter => Some(Msg::FilePickerConfirm),
        KeyCode::Backspace => Some(Msg::FilePickerBackspace),
        KeyCode::Char(c) => Some(Msg::FilePickerFilter(c)),
        _ => None,
    }
}

fn key_to_history_search_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchCancel),
        KeyCode::Enter => Some(Msg::HistorySearchConfirm),
        KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchPrev),
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchNext),
        KeyCode::Backspace => Some(Msg::HistorySearchBackspace),
        KeyCode::Char(c) => Some(Msg::HistorySearchQuery(c)),
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

fn key_to_questionnaire_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseQuestionnaire),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::QuestionnaireUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::QuestionnaireDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Msg::QuestionnairePrevQuestion),
        KeyCode::Right | KeyCode::Char('l') => Some(Msg::QuestionnaireNextQuestion),
        KeyCode::Enter => Some(Msg::QuestionnaireSelect),
        KeyCode::Char('z') => Some(Msg::QuestionnaireToggleCustom),
        _ => None,
    }
}
