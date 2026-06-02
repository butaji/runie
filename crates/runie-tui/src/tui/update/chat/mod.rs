//! Chat domain update functions.
//! Handles: messages, textarea input, scroll, submit, clear.

pub mod modal;
pub mod submit;
pub mod scroll;
pub mod search;
pub mod metadata;
pub mod permission;
pub mod entry;
pub mod history;

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::update::ui::UiCmd;
use crate::tui::key_to_textarea_input;

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
    matches!(msg, Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown
        | Msg::ScrollHalfPageUp | Msg::ScrollHalfPageDown | Msg::ScrollToTop | Msg::ScrollToBottom
        | Msg::ScrollToPrevUserTurn | Msg::ScrollToNextUserTurn)
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
    

    // Fast-path: predicate-based routing for grouped message families
    if is_input_msg(&msg) { return handle_input_msg(state, msg); }
    if is_scroll_msg(&msg) { return scroll::handle_scroll_msg(state, &msg); }
    if is_clear_msg(&msg) { return handle_clear_msg(state, msg); }
    if is_history_msg(&msg) { return history::handle_history_msg(state, msg); }
    if is_history_search_msg(&msg) { return search::handle_history_search_msg(state, msg); }
    if is_overlay_msg(&msg) { return handle_overlay_msg(state, msg); }

    // Try handlers with early return
    if let Some(cmds) = try_handle_permission(state, &msg) { return cmds; }
    if let Some(cmds) = try_handle_entry(state, &msg) { return cmds; }
    if let Some(cmds) = try_handle_misc(state, &msg) { return cmds; }

    vec![]
}

fn try_handle_permission(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<ChatCmd>> {
    use crate::tui::state::Msg;
    match msg {
        Msg::TogglePermissionMode => { permission::toggle_permission_mode(state); Some(vec![]) }
        Msg::ToggleAutoApprove => { permission::toggle_auto_approve(state); Some(vec![]) }
        Msg::ClearAlwaysApprove => { permission::clear_always_approve(state); Some(vec![]) }
        Msg::ToggleScrollFocus => { permission::toggle_scroll_focus(state); Some(vec![]) }
        _ => None,
    }
}

fn try_handle_entry(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<ChatCmd>> {
    try_handle_entry_fold(state, msg).or_else(|| try_handle_entry_action(state, msg))
}

fn try_handle_entry_fold(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<ChatCmd>> {
    use crate::tui::state::Msg;
    match msg {
        Msg::CollapseEntry => { entry::collapse_entry(state); Some(vec![]) }
        Msg::ExpandEntry => { entry::expand_entry(state); Some(vec![]) }
        Msg::ToggleFoldEntry => { entry::toggle_fold_entry(state); Some(vec![]) }
        Msg::ToggleAllEntries => { entry::toggle_all_entries(state); Some(vec![]) }
        _ => None,
    }
}

fn try_handle_entry_action(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<ChatCmd>> {
    use crate::tui::state::Msg;
    match msg {
        Msg::CopyBlockContent => { entry::copy_block_content(state); Some(vec![]) }
        Msg::CopyBlockMetadata => { entry::copy_block_metadata(state); Some(vec![]) }
        Msg::OpenEntry => { entry::open_entry(state); Some(vec![]) }
        Msg::OpenEntryOptions => { entry::open_entry_options(state); Some(vec![]) }
        _ => None,
    }
}

fn try_handle_misc(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<ChatCmd>> {
    use crate::tui::state::Msg;
    match msg {
        Msg::Submit => Some(submit::handle_submit(state)),
        Msg::Interject => Some(submit::handle_interject(state)),
        Msg::MouseClick { x, y, button } => { handle_mouse_click(state, *x, *y, *button); Some(vec![]) }
        Msg::ToggleRawMarkdown => { entry::toggle_raw_markdown(state); Some(vec![]) }
        Msg::FocusPrompt => { entry::focus_prompt(state); Some(vec![]) }
        Msg::GoHome => { entry::go_home(state); Some(vec![]) }
        _ => None,
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

fn handle_clear_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::ClearInputConfirm => handle_clear_input_confirm(state),
        Msg::ClearInput => handle_clear_input(state),
        Msg::ClearChat => handle_clear_chat(state),
        _ => vec![],
    }
}

fn handle_newline(state: &mut AppState) -> Vec<ChatCmd> {
    state.textarea.insert_newline();
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

fn handle_mouse_click(state: &mut AppState, x: u16, y: u16, _button: u16) {
    tracing::debug!("Mouse click at ({}, {})", x, y);
    let entry_index = y as usize;
    if entry_index < state.messages.len() {
        state.scroll.feed_offset = entry_index;
        state.scroll.scroll_focused = true;
        state.input_right_info = String::new();
    }
}
