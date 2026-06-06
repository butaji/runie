//! Update - State Transitions (mutable borrow pattern, no cloning)
use crate::labels::{thought_with_time, tool_running, tool_done};
use crate::model::{AppState, ChatMessage};
use crate::Event;

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl AppState {
    /// Update state with event - uses mutable borrow, no cloning
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Input(c) => self.push_input(c),
            Event::Backspace => self.pop_input(),
            Event::Submit => self.submit(),
            Event::ScrollUp => self.scroll = self.scroll.saturating_add(1),
            Event::ScrollDown => self.scroll = self.scroll.saturating_sub(1),
            Event::Quit => {}
            Event::Reset => *self = AppState::default(),
            Event::AgentThinking { id } => self.set_thinking(id),
            Event::AgentThoughtDone { id } => self.add_thought(id),
            Event::AgentToolStart { id, name } => self.start_tool(id, name),
            Event::AgentToolEnd { duration_secs } => self.end_tool(duration_secs),
            Event::AgentResponse { id, content } => self.append_response(id, content),
            Event::AgentTurnComplete { id, duration_secs } => self.complete_turn(id, duration_secs),
            Event::AgentDone { .. } => self.finish_turn(),
            Event::AgentError { id, message } => self.add_error(id, message),
            Event::SpawnAgent => {}
        }
    }

    fn push_input(&mut self, c: char) {
        self.input.push(c);
    }

    fn pop_input(&mut self) {
        self.input.pop();
    }

    fn submit(&mut self) {
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        if content.is_empty() {
            return;
        }
        if content == "/reset" {
            *self = AppState::default();
            return;
        }
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push((content, id));
        self.mark_dirty();
    }

    fn set_thinking(&mut self, id: String) {
        self.streaming = true;
        self.current_request_id = Some(id);
        self.thinking_started_at = Some(std::time::Instant::now());
        self.turn_active = true;
        self.current_action = Some("Thinking".to_string());
        self.turn_started_at.get_or_insert_with(std::time::Instant::now);
        self.mark_dirty();
    }

    fn add_thought(&mut self, id: String) {
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
    }

    fn start_tool(&mut self, id: String, name: String) {
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
    }

    fn end_tool(&mut self, duration_secs: f64) {
        self.current_action = None;
        self.tool_started_at = None;
        if let Some(name) = self.current_tool_name.take() {
            if let Some(last) = self.messages.iter_mut().rev().find(|m| m.role == "tool") {
                last.content = tool_done(&name, duration_secs);
            }
        }
        self.mark_dirty();
    }

    fn append_response(&mut self, id: String, content: String) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" && last.id == id {
                last.content.push_str(&content);
                self.mark_dirty();
                return;
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
    }

    fn complete_turn(&mut self, id: String, duration_secs: f64) {
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
    }

    fn finish_turn(&mut self) {
        self.current_request_id = None;
        self.current_tool_name = None;
        self.has_intermediate_steps = false;
        self.turn_active = false;
        self.turn_started_at = None;
        if self.request_queue.is_empty() {
            self.streaming = false;
            self.thinking_started_at = None;
        }
    }

    fn add_error(&mut self, id: String, message: String) {
        self.streaming = false;
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
        });
        self.mark_dirty();
    }

    pub fn peek_queue(&self) -> Option<(String, String)> {
        self.request_queue.first().cloned()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        if !self.request_queue.is_empty() {
            Some(self.request_queue.remove(0))
        } else {
            None
        }
    }
}
