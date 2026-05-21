use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
use runie_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];

    // Log msg before applying (for time-travel debugging)
    if state.action_log.len() >= state.action_log_capacity {
        state.action_log.remove(0); // Remove oldest when at capacity
    }
    state.action_log.push(msg.clone());

    match msg {
        Msg::Quit => state.running = false,

        Msg::Submit => {
            let text = state.input_lines.join("\n");
            if !text.is_empty() {
                state.messages.push(MessageItem::User {
                    text: text.clone(),
                    model: Some("You".to_string()),
                    timestamp: None,
                });
                state.input_lines = vec![String::new()];
                state.cursor_col = 0;
                state.cursor_row = 0;
                cmds.push(Cmd::SpawnAgent {
                    messages: to_agent_messages(&state.messages),
                });
            }
        }

        Msg::InsertChar(c) => {
            if state.cursor_row < state.input_lines.len() {
                state.input_lines[state.cursor_row].insert(state.cursor_col, c);
                state.cursor_col += 1;
            }
        }

        Msg::Backspace => {
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

        Msg::InsertNewline => {
            if state.cursor_row < state.input_lines.len() {
                let remainder = state.input_lines[state.cursor_row].split_off(state.cursor_col);
                state.cursor_row += 1;
                state.cursor_col = 0;
                state.input_lines.insert(state.cursor_row, remainder);
            }
        }

        Msg::MoveCursorLeft => {
            if state.cursor_col > 0 {
                state.cursor_col -= 1;
            } else if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.input_lines[state.cursor_row].len();
            }
        }

        Msg::MoveCursorRight => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.cursor_col += 1;
            } else if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = 0;
            }
        }

        Msg::MoveCursorUp => {
            if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Msg::MoveCursorDown => {
            if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Msg::MoveCursorToStart => state.cursor_col = 0,

        Msg::MoveCursorToEnd => {
            state.cursor_col = state.input_lines[state.cursor_row].len();
        }

        Msg::DeleteForward => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.input_lines[state.cursor_row].remove(state.cursor_col);
            }
        }

        Msg::DeleteWordBackward => {
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

        Msg::DeleteToStart => {
            state.input_lines[state.cursor_row].drain(..state.cursor_col);
            state.cursor_col = 0;
        }

        Msg::ToggleSidebar => state.show_sidebar = !state.show_sidebar,

        Msg::OpenCommandPalette => {
            state.command_palette_open = true;
            state.mode = TuiMode::CommandPalette;
            state.command_palette_filter.clear();
            state.command_palette_selected = 0;
        }

        Msg::CloseModal => {
            state.mode = TuiMode::Chat;
            state.command_palette_open = false;
            state.permission_modal_tool = None;
        }

        Msg::ConfirmModal => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }

        Msg::AgentEvent(event) => {
            match event {
                AgentEvent::MessageStart { message } => {
                    state.agent_running = true;
                    state.current_model = Some(message.role.clone());
                    state.messages.push(MessageItem::Assistant {
                        text: String::new(),
                        model: state.current_model.clone(),
                        timestamp: None,
                    });
                }
                AgentEvent::MessageUpdate { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let new_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = new_text;
                        }
                    }
                }
                AgentEvent::MessageEnd { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let final_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = final_text;
                        }
                    }
                }
                AgentEvent::ToolExecutionStart { tool_call_id } => {
                    state.messages.push(MessageItem::ToolCall {
                        name: tool_call_id,
                        args: String::new(),
                        result: None,
                        is_error: false,
                    });
                }
                AgentEvent::ToolExecutionEnd { result, .. } => {
                    let result_text = result
                        .content
                        .iter()
                        .filter_map(|part| {
                            if let ContentPart::Text { text } = part {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let is_err = result.is_error;
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::ToolCall { ref mut result, ref mut is_error, .. } = last {
                            *result = Some(result_text);
                            *is_error = is_err;
                        }
                    }
                }
                AgentEvent::AgentEnd { .. } => {
                    state.agent_running = false;
                    state.current_model = None;
                }
                AgentEvent::Error { message } => {
                    state
                        .messages
                        .push(MessageItem::System { text: format!("Error: {}", message) });
                    state.agent_running = false;
                }
                AgentEvent::PermissionRequest { tool_name, tool_args, .. } => {
                    state.permission_modal_tool = Some(tool_name.clone());
                    state.permission_modal_args = Some(tool_args.clone());
                    state.permission_modal_desc =
                        Some(format!("Agent wants to execute '{}'", tool_name));
                    state.mode = TuiMode::Permission;
                }
                _ => {}
            }
        }

        Msg::PermissionConfirm => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Allow });
        }
        Msg::PermissionCancel => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Deny });
        }
        Msg::PermissionAlways => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::AllowAlways });
        }
        Msg::PermissionSkip => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Skip });
        }
        Msg::CommandPaletteFilter(c) => {
            state.command_palette_filter.push(c);
        }
        Msg::CommandPaletteBackspace => {
            state.command_palette_filter.pop();
        }
        Msg::CommandPaletteUp => {
            if state.command_palette_selected > 0 {
                state.command_palette_selected -= 1;
            }
        }
        Msg::CommandPaletteDown => {
            state.command_palette_selected += 1;
        }
        Msg::CommandPaletteConfirm => {
            state.command_palette_open = false;
            state.mode = TuiMode::Chat;
        }
        Msg::ScrollUp => {
            state.feed_scroll_offset = state.feed_scroll_offset.saturating_sub(1);
        }
        Msg::ScrollDown => {
            state.feed_scroll_offset += 1;
        }
    }

    cmds
}

// ─── event_to_msg ──────────────────────────────────────────────────────────────
// Convert crossterm events to Msg


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
