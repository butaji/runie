//! Chat domain update functions.
//! Handles: messages, textarea input, scroll, submit, clear.

pub mod modal;
pub mod submit;

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::ui::UiCmd;
use crate::tui::key_to_textarea_input;
use std::time::Instant;

fn current_timestamp() -> Option<String> {
    use chrono::Local;
    Some(Local::now().format("%-I:%M %p").to_string())
}

/// Chat-specific commands returned by update functions.
#[derive(Debug, Clone)]
pub enum ChatCmd {
    SpawnAgent { messages: Vec<runie_agent::AgentMessage> },
    Ui(UiCmd),
}

impl From<ChatCmd> for crate::tui::state::Cmd {
    fn from(cmd: ChatCmd) -> Self {
        match cmd {
            ChatCmd::SpawnAgent { messages } => crate::tui::state::Cmd::SpawnAgent { messages },
            ChatCmd::Ui(ui_cmd) => crate::tui::state::Cmd::from(ui_cmd),
        }
    }
}

fn is_input_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::TextareaKey(_) | Msg::InsertNewline | Msg::Paste(_))
}

fn is_scroll_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown)
}

fn is_clear_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::ClearInputConfirm | Msg::ClearInput | Msg::ClearChat)
}

fn is_history_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::HistoryUp | Msg::HistoryDown)
}

fn is_history_search_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::HistorySearchStart | Msg::HistorySearchQuery(_) | Msg::HistorySearchBackspace | Msg::HistorySearchNext | Msg::HistorySearchPrev | Msg::HistorySearchCancel | Msg::HistorySearchConfirm)
}

fn is_slash_menu_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::SlashMenuUp | Msg::SlashMenuDown | Msg::SlashMenuConfirm | Msg::CloseSlashMenu)
}

fn is_shortcuts_panel_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::OpenShortcutsPanel | Msg::CloseShortcutsPanel | Msg::ShortcutsPanelUp | Msg::ShortcutsPanelDown | Msg::ShortcutsPanelToggleSection | Msg::ShortcutsPanelToggleFilter | Msg::ShortcutsPanelFilterInput(_) | Msg::ShortcutsPanelFilterBackspace)
}

fn is_settings_modal_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::OpenSettingsModal | Msg::CloseSettingsModal | Msg::SettingsModalUp | Msg::SettingsModalDown | Msg::SettingsModalNextTab | Msg::SettingsModalPrevTab | Msg::SettingsModalSelect)
}

fn is_home_screen_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::HomeScreenUp | Msg::HomeScreenDown | Msg::HomeScreenSelect | Msg::CloseHomeScreen)
}

fn is_file_picker_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::FilePickerUp | Msg::FilePickerDown | Msg::FilePickerConfirm | Msg::FilePickerFilter(_) | Msg::FilePickerBackspace | Msg::CloseFilePicker)
}

/// Update chat domain: messages, textarea, scroll, submit, clear.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::Submit => submit::handle_submit(state),
        Msg::Interject => submit::handle_interject(state),
        Msg::TogglePermissionMode => { toggle_permission_mode(state); vec![] }
        Msg::ClearAlwaysApprove => { clear_always_approve(state); vec![] }
        Msg::ToggleScrollFocus => { toggle_scroll_focus(state); vec![] }
        m if is_input_msg(&m) => handle_input_msg(state, m),
        m if is_scroll_msg(&m) => handle_scroll_msg(state, m),
        m if is_clear_msg(&m) => handle_clear_msg(state, m),
        m if is_history_msg(&m) => handle_history_msg(state, m),
        m if is_history_search_msg(&m) => handle_history_search_msg(state, m),
        m if is_overlay_msg(&m) => handle_overlay_msg(state, m),
        _ => vec![],
    }
}

fn is_overlay_msg(msg: &crate::tui::state::Msg) -> bool {
    is_slash_menu_msg(msg) || is_shortcuts_panel_msg(msg) || is_settings_modal_msg(msg)
        || is_home_screen_msg(msg) || is_file_picker_msg(msg)
}

fn handle_overlay_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    if is_slash_menu_msg(&msg) { return modal::handle_slash_menu_msg(state, msg); }
    if is_shortcuts_panel_msg(&msg) { return modal::handle_shortcuts_panel_msg(state, msg); }
    if is_settings_modal_msg(&msg) { return modal::handle_settings_modal_msg(state, msg); }
    if is_home_screen_msg(&msg) { return modal::handle_home_screen_msg(state, msg); }
    if is_file_picker_msg(&msg) { return modal::handle_file_picker_msg(state, msg); }
    vec![]
}

fn handle_input_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    use crate::tui::state::TuiMode;
    if matches!(
        state.mode,
        TuiMode::Permission | TuiMode::Overlay | TuiMode::CommandPalette | TuiMode::Onboarding
    ) {
        return vec![];
    }
    match msg {
        Msg::TextareaKey(key) => { modal::handle_textarea_key(state, key); vec![] }
        Msg::InsertNewline => handle_newline(state),
        Msg::Paste(text) => { handle_paste(state, text); vec![] }
        _ => vec![],
    }
}

fn toggle_permission_mode(state: &mut AppState) {
    use crate::tui::state::PermissionMode;
    state.permission_mode = match state.permission_mode {
        PermissionMode::Normal => PermissionMode::AutoApprove,
        PermissionMode::AutoApprove => PermissionMode::Plan,
        PermissionMode::Plan => PermissionMode::Normal,
    };
    let mode_name = match state.permission_mode {
        PermissionMode::Normal => "Normal",
        PermissionMode::AutoApprove => "AutoApprove",
        PermissionMode::Plan => "Plan",
    };
    state.input_right_info = format!("Mode: {}", mode_name);
}

fn clear_always_approve(state: &mut AppState) {
    let count = state.allowed_tools.len() + state.allowed_categories.len();
    state.allowed_tools.clear();
    state.allowed_categories.clear();
    state.input_right_info = format!("Cleared {} always-approve entries", count);
}

fn toggle_scroll_focus(state: &mut AppState) {
    state.scroll.scroll_focused = !state.scroll.scroll_focused;
    state.input_right_info = if state.scroll.scroll_focused {
        "[SCROLL]".to_string()
    } else {
        String::new()
    };
}

fn handle_scroll_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    // A "page" in the scroll model is one viewport-height worth of
    // messages.  20 is the conventional default; tests rely on this
    // (test_page_scroll_1000_messages) to reach the end of long feeds in
    // a reasonable number of PageDown presses.
    const PAGE_SIZE: i32 = 20;
    match msg {
        Msg::ScrollUp => handle_scroll(state, 1),
        Msg::ScrollDown => handle_scroll(state, -1),
        Msg::ScrollPageUp => handle_scroll(state, PAGE_SIZE),
        Msg::ScrollPageDown => handle_scroll(state, -PAGE_SIZE),
        _ => vec![],
    }
}

fn handle_clear_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::ClearInputConfirm => handle_clear_input_confirm(state),
        Msg::ClearInput => handle_clear_input(state),
        Msg::ClearChat => handle_clear_chat(state),
        _ => vec![],
    }
}

fn handle_history_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::HistoryUp => handle_history_up(state),
        Msg::HistoryDown => handle_history_down(state),
        _ => vec![],
    }
}

fn start_history_search(state: &mut AppState) {
    if !state.input_history.is_empty() {
        state.history_search_query.clear();
        state.history_search_matches = (0..state.input_history.len()).rev().collect();
        state.history_search_index = 0;
        if state.input_history_index.is_none() && state.input_draft.is_empty() {
            state.input_draft = state.textarea.lines().join("\n");
        }
    }
}

fn confirm_history_search(state: &mut AppState) {
    state.history_search_query.clear();
    state.history_search_matches.clear();
    state.history_search_index = 0;
    state.input_history_index = None;
    state.input_draft.clear();
}

fn cancel_history_search(state: &mut AppState) {
    state.textarea.select_all();
    state.textarea.cut();
    state.textarea.insert_str(&state.input_draft);
    state.input_draft.clear();
    state.input_history_index = None;
    state.history_search_query.clear();
    state.history_search_matches.clear();
    state.history_search_index = 0;
}

fn handle_history_search_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::HistorySearchStart => { start_history_search(state); vec![] }
        Msg::HistorySearchQuery(c) => { state.history_search_query.push(c); update_history_search(state); vec![] }
        Msg::HistorySearchBackspace => { state.history_search_query.pop(); update_history_search(state); vec![] }
        Msg::HistorySearchNext => {
            if !state.history_search_matches.is_empty() {
                state.history_search_index = (state.history_search_index + 1).min(state.history_search_matches.len() - 1);
                apply_history_search_selection(state);
            }
            vec![]
        }
        Msg::HistorySearchPrev => { state.history_search_index = state.history_search_index.saturating_sub(1); apply_history_search_selection(state); vec![] }
        Msg::HistorySearchConfirm => { confirm_history_search(state); vec![] }
        Msg::HistorySearchCancel => { cancel_history_search(state); vec![] }
        _ => vec![],
    }
}

fn update_history_search(state: &mut AppState) {
    let query = state.history_search_query.to_lowercase();
    state.history_search_matches = state.input_history
        .iter()
        .enumerate()
        .filter(|(_, text)| text.to_lowercase().contains(&query))
        .map(|(i, _)| i)
        .rev()
        .collect();
    state.history_search_index = 0;
    apply_history_search_selection(state);
}

fn apply_history_search_selection(state: &mut AppState) {
    if let Some(&idx) = state.history_search_matches.get(state.history_search_index) {
        if let Some(text) = state.input_history.get(idx) {
            state.textarea.select_all();
            state.textarea.cut();
            state.textarea.insert_str(text);
            state.input_history_index = Some(idx);
        }
    }
}

fn handle_newline(state: &mut AppState) -> Vec<ChatCmd> {
    state.textarea.insert_newline();
    vec![]
}

fn handle_scroll(state: &mut AppState, delta: i32) -> Vec<ChatCmd> {
    let page = delta.unsigned_abs() as usize;
    let new_offset = if delta > 0 {
        state.scroll.feed_offset.saturating_sub(page)
    } else {
        // saturating_add guards against usize overflow when feed_offset has
        // been set to an extreme value (e.g. usize::MAX from a test) — the
        // .min below then clamps to the last valid message index.
        state.scroll
            .feed_offset
            .saturating_add(page)
            .min(state.messages.len().saturating_sub(1))
    };
    state.scroll.feed_offset = new_offset;
    state.scroll.user_scrolled_up = new_offset > 0;
    vec![]
}

fn handle_clear_input(state: &mut AppState) -> Vec<ChatCmd> {
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    vec![]
}

fn handle_clear_chat(state: &mut AppState) -> Vec<ChatCmd> {
    state.messages.clear();
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
    vec![]
}

fn handle_paste(state: &mut AppState, text: String) -> Vec<ChatCmd> {
    // Pasting cancels any in-progress history browsing — the visible text
    // is no longer a history entry, the user's draft is discarded.  We
    // then append at the cursor so existing textarea content is preserved
    // (test_paste_appends_to_existing) UNLESS the current text is exactly
    // a history item, in which case it is replaced (test_paste_while_browsing_history).
    let current = state.textarea.lines().join("\n");
    let is_history_view = state.input_history_index.is_some()
        && state
            .input_history
            .get(state.input_history_index.unwrap())
            .map(|h| h == &current)
            .unwrap_or(false);
    state.input_history_index = None;
    state.input_draft.clear();
    if is_history_view {
        state.textarea.select_all();
        state.textarea.cut();
    } else {
        state.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    }
    state.textarea.insert_str(&text);
    vec![]
}


// ─── Clear Input Confirm ────────────────────────────────────────────────────────

fn handle_clear_input_confirm(state: &mut AppState) -> Vec<ChatCmd> {
    if state.clear_input_confirm.wants_clear() {
        state.textarea.select_all();
        state.textarea.delete_line_by_end();
        state.input_right_info = String::new();
    } else {
        state.input_right_info = "Ctrl+C again to clear text".to_string();
    }
    vec![]
}

// ─── Input History ─────────────────────────────────────────────────────────────

fn handle_history_up(state: &mut AppState) -> Vec<ChatCmd> {
    if state.input_history.is_empty() {
        return vec![];
    }

    // Save current draft only on the FIRST history-up press. If the draft
    // is already populated (from a previous session, or restored by a
    // history-down), do not overwrite it — otherwise the user's pre-existing
    // draft is lost the moment they press up.
    if state.input_history_index.is_none() && state.input_draft.is_empty() {
        state.input_draft = state.textarea.lines().join("\n");
    }

    // Move back in history
    let new_index = state.input_history_index.map_or(
        state.input_history.len().saturating_sub(1),
        |i| i.saturating_sub(1),
    );

    if let Some(text) = state.input_history.get(new_index) {
        state.input_history_index = Some(new_index);
        state.textarea.select_all();
        state.textarea.cut();
        state.textarea.insert_str(text);
    }
    vec![]
}

fn handle_history_down(state: &mut AppState) -> Vec<ChatCmd> {
    if let Some(index) = state.input_history_index {
        if index + 1 >= state.input_history.len() {
            // Back to draft
            state.input_history_index = None;
            state.textarea.select_all();
            state.textarea.cut();
            state.textarea.insert_str(&state.input_draft);
            state.input_draft.clear();
        } else {
            // Forward in history
            let new_index = index + 1;
            if let Some(text) = state.input_history.get(new_index) {
                state.input_history_index = Some(new_index);
                state.textarea.select_all();
                state.textarea.cut();
                state.textarea.insert_str(text);
            }
        }
    }
    vec![]
}

// ─── Message Conversion ────────────────────────────────────────────────────────

fn to_agent_messages(items: &[MessageItem]) -> Vec<runie_agent::AgentMessage> {
    use runie_agent::{AgentMessage, ContentPart};
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0, usage: None, stop_reason: None, error_message: None,
            tool_calls: vec![],
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0, usage: None, stop_reason: None, error_message: None,
            tool_calls: vec![],
        }),
        MessageItem::Error { .. } => None,
        _ => None,
    }).collect()
}
