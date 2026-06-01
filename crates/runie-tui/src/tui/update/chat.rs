//! Chat domain update functions.
//! Handles: messages, textarea input, scroll, submit, clear.

use crate::components::MessageItem;
use crate::tui::state::AppState;
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

/// Update chat domain: messages, textarea, scroll, submit, clear.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;

    // Submit
    if matches!(msg, Msg::Submit) { return handle_submit(state); }

    // Textarea input - group
    if matches!(msg, Msg::TextareaKey(_) | Msg::InsertNewline | Msg::Paste(_)) {
        return handle_input_msg(state, msg);
    }

    // Scroll - combine all scroll messages
    if matches!(msg, Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown) {
        return handle_scroll_msg(state, msg);
    }

    // Clear - group
    if matches!(msg, Msg::ClearInputConfirm | Msg::ClearInput | Msg::ClearChat) {
        return handle_clear_msg(state, msg);
    }

    // History - group
    if matches!(msg, Msg::HistoryUp | Msg::HistoryDown) {
        return handle_history_msg(state, msg);
    }

    vec![]
}

fn handle_input_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    use crate::tui::state::TuiMode;
    // Block textarea input in modal/blocking modes (permission, overlay,
    // command palette, onboarding) so paste/type can't leak. DiffViewer
    // intentionally still allows it for copy-paste comparisons.
    if matches!(
        state.mode,
        TuiMode::Permission | TuiMode::Overlay | TuiMode::CommandPalette | TuiMode::Onboarding
    ) {
        return vec![];
    }
    match msg {
        Msg::TextareaKey(key) => { handle_textarea_key(state, key); vec![] }
        Msg::InsertNewline => handle_newline(state),
        Msg::Paste(text) => { handle_paste(state, text); vec![] }
        _ => vec![],
    }
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

fn handle_textarea_key(state: &mut AppState, key: crossterm::event::KeyEvent) -> Vec<ChatCmd> {
    state.textarea.input(key_to_textarea_input(key));
    vec![]
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

// ─── Submit ─────────────────────────────────────────────────────────────────────

fn handle_submit(state: &mut AppState) -> Vec<ChatCmd> {
    let text = state.textarea.lines().join("\n");
    if text.chars().all(|c| c.is_whitespace()) {
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }

    // Check if input is a slash command - route to slash handler instead of agent
    if text.starts_with('/') {
        if let Some(cmd) = runie_core::slash_command::parse_slash_command(&text) {
            let commands = super::slash::handle_slash(state, cmd);
            state.textarea.select_all();
            state.textarea.delete_line_by_end();
            return commands.into_iter().map(ChatCmd::Ui).collect();
        } else {
            // Unknown slash command — show error, don't submit
            state.messages.push(MessageItem::Error {
                message: format!("Unknown command: {}. Type /help for available commands.", text),
                recoverable: true,
            });
            state.textarea.select_all();
            state.textarea.delete_line_by_end();
            return vec![];
        }
    }

    // Bug 1 fix: If agent is already running, cancel it first before proceeding.
    // This ensures the old placeholder is removed before adding new message.
    if state.agent_running {
        cancel_running_agent(state);
    }

    if should_defer_submit(state) {
        return vec![];
    }

    prepare_agent_messages(state, &text);
    finalize_submit(state, text)
}

// ─── Helper functions ───────────────────────────────────────────────────────

fn prepare_agent_messages(state: &mut AppState, text: &str) {
    save_to_history(state, text);
    reset_history_nav(state);
    // Note: cancel_running_agent is called in handle_submit BEFORE prepare_agent_messages
    reset_scroll(state);
    state.agent_start_time = Some(Instant::now());
    // Bug 1 fix: Do NOT set agent_running = true here.
    // agent_running should only be set in finalize_submit when we actually proceed.
    // This prevents deferred submits from blocking subsequent submits.
}

fn save_to_history(state: &mut AppState, text: &str) {
    if !text.trim().is_empty() {
        state.input_history.push(text.to_string());
        if state.input_history.len() > 100 {
            state.input_history.remove(0);
        }
    }
}

fn reset_history_nav(state: &mut AppState) {
    state.input_history_index = None;
    state.input_draft.clear();
}

fn cancel_running_agent(state: &mut AppState) {
    if !state.agent_running {
        return;
    }
    state.agent_running = false;
    state.is_thinking = false;
    state.thinking_start = None;
    state.thinking_duration = None;
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    // Remove empty placeholder from previous turn if exists
    if let Some(MessageItem::Assistant { text, .. }) = state.messages.last() {
        if text.is_empty() {
            state.messages.pop();
        }
    }
}

fn reset_scroll(state: &mut AppState) {
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
}

fn should_defer_submit(state: &mut AppState) -> bool {
    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.input_right_info = "Loading models... retry after".to_string();
            return true;
        }
    }
    false
}

fn finalize_submit(state: &mut AppState, text: String) -> Vec<ChatCmd> {
    let model_missing =
        state.current_model.as_deref().map_or(true, |s| s.is_empty()) && state.onboarding.is_none();

    if model_missing {
        // No model configured — show error immediately, don't start agent
        add_user_message_only(state, &text);
        state
            .messages
            .push(MessageItem::Error {
                message: "No model configured. Press Ctrl+O or type /onboard to set up a model."
                    .to_string(),
                recoverable: true,
            });
        state.input_right_info = String::new();
        return vec![];
    }

    // Model is configured — proceed with agent
    state.agent_running = true;
    add_user_and_placeholder(state, &text);
    let agent_messages = to_agent_messages(&state.messages);
    vec![ChatCmd::SpawnAgent {
        messages: agent_messages,
    }]
}

fn add_user_message_only(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}

fn add_user_and_placeholder(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    state
        .messages
        .push(MessageItem::Assistant {
            text: String::new(),
            model: state.current_model.clone(),
            timestamp: current_timestamp(),
        });
    state.is_thinking = true;
    state.thinking_start = Some(Instant::now());
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
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
