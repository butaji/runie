use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
use runie_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];
    log_action(state, &msg);

    match msg {
        Msg::Quit => state.running = false,
        Msg::Submit => cmds.extend(handle_submit(state)),
        Msg::InsertChar(c) => handle_insert_char(state, c),
        Msg::Backspace => handle_backspace(state),
        Msg::InsertNewline => handle_insert_newline(state),
        Msg::MoveCursorLeft | Msg::MoveCursorRight | Msg::MoveCursorUp | Msg::MoveCursorDown => {
            handle_cursor_move(state, &msg);
        }
        Msg::MoveCursorToStart => state.cursor_col = 0,
        Msg::MoveCursorToEnd => state.cursor_col = state.input_lines[state.cursor_row].len(),
        Msg::DeleteForward => handle_delete_forward(state),
        Msg::DeleteWordBackward => handle_delete_word_backward(state),
        Msg::DeleteToStart => handle_delete_to_start(state),
        Msg::ToggleSidebar => state.show_sidebar = !state.show_sidebar,
        Msg::OpenCommandPalette => open_palette(state),
        Msg::CloseModal | Msg::ConfirmModal => handle_close_modal(state),
        Msg::AgentEvent(event) => handle_agent_event(state, event),
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => {
            cmds.push(handle_permission_msg(state, msg));
        }
        Msg::CommandPaletteFilter(c) => state.command_palette_filter.push(c),
        Msg::CommandPaletteBackspace => { state.command_palette_filter.pop(); }
        Msg::CommandPaletteUp | Msg::CommandPaletteDown | Msg::CommandPaletteConfirm => {
            handle_palette_msg(state, msg);
        }
        Msg::ScrollUp => { state.feed_scroll_offset = state.feed_scroll_offset.saturating_sub(1); }
        Msg::ScrollDown => state.feed_scroll_offset += 1,
        Msg::Tick | Msg::CursorBlink => handle_anim(state, &msg),
        Msg::SlashCommand(cmd) => cmds.extend(handle_slash(state, cmd)),
        Msg::ToggleSessionTree => handle_tree(state),
        Msg::SessionTreeUp | Msg::SessionTreeDown => handle_tree_nav(state, &msg),
        Msg::SessionTreeConfirm => handle_tree_confirm(state),
    }

    cmds
}

fn log_action(state: &mut AppState, msg: &Msg) {
    if state.action_log.len() >= state.action_log_capacity {
        state.action_log.remove(0);
    }
    state.action_log.push(msg.clone());
}

fn open_palette(state: &mut AppState) {
    state.command_palette_open = true;
    state.mode = TuiMode::CommandPalette;
    state.command_palette_filter.clear();
    state.command_palette_selected = 0;
}

fn handle_submit(state: &mut AppState) -> Vec<Cmd> {
    let text = state.input_lines.join("\n");
    if text.is_empty() {
        return vec![];
    }
    state.messages.push(MessageItem::User {
        text: text.clone(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.input_lines = vec![String::new()];
    state.cursor_col = 0;
    state.cursor_row = 0;
    vec![Cmd::SpawnAgent { messages: to_agent_messages(&state.messages) }]
}

fn handle_insert_char(state: &mut AppState, c: char) {
    if state.cursor_row < state.input_lines.len() {
        state.input_lines[state.cursor_row].insert(state.cursor_col, c);
        state.cursor_col += 1;
    }
}

fn handle_backspace(state: &mut AppState) {
    if state.cursor_col > 0 {
        state.input_lines[state.cursor_row].remove(state.cursor_col - 1);
        state.cursor_col -= 1;
    } else if state.cursor_row > 0 {
        let line = state.input_lines.remove(state.cursor_row);
        state.cursor_row -= 1;
        state.cursor_col = state.input_lines[state.cursor_row].len();
        state.input_lines[state.cursor_row].push_str(&line);
    }
}

fn handle_insert_newline(state: &mut AppState) {
    if state.cursor_row < state.input_lines.len() {
        let remainder = state.input_lines[state.cursor_row].split_off(state.cursor_col);
        state.cursor_row += 1;
        state.cursor_col = 0;
        state.input_lines.insert(state.cursor_row, remainder);
    }
}

fn handle_move_left(state: &mut AppState) {
    if state.cursor_col > 0 {
        state.cursor_col -= 1;
    } else if state.cursor_row > 0 {
        state.cursor_row -= 1;
        state.cursor_col = state.input_lines[state.cursor_row].len();
    }
}

fn handle_move_right(state: &mut AppState) {
    if state.cursor_col < state.input_lines[state.cursor_row].len() {
        state.cursor_col += 1;
    } else if state.cursor_row + 1 < state.input_lines.len() {
        state.cursor_row += 1;
        state.cursor_col = 0;
    }
}

fn handle_move_up(state: &mut AppState) {
    if state.cursor_row > 0 {
        state.cursor_row -= 1;
        state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
    }
}

fn handle_move_down(state: &mut AppState) {
    if state.cursor_row + 1 < state.input_lines.len() {
        state.cursor_row += 1;
        state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
    }
}

fn handle_delete_forward(state: &mut AppState) {
    if state.cursor_col < state.input_lines[state.cursor_row].len() {
        state.input_lines[state.cursor_row].remove(state.cursor_col);
    }
}

fn handle_delete_word_backward(state: &mut AppState) {
    let line = &state.input_lines[state.cursor_row];
    let before = &line[..state.cursor_col];
    if let Some(pos) = before.rfind(|c: char| c.is_whitespace()) {
        state.input_lines[state.cursor_row].drain(pos..state.cursor_col);
        state.cursor_col = pos;
    } else {
        state.input_lines[state.cursor_row].clear();
        state.cursor_col = 0;
    }
}

fn handle_delete_to_start(state: &mut AppState) {
    state.input_lines[state.cursor_row].drain(..state.cursor_col);
    state.cursor_col = 0;
}

fn handle_close_modal(state: &mut AppState) {
    state.mode = TuiMode::Chat;
    state.command_palette_open = false;
    state.permission_modal_tool = None;
    state.diff_viewer = None;
    state.session_tree.hide();
}

fn handle_permission(state: &mut AppState, decision: PermissionDecision) -> Cmd {
    state.mode = TuiMode::Chat;
    state.permission_modal_tool = None;
    Cmd::SendPermission { decision }
}

fn handle_agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::MessageStart { message } => on_message_start(state, message),
        AgentEvent::MessageUpdate { message } => on_message_update(state, message),
        AgentEvent::MessageEnd { message } => on_message_end(state, message),
        AgentEvent::ToolExecutionStart { tool_call_id } => on_tool_start(state, tool_call_id),
        AgentEvent::ToolExecutionEnd { result, .. } => on_tool_end(state, result),
        AgentEvent::AgentEnd { .. } => on_agent_end(state),
        AgentEvent::Error { message } => on_agent_error(state, message),
        AgentEvent::PermissionRequest { tool_name, tool_args, .. } => on_permission_request(state, tool_name, tool_args),
        _ => {}
    }
}

fn on_message_start(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    state.agent_running = true;
    state.current_model = Some(message.role.clone());
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: state.current_model.clone(),
        timestamp: None,
    });
}

fn on_message_update(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    update_last_assistant(state, &message.content);
}

fn on_message_end(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    update_last_assistant(state, &message.content);
}

fn update_last_assistant(state: &mut AppState, content: &[ContentPart]) {
    if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
        *text = extract_text_content(content);
    }
}

fn on_tool_start(state: &mut AppState, tool_call_id: String) {
    state.messages.push(MessageItem::ToolCall {
        name: tool_call_id,
        args: String::new(),
        result: None,
        is_error: false,
    });
}

fn on_tool_end(state: &mut AppState, tool_result: runie_agent::events::ToolResult) {
    let text = extract_text_content(&tool_result.content);
    if let Some(MessageItem::ToolCall { ref mut result, ref mut is_error, .. }) = state.messages.last_mut() {
        *result = Some(text);
        *is_error = tool_result.is_error;
    }
}

fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    state.current_model = None;
}

fn on_agent_error(state: &mut AppState, message: String) {
    state.messages.push(MessageItem::System { text: format!("Error: {}", message) });
    state.agent_running = false;
}

fn on_permission_request(state: &mut AppState, tool_name: String, tool_args: String) {
    state.permission_modal_tool = Some(tool_name.clone());
    state.permission_modal_args = Some(tool_args.clone());
    state.permission_modal_desc = Some(format!("Agent wants to execute '{}'", tool_name));
    state.mode = TuiMode::Permission;
}

fn extract_text_content(parts: &[ContentPart]) -> String {
    parts.iter()
        .filter_map(|part| {
            if let ContentPart::Text { text } = part {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn to_agent_messages(items: &[MessageItem]) -> Vec<AgentMessage> {
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        _ => None,
    }).collect()
}

fn handle_permission_msg(state: &mut AppState, msg: Msg) -> Cmd {
    let decision = match msg {
        Msg::PermissionConfirm => PermissionDecision::Allow,
        Msg::PermissionCancel => PermissionDecision::Deny,
        Msg::PermissionAlways => PermissionDecision::AllowAlways,
        Msg::PermissionSkip => PermissionDecision::Skip,
        _ => PermissionDecision::Allow,
    };
    handle_permission(state, decision)
}

fn handle_anim(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::Tick => {
            state.animation.braille_frame = (state.animation.braille_frame + 1) % 8;
            state.animation.last_tick = std::time::Instant::now();
        }
        Msg::CursorBlink => {
            state.animation.streaming_cursor_visible = !state.animation.streaming_cursor_visible;
            state.animation.last_cursor_blink = std::time::Instant::now();
        }
        _ => {}
    }
}

fn handle_cursor_move(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::MoveCursorLeft => handle_move_left(state),
        Msg::MoveCursorRight => handle_move_right(state),
        Msg::MoveCursorUp => handle_move_up(state),
        Msg::MoveCursorDown => handle_move_down(state),
        _ => {}
    }
}

fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<Cmd> {
    let mut cmds = vec![];
    match cmd {
        runie_core::slash_command::SlashCommand::New => {
            state.messages.clear();
            state.feed_scroll_offset = 0;
            state.messages.push(MessageItem::System { text: "New session started".to_string() });
        }
        runie_core::slash_command::SlashCommand::Clear => {
            state.messages.clear();
            state.feed_scroll_offset = 0;
        }
        runie_core::slash_command::SlashCommand::Model(model) => {
            state.current_model = Some(model.clone());
            state.messages.push(MessageItem::System { text: format!("Model switched to {}", model) });
        }
        runie_core::slash_command::SlashCommand::Compact => {
            state.messages.push(MessageItem::System { text: "Session compaction not yet implemented".to_string() });
        }
        runie_core::slash_command::SlashCommand::Save(name) => cmds.push(Cmd::SaveSession { name }),
        runie_core::slash_command::SlashCommand::Load(name) => cmds.push(Cmd::LoadSession { name }),
        runie_core::slash_command::SlashCommand::Tree => handle_tree(state),
        runie_core::slash_command::SlashCommand::Fork => state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() }),
        runie_core::slash_command::SlashCommand::Quit => state.running = false,
        runie_core::slash_command::SlashCommand::Help => {
            state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() });
        }
        runie_core::slash_command::SlashCommand::Unknown(cmd) => {
            state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) });
        }
    }
    cmds
}

fn handle_tree(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}

fn handle_tree_nav(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::SessionTreeUp => state.session_tree.move_up(),
        Msg::SessionTreeDown => state.session_tree.move_down(),
        _ => {}
    }
}

fn handle_tree_confirm(state: &mut AppState) {
    if let Some(id) = state.session_tree.get_selected_id() {
        state.messages.push(MessageItem::System { text: format!("Jumped to message: {}", &id[..8]) });
    }
    state.session_tree.hide();
    state.mode = TuiMode::Chat;
}

fn handle_palette_msg(state: &mut AppState, msg: Msg) {
    match msg {
        Msg::CommandPaletteUp => {
            if state.command_palette_selected > 0 {
                state.command_palette_selected -= 1;
            }
        }
        Msg::CommandPaletteDown => state.command_palette_selected += 1,
        Msg::CommandPaletteConfirm => handle_close_modal(state),
        _ => {}
    }
}
