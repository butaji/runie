use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::state::{Msg, OnboardingStep};

// --- Chat navigation helpers ---

type KeyHandler = fn(KeyEvent) -> Option<Msg>;

fn scroll_down(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Char('j') | KeyCode::Down).then_some(Msg::ScrollDown)
}

fn scroll_up(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Char('k') | KeyCode::Up).then_some(Msg::ScrollUp)
}

fn scroll_to_top(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('g')).then_some(Msg::ScrollToTop)
}

fn scroll_to_bottom(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('G')).then_some(Msg::ScrollToBottom)
}

fn scroll_to_prev_user(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Char('H'))
        .then_some(Msg::ScrollToPrevUserTurn)
}

fn scroll_to_next_user(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Char('L'))
        .then_some(Msg::ScrollToNextUserTurn)
}

fn collapse_entry(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Left | KeyCode::Char('h')).then_some(Msg::CollapseEntry)
}

fn expand_entry(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Right | KeyCode::Char('l')).then_some(Msg::ExpandEntry)
}

fn toggle_fold_entry(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('e')).then_some(Msg::ToggleFoldEntry)
}

fn toggle_all_entries(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('E')).then_some(Msg::ToggleAllEntries)
}

fn copy_block_content(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('y')).then_some(Msg::CopyBlockContent)
}

fn copy_block_metadata(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('Y')).then_some(Msg::CopyBlockMetadata)
}

fn toggle_raw_markdown(key: KeyEvent) -> Option<Msg> {
    (key.code == KeyCode::Char('r')).then_some(Msg::ToggleRawMarkdown)
}

fn focus_prompt(key: KeyEvent) -> Option<Msg> {
    matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('i')).then_some(Msg::FocusPrompt)
}

fn page_scroll(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        _ => None,
    }
}

fn shift_scroll_nav(key: KeyEvent) -> Option<Msg> {
    if !key.modifiers.contains(KeyModifiers::SHIFT) {
        return None;
    }
    match key.code {
        KeyCode::Left => Some(Msg::ScrollToPrevUserTurn),
        KeyCode::Right => Some(Msg::ScrollToNextUserTurn),
        _ => None,
    }
}

/// Vim-style navigation when scroll is focused.
fn scroll_focused_nav(key: KeyEvent) -> Option<Msg> {
    nav_keys(key)
        .or_else(|| fold_keys(key))
        .or_else(|| action_keys(key))
        .or_else(|| scroll_nav_page(key))
        .or_else(|| shift_scroll_nav(key))
        .or_else(|| focus_prompt(key))
}

/// Navigation keys: j/k/g/G/H/L
fn nav_keys(key: KeyEvent) -> Option<Msg> {
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

/// Fold keys: h/l/e/E
fn fold_keys(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('h') | KeyCode::Left => Some(Msg::CollapseEntry),
        KeyCode::Char('l') | KeyCode::Right => Some(Msg::ExpandEntry),
        KeyCode::Char('e') => Some(Msg::ToggleFoldEntry),
        KeyCode::Char('E') => Some(Msg::ToggleAllEntries),
        _ => None,
    }
}

/// Action keys: y/Y/r/o/O
fn action_keys(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Char('y') => Some(Msg::CopyBlockContent),
        KeyCode::Char('Y') => Some(Msg::CopyBlockMetadata),
        KeyCode::Char('r') => Some(Msg::ToggleRawMarkdown),
        KeyCode::Char('o') => Some(Msg::OpenEntry),
        KeyCode::Char('O') => Some(Msg::OpenEntryOptions),
        _ => None,
    }
}

/// Prompt/history navigation when scroll is NOT focused.
fn prompt_focused_nav(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Up => Some(Msg::HistoryUp),
        KeyCode::Down => Some(Msg::HistoryDown),
        KeyCode::PageUp => Some(Msg::ScrollPageUp),
        KeyCode::PageDown => Some(Msg::ScrollPageDown),
        _ => None,
    }
}

/// Routes chat navigation based on scroll focus state.
pub fn chat_navigation_msg(key: KeyEvent, scroll_focused: bool) -> Option<Msg> {
    if scroll_focused {
        scroll_focused_nav(key)
    } else {
        prompt_focused_nav(key)
    }
}

// --- History search ---

/// History search mode key handling.
pub fn key_to_history_search_msg(key: KeyEvent) -> Option<Msg> {
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

fn is_history_search_cancel(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc) || (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
}

fn history_search_ctrl_nav(key: &KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchPrev),
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::HistorySearchNext),
        _ => None,
    }
}

// --- Overlay mode ---

/// Overlay escape key handler.
fn handle_overlay_esc() -> Option<Msg> {
    Some(Msg::CloseModal)
}

/// Overlay navigation keys (vim-style + arrows).
fn handle_overlay_nav(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
        _ => None,
    }
}

/// Overlay mode key handling.
pub fn key_to_overlay_msg(key: KeyEvent, state: &crate::tui::state::AppState) -> Option<Msg> {
    // Ctrl+Q closes overlay
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('q')) {
        return handle_overlay_esc();
    }
    // Block plain Ctrl+letter combos (not Ctrl+Q)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // Route to extensions modal when active
    if state.extensions_modal.is_some() {
        return key_to_extensions_modal_msg(key);
    }
    // Route to model picker when active
    if state.model_picker.is_some() {
        return key_to_model_picker_msg(key);
    }
    // Default overlay navigation
    match key.code {
        KeyCode::Esc => handle_overlay_esc(),
        _ => handle_overlay_nav(key),
    }
}

/// Extensions modal key handling.
pub fn key_to_extensions_modal_msg(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Enter => Some(Msg::ExtensionsModalSelect),
        KeyCode::Backspace => Some(Msg::ExtensionsModalSearchBackspace),
        KeyCode::Char(c) => Some(Msg::ExtensionsModalSearchInput(c)),
        _ => extensions_modal_nav(key.code),
    }
}

fn extensions_modal_nav(code: KeyCode) -> Option<Msg> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::ExtensionsModalUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::ExtensionsModalDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Msg::ExtensionsModalLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Msg::ExtensionsModalRight),
        _ => None,
    }
}

/// Model picker key handling.
fn key_to_model_picker_msg(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseModal),
        KeyCode::Up | KeyCode::Char('k') => Some(Msg::SelectUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Msg::SelectDown),
        KeyCode::Enter => Some(Msg::SelectConfirm),
        KeyCode::Char('d') => Some(Msg::SelectToggleDetails),
        _ => None,
    }
}

// --- Onboarding helpers ---

pub fn key_to_onboarding_navigation(key: KeyEvent) -> Option<Msg> {
    if matches!(key.code, KeyCode::Up) {
        return Some(Msg::OnboardingNavigateUp);
    }
    if matches!(key.code, KeyCode::Down) {
        return Some(Msg::OnboardingNavigateDown);
    }
    if matches!(key.code, KeyCode::Enter) {
        return Some(Msg::OnboardingNext);
    }
    None
}

pub fn key_to_onboarding_esc(is_welcome: bool) -> Option<Msg> {
    Some(if is_welcome { Msg::OnboardingSkip } else { Msg::OnboardingBack })
}

pub fn key_to_onboarding_space(state: &crate::tui::state::AppState) -> Option<Msg> {
    let step = state.onboarding.as_ref().map(|o| o.step.clone());
    if matches!(step, Some(OnboardingStep::ModelSelect)) {
        let idx = state.onboarding.as_ref().map(|o| o.selected_item).unwrap_or(0);
        return Some(Msg::OnboardingSelectModel(idx));
    }
    None
}

pub fn key_to_onboarding_char(key: KeyEvent, is_picker_step: bool) -> Option<Msg> {
    if let KeyCode::Char(c) = key.code {
        return Some(if is_picker_step {
            Msg::OnboardingSearchInput(c)
        } else {
            Msg::OnboardingKeyInput(c)
        });
    }
    None
}

pub fn key_to_onboarding_backspace(is_picker_step: bool) -> Option<Msg> {
    Some(if is_picker_step {
        Msg::OnboardingSearchBackspace
    } else {
        Msg::OnboardingKeyBackspace
    })
}

// --- Shortcuts panel ---

pub fn shortcuts_panel_filter_msg(key: KeyEvent) -> Option<Msg> {
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

pub fn shortcuts_panel_normal_msg(key: KeyEvent) -> Option<Msg> {
    match key.code {
        KeyCode::Esc => Some(Msg::CloseShortcutsPanel),
        KeyCode::Char('f') | KeyCode::Char('/') => Some(Msg::ShortcutsPanelToggleFilter),
        KeyCode::Char('e') | KeyCode::Enter | KeyCode::Char(' ') => Some(Msg::ShortcutsPanelToggleSection),
        KeyCode::Up => Some(Msg::ShortcutsPanelUp),
        KeyCode::Down => Some(Msg::ShortcutsPanelDown),
        _ => None,
    }
}
