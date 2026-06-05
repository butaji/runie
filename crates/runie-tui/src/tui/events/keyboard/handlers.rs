//! Individual key handlers for each mode.

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{AppState, Msg, OnboardingStep};

pub(super) fn key_to_overlay_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q')) {
        return Some(Msg::CloseModal);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    if state.extensions_modal.is_some() {
        return key_to_extensions_modal_msg(key);
    }
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

pub(super) fn key_to_model_picker_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
        KeyCode::Char('d') => Some(Msg::SelectToggleDetails),
        _ => None,
    }
}

pub(super) fn key_to_extensions_modal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    use crate::tui::state::Msg;
    // Esc, Enter, Backspace, Char - 4 arms
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Enter => Some(Msg::ExtensionsModalSelect),
        KeyCode::Backspace => Some(Msg::ExtensionsModalSearchBackspace),
        KeyCode::Char(c) => Some(Msg::ExtensionsModalSearchInput(c)),
        _ => extensions_modal_nav(key.code),
    }
}

fn extensions_modal_nav(code: KeyCode) -> Option<Msg> {
    use crate::tui::state::Msg;
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::ExtensionsModalUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::ExtensionsModalDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Msg::ExtensionsModalLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Msg::ExtensionsModalRight),
        _ => None,
    }
}

pub(super) fn key_to_slash_menu_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseSlashMenu),
        KeyCode::Up => Some(Msg::SlashMenuUp),
        KeyCode::Down => Some(Msg::SlashMenuDown),
        KeyCode::Enter => Some(Msg::SlashMenuConfirm),
        // Let character keys fall through to normal routing so they update textarea + filter
        _ => None,
    }
}

pub(super) fn shortcuts_panel_filter_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseShortcutsPanel),
        KeyCode::Backspace => Some(Msg::ShortcutsPanelFilterBackspace),
        KeyCode::Char(c) => Some(Msg::ShortcutsPanelFilterInput(c)),
        KeyCode::Up => Some(Msg::ShortcutsPanelUp),
        KeyCode::Down => Some(Msg::ShortcutsPanelDown),
        KeyCode::Enter => Some(Msg::ShortcutsPanelToggleSection),
        _ => None,
    }
}

pub(super) fn shortcuts_panel_normal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseShortcutsPanel),
        KeyCode::Char('f') | KeyCode::Char('/') => Some(Msg::ShortcutsPanelToggleFilter),
        KeyCode::Char('e') | KeyCode::Enter | KeyCode::Char(' ') => Some(Msg::ShortcutsPanelToggleSection),
        KeyCode::Up => Some(Msg::ShortcutsPanelUp),
        KeyCode::Down => Some(Msg::ShortcutsPanelDown),
        _ => None,
    }
}

pub(super) fn key_to_shortcuts_panel_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if state.shortcuts_panel.filter_mode {
        shortcuts_panel_filter_msg(key)
    } else {
        shortcuts_panel_normal_msg(key)
    }
}

pub(super) fn key_to_home_screen_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if let Some(msg) = home_action_keys(key) {
        return Some(msg);
    }
    home_nav_keys(key)
}

fn home_action_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('s') => Some(Msg::CloseHomeScreen),
        KeyCode::Char('w') => Some(Msg::ToggleWorktreeMode),
        KeyCode::Char('i') => Some(Msg::ImportClaudeSettings),
        _ => None,
    }
}

fn home_nav_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::Quit),
        KeyCode::Up => Some(Msg::HomeScreenUp),
        KeyCode::Down => Some(Msg::HomeScreenDown),
        KeyCode::Enter | KeyCode::Char('n') => Some(Msg::HomeScreenSelect),
        KeyCode::Char('r') => Some(Msg::CloseHomeScreen),
        KeyCode::Char('s') => Some(Msg::OpenSettingsModal),
        KeyCode::Char('h') => Some(Msg::HomeScreenToggleSessions),
        _ => None,
    }
}

pub(super) fn key_to_settings_modal_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
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

pub(super) fn chat_navigation_msg(key: crossterm::event::KeyEvent, scroll_focused: bool) -> Option<Msg> {
    if scroll_focused {
        scroll_focused_nav(key)
    } else {
        prompt_focused_nav(key)
    }
}

fn scroll_focused_nav(key: crossterm::event::KeyEvent) -> Option<Msg> {
    nav_keys(key)
        .or_else(|| fold_keys(key))
        .or_else(|| action_keys(key))
        .or_else(|| page_shift_keys(key))
        .or_else(|| focus_key(key))
}
fn nav_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Msg::ScrollDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Msg::ScrollUp),
        KeyCode::Char('g') => Some(Msg::ScrollToTop),
        KeyCode::Char('G') => Some(Msg::ScrollToBottom),
        KeyCode::Char('H') => Some(Msg::ScrollToPrevUserTurn),
        KeyCode::Char('L') => Some(Msg::ScrollToNextUserTurn),
        _ => None,
    }
}
fn fold_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    let mods = key.modifiers;
    match key.code {
        KeyCode::Left | KeyCode::Char('h') if mods.is_empty() => Some(Msg::CollapseEntry),
        KeyCode::Right | KeyCode::Char('l') if mods.is_empty() => Some(Msg::ExpandEntry),
        KeyCode::Char('e') => Some(Msg::ToggleFoldEntry),
        KeyCode::Char('E') => Some(Msg::ToggleAllEntries),
        _ => None,
    }
}
fn action_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('y') => Some(Msg::CopyBlockContent),
        KeyCode::Char('Y') => Some(Msg::CopyBlockMetadata),
        KeyCode::Char('r') => Some(Msg::ToggleRawMarkdown),
        KeyCode::Char('o') => Some(Msg::OpenEntry),
        KeyCode::Char('O') => Some(Msg::OpenEntryOptions),
        _ => None,
    }
}
fn page_shift_keys(key: crossterm::event::KeyEvent) -> Option<Msg> {
    let mods = key.modifiers;
    match key.code {
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        KeyCode::Left if mods.contains(KeyModifiers::SHIFT) => Some(Msg::ScrollToPrevUserTurn),
        KeyCode::Right if mods.contains(KeyModifiers::SHIFT) => Some(Msg::ScrollToNextUserTurn),
        _ => None,
    }
}
fn focus_key(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('i') => Some(Msg::FocusPrompt),
        _ => None,
    }
}
fn prompt_focused_nav(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Up => Some(Msg::HistoryUp),
        KeyCode::Down => Some(Msg::HistoryDown),
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        _ => None,
    }
}

pub(super) fn key_to_chat_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if let Some(msg) = handle_chat_ctrl(key) {
        return Some(msg);
    }
    if let Some(msg) = handle_chat_special(key) {
        return Some(msg);
    }
    if let Some(msg) = chat_navigation_msg(key, state.scroll.scroll_focused) {
        return Some(msg);
    }
    Some(Msg::TextareaKey(key))
}

fn handle_chat_ctrl(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
        match key.code {
            KeyCode::Char('e') => return Some(Msg::ToggleThoughts),
            KeyCode::Char('a') => return Some(Msg::ToggleSubagentPanel),
            KeyCode::Char('n') => return Some(Msg::NewSessionWorktree),
            _ => {}
        }
    }
    if is_ctrl_combo(key) {
        return ctrl_chat_key(key);
    }
    None
}

fn handle_chat_special(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Enter) {
        return if key.modifiers.contains(KeyModifiers::SHIFT) {
            Some(Msg::InsertNewline)
        } else {
            Some(Msg::Submit)
        };
    }
    if matches!(key.code, KeyCode::Esc) {
        return Some(Msg::ToggleScrollFocus);
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) && matches!(key.code, KeyCode::Tab) {
        return Some(Msg::TogglePermissionMode);
    }
    if matches!(key.code, KeyCode::Tab) {
        return Some(Msg::ToggleScrollFocus);
    }
    if matches!(key.code, KeyCode::Char('i') | KeyCode::Char(' ')) {
        return Some(Msg::FocusPrompt);
    }
    None
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

pub(super) fn key_to_permission_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if is_ctrl_combo(key) && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
        return Some(Msg::PermissionCancel);
    }
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => Some(Msg::PermissionConfirm),
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => Some(Msg::PermissionCancel),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(Msg::PermissionAlways),
        KeyCode::Char('s') => Some(Msg::PermissionSkip),
        _ => None,
    }
}

pub(super) fn key_to_plan_modal_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    if matches!(key.code, KeyCode::Esc) {
        return Some(Msg::CloseModal);
    }
    if matches!(key.code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')) {
        return Some(Msg::PlanModeApprove);
    }
    if matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')) {
        return Some(Msg::PlanModeDeny);
    }
    if matches!(key.code, KeyCode::Up | KeyCode::Char('k')) {
        return Some(Msg::PlanModeViewPrev);
    }
    if matches!(key.code, KeyCode::Down | KeyCode::Char('j')) {
        return Some(Msg::PlanModeViewNext);
    }
    if let KeyCode::Char(_c) = key.code {
        if state.plan_modal.is_open() {
            return Some(Msg::TextareaKey(key));
        }
    }
    None
}

pub(super) fn key_to_palette_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Esc) {
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

pub(super) fn key_to_context_usage_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Msg::CloseContextUsageModal),
        _ => None,
    }
}

pub(super) fn key_to_diff_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
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

pub(super) fn key_to_tree_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SessionTreeUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SessionTreeDown),
        KeyCode::Enter => Some(Msg::SessionTreeConfirm),
        _ => None,
    }
}

pub(super) fn key_to_file_picker_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
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

pub(super) fn key_to_history_search_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if is_history_search_cancel(&key) {
        return Some(Msg::HistorySearchCancel);
    }
    match key.code {
        KeyCode::Enter => Some(Msg::HistorySearchConfirm),
        KeyCode::Backspace => Some(Msg::HistorySearchBackspace),
        KeyCode::Char(c) => Some(Msg::HistorySearchQuery(c)),
        _ => history_search_ctrl_nav(&key),
    }
}

fn is_history_search_cancel(key: &crossterm::event::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc) || (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
}

fn history_search_ctrl_nav(key: &crossterm::event::KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchPrev),
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchNext),
        _ => None,
    }
}

pub(super) fn key_to_onboarding_navigation(key: crossterm::event::KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Up) { return Some(Msg::OnboardingNavigateUp); }
    if matches!(key.code, KeyCode::Down) { return Some(Msg::OnboardingNavigateDown); }
    if matches!(key.code, KeyCode::Enter) { return Some(Msg::OnboardingNext); }
    None
}

pub(super) fn key_to_onboarding_esc(is_welcome: bool) -> Option<Msg> {
    Some(if is_welcome { Msg::OnboardingSkip } else { Msg::OnboardingBack })
}

pub(super) fn key_to_onboarding_space(state: &AppState) -> Option<Msg> {
    let step = state.onboarding.as_ref().map(|o| o.step.clone());
    if matches!(step, Some(OnboardingStep::ModelSelect)) {
        let idx = state.onboarding.as_ref().map(|o| o.selected_item).unwrap_or(0);
        return Some(Msg::OnboardingSelectModel(idx));
    }
    None
}

pub(super) fn key_to_onboarding_char(key: crossterm::event::KeyEvent, is_picker_step: bool) -> Option<Msg> {
    if let KeyCode::Char(c) = key.code {
        return Some(if is_picker_step { Msg::OnboardingSearchInput(c) } else { Msg::OnboardingKeyInput(c) });
    }
    None
}

pub(super) fn key_to_onboarding_backspace(is_picker_step: bool) -> Option<Msg> {
    Some(if is_picker_step { Msg::OnboardingSearchBackspace } else { Msg::OnboardingKeyBackspace })
}

pub(super) fn key_to_onboarding_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
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

pub(super) fn key_to_questionnaire_msg(key: crossterm::event::KeyEvent) -> Option<Msg> {
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
fn is_ctrl_combo(key: crossterm::event::KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::SHIFT)
}
