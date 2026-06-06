//! Update - State Transitions
use crate::labels::{thought_with_time, tool_running, tool_done};
use crate::model::{AppState, ChatMessage};
use crate::Event;

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

pub fn update(state: AppState, event: Event) -> AppState {
    match event {
        Event::Input(c) => state.push_input(c),
        Event::Backspace => state.pop_input(),
        Event::Submit => state.submit().marking_dirty(),
        Event::ScrollUp => state.scroll_up(),
        Event::ScrollDown => state.scroll_down(),
        Event::Quit => state,
        Event::Reset => AppState::default().marking_dirty(),
        Event::AgentThinking { id } => state.agent_thinking(id),
        Event::AgentThoughtDone { id } => state.agent_thought_done(id),
        Event::AgentToolStart { id, name } => state.agent_tool_start(id, name),
        Event::AgentToolEnd { duration_secs } => state.agent_tool_end(duration_secs),
        Event::AgentResponse { id, content } => state.agent_response(id, content),
        Event::AgentTurnComplete { id, duration_secs } => {
            state.agent_turn_complete(id, duration_secs)
        }
        Event::AgentDone { id: _ } => state.agent_done(),
        Event::AgentError { id, message } => state.agent_error(id, message),
        Event::SpawnAgent => state,
    }
}

trait DirtyMark {
    fn marking_dirty(self) -> Self;
}

impl DirtyMark for AppState {
    fn marking_dirty(mut self) -> Self {
        self.mark_dirty();
        self
    }
}

impl AppState {
    fn agent_thinking(mut self, id: String) -> Self {
        self.streaming = true;
        self.current_request_id = Some(id);
        self.thinking_started_at = Some(std::time::Instant::now());
        self.turn_active = true;
        self.current_action = Some("Thinking".to_string());
        if self.turn_started_at.is_none() {
            self.turn_started_at = Some(std::time::Instant::now());
        }
        self.mark_dirty();
        self
    }

    fn agent_thought_done(mut self, id: String) -> Self {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.current_action = None;
        self.thinking_started_at = None;
        self.messages.push(ChatMessage {
            role: "thought".into(),
            content: thought_with_time(duration),
            timestamp: now(),
            id,
        });
        self.mark_dirty();
        self
    }

    fn agent_tool_start(mut self, id: String, name: String) -> Self {
        self.current_request_id = Some(id.clone());
        self.current_tool_name = Some(name.clone());
        self.tool_started_at = Some(std::time::Instant::now());
        self.has_intermediate_steps = true;
        self.current_action = Some(format!("Running {}", name));
        self.messages.push(ChatMessage {
            role: "tool".into(),
            content: tool_running(&name),
            timestamp: now(),
            id,
        });
        self.mark_dirty();
        self
    }

    fn agent_tool_end(mut self, duration_secs: f64) -> Self {
        self.current_action = None;
        self.tool_started_at = None;
        if let Some(name) = self.current_tool_name.take() {
            if let Some(last) = self.messages.iter_mut().rev().find(|m| m.role == "tool") {
                last.content = tool_done(&name, duration_secs);
            }
        }
        self.mark_dirty();
        self
    }

    fn agent_response(mut self, id: String, content: String) -> Self {
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" && last.id == id {
                last.content.push_str(&content);
                self.mark_dirty();
                return self;
            }
        }
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content,
            timestamp: now(),
            id: id.clone(),
        });
        self.current_request_id = Some(id);
        self.mark_dirty();
        self
    }

    fn agent_turn_complete(mut self, id: String, duration_secs: f64) -> Self {
        if self.has_intermediate_steps {
            self.messages.push(ChatMessage {
                role: "turn_complete".into(),
                content: format!("Turn completed in {:.1}s", duration_secs),
                timestamp: now(),
                id,
            });
            self.mark_dirty();
        }
        self.turn_started_at = None;
        self
    }

    fn agent_done(mut self) -> Self {
        self.current_request_id = None;
        self.current_tool_name = None;
        self.has_intermediate_steps = false;
        self.turn_active = false;
        self.turn_started_at = None;
        if self.request_queue.is_empty() {
            self.streaming = false;
            self.thinking_started_at = None;
        } else {
            self.streaming = true;
        }
        self
    }

    fn agent_error(mut self, id: String, message: String) -> Self {
        self.streaming = false;
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
        });
        self.mark_dirty();
        self
    }

    fn push_input(mut self, c: char) -> Self {
        self.input.push(c);
        self
    }

    fn pop_input(mut self) -> Self {
        self.input.pop();
        self
    }

    fn submit(mut self) -> Self {
        if self.input.is_empty() {
            return self;
        }
        let content = self.input.trim().to_string();
        self.input.clear();
        if content == "/reset" {
            return AppState::default();
        }
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push((content, id));
        self
    }

    pub fn peek_queue(&self) -> Option<(String, String)> {
        self.request_queue.first().cloned()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        if self.request_queue.is_empty() {
            None
        } else {
            Some(self.request_queue.remove(0))
        }
    }

    fn scroll_up(mut self) -> Self {
        self.scroll = self.scroll.saturating_add(1);
        self
    }

    fn scroll_down(mut self) -> Self {
        self.scroll = self.scroll.saturating_sub(1);
        self
    }
}
