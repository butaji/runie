use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

mod agent;
mod bash;
mod input;
mod line_nav;
mod queue;

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub(crate) fn strip_tool_markers(content: &str) -> String {
    if let Some(pos) = content.find("TOOL:") {
        let before = &content[..pos];
        return before.trim_end().to_string();
    }
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if val.get("name").is_some() && val.get("arguments").is_some() {
                    continue;
                }
            }
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

pub(crate) fn content_has_tool_markers(content: &str) -> bool {
    if content.contains("TOOL:") {
        return true;
    }
    content.lines().any(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            return false;
        }
        serde_json::from_str::<serde_json::Value>(trimmed)
            .map(|v| v.get("name").is_some() && v.get("arguments").is_some())
            .unwrap_or(false)
    })
}

impl AppState {
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Input(c) => self.push_input(c),
            Event::Backspace => self.pop_input(),
            Event::Newline => self.insert_newline(),
            Event::CursorLeft => self.cursor_left(),
            Event::CursorRight => self.cursor_right(),
            Event::CursorStart => self.cursor_start(),
            Event::CursorEnd => self.cursor_end(),
            Event::DeleteWord => self.delete_word(),
            Event::DeleteToEnd => self.delete_to_end(),
            Event::DeleteToStart => self.delete_to_start(),
            Event::KillChar => self.kill_char(),
            Event::HistoryPrev => {
                if self.input.contains('\n') {
                    self.move_cursor_up();
                } else {
                    self.history_prev();
                }
            }
            Event::HistoryNext => {
                if self.input.contains('\n') {
                    self.move_cursor_down();
                } else {
                    self.history_next();
                }
            }
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            Event::CursorWordLeft => self.cursor_word_left(),
            Event::CursorWordRight => self.cursor_word_right(),
            Event::Paste(text) => self.paste(&text),
            Event::Submit => self.submit(),
            Event::ScrollUp => {
                if self.messages.is_empty() && !self.turn_active {
                    self.input_flash = 3;
                }
                self.scroll = self.scroll.saturating_add(1);
            }
            Event::ScrollDown => {
                if self.scroll == 0 {
                    self.input_flash = 3;
                }
                self.scroll = self.scroll.saturating_sub(1);
            }
            Event::Quit => self.should_quit = true,
            Event::Reset => *self = AppState::default(),
            Event::AgentThinking { id } => {
                self.set_thinking(id);
                self.ensure_turn_complete_last();
            }
            Event::AgentThoughtDone { id } => {
                self.add_thought(id);
                self.ensure_turn_complete_last();
            }
            Event::AgentToolStart { id, name } => {
                self.start_tool(id, name);
                self.ensure_turn_complete_last();
            }
            Event::AgentToolEnd { duration_secs, output } => {
                self.end_tool(duration_secs, output);
                self.ensure_turn_complete_last();
            }
            Event::AgentResponse { id, content } => {
                self.append_response(id, content);
                self.ensure_turn_complete_last();
            }
            Event::AgentTurnComplete { id, duration_secs } => {
                self.complete_turn(id, duration_secs);
                self.ensure_turn_complete_last();
            }
            Event::AgentDone { id } => self.finish_turn(id),
            Event::AgentError { id, message } => {
                self.add_error(id, message);
                self.ensure_turn_complete_last();
            }
            Event::SwitchModel { provider, model } => self.switch_model(provider, model),
            Event::SwitchTheme { name } => self.switch_theme(name),
            Event::FollowUp => self.queue_follow_up(),
            Event::Dequeue => self.dequeue(),
            Event::OpenExternalEditor => {}
            Event::ExternalEditorDone { content } => {
                self.input = content;
                self.cursor_pos = self.input.len();
                self.mark_dirty();
            }
            Event::Abort => self.abort_queue(),
            Event::SpawnAgent => {}
            Event::ToggleExpand => self.toggle_expand_all(),
        }
    }

    fn toggle_expand_all(&mut self) {
        self.all_collapsed = !self.all_collapsed;
        self.messages_changed();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        self.current_provider = provider.clone();
        self.current_model = model.clone();
        self.add_system_msg(format!("Switched to {}/{}", provider, model));
    }

    fn switch_theme(&mut self, name: String) {
        self.theme_name = name.clone();
        self.add_system_msg(format!("Theme switched to '{}'", name));
    }

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.request_queue.pop_front()
    }

    fn add_system_msg(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: Role::System,
            content,
            timestamp: now(),
            id: "system".to_string(),
        });
        self.messages_changed();
    }

    /// Move TurnComplete to the end of messages and bump its timestamp.
    /// Called after every agent event to ensure TurnComplete remains last.
    fn ensure_turn_complete_last(&mut self) {
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::TurnComplete) {
            let mut tc = self.messages.remove(idx);
            tc.timestamp = now();
            self.messages.push(tc);
            self.messages_changed();
        }
    }
}
