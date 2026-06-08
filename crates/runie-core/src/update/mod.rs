use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

mod agent;
mod input;
mod queue;
mod slash;

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
            Event::CursorLeft => self.cursor_left(),
            Event::CursorRight => self.cursor_right(),
            Event::CursorStart => self.cursor_start(),
            Event::CursorEnd => self.cursor_end(),
            Event::DeleteWord => self.delete_word(),
            Event::DeleteToEnd => self.delete_to_end(),
            Event::DeleteToStart => self.delete_to_start(),
            Event::KillChar => self.kill_char(),
            Event::HistoryPrev => self.history_prev(),
            Event::HistoryNext => self.history_next(),
            Event::Submit => self.submit(),
            Event::ScrollUp => self.scroll = self.scroll.saturating_add(1),
            Event::ScrollDown => self.scroll = self.scroll.saturating_sub(1),
            Event::Quit => {}
            Event::Reset => *self = AppState::default(),
            Event::AgentThinking { id } => self.set_thinking(id),
            Event::AgentThoughtDone { id } => self.add_thought(id),
            Event::AgentToolStart { id, name } => self.start_tool(id, name),
            Event::AgentToolEnd { duration_secs, output } => self.end_tool(duration_secs, output),
            Event::AgentResponse { id, content } => self.append_response(id, content),
            Event::AgentTurnComplete { id, duration_secs } => self.complete_turn(id, duration_secs),
            Event::AgentDone { .. } => self.finish_turn(),
            Event::AgentError { id, message } => self.add_error(id, message),
            Event::SwitchModel { provider, model } => self.switch_model(provider, model),
            Event::FollowUp => self.queue_follow_up(),
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
}
