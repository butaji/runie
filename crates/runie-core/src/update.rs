//! Update - State Transitions (mutable borrow pattern, no cloning)
use crate::labels::{thought_with_time, tool_running, tool_done};
use crate::model::{AppState, ChatMessage, Role};
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
            Event::SwitchModel { provider, model } => self.switch_model(provider, model),
            Event::ShowHelp => self.show_help(),
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
        if content == "/help" {
            self.show_help();
            return;
        }
        if let Some(rest) = content.strip_prefix("/model ") {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                self.switch_model(parts[0].to_string(), parts[1].to_string());
            } else {
                self.add_system_msg("Usage: /model provider/model".to_string());
            }
            return;
        }
        if content == "/sessions" {
            match crate::session::list() {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        self.add_system_msg("No saved sessions.".to_string());
                    } else {
                        self.add_system_msg(format!("Sessions:\n{}", sessions.join("\n")));
                    }
                }
                Err(e) => self.add_system_msg(format!("Error: {}", e)),
            }
            return;
        }
        if let Some(name) = content.strip_prefix("/save ") {
            let session = crate::session::Session {
                name: name.to_string(),
                created_at: now(),
                updated_at: now(),
                messages: self.messages.clone(),
                provider: self.current_provider.clone(),
                model: self.current_model.clone(),
            };
            match crate::session::save(name, &session) {
                Ok(()) => self.add_system_msg(format!("Session '{}' saved.", name)),
                Err(e) => self.add_system_msg(format!("Error saving: {}", e)),
            }
            return;
        }
        if let Some(name) = content.strip_prefix("/load ") {
            match crate::session::load(name) {
                Ok(session) => {
                    self.messages = session.messages;
                    self.current_provider = session.provider;
                    self.current_model = session.model;
                    self.mark_dirty();
                    self.add_system_msg(format!("Session '{}' loaded.", name));
                }
                Err(e) => self.add_system_msg(format!("Error loading: {}", e)),
            }
            return;
        }
        if let Some(name) = content.strip_prefix("/delete ") {
            match crate::session::delete(name) {
                Ok(()) => self.add_system_msg(format!("Session '{}' deleted.", name)),
                Err(e) => self.add_system_msg(format!("Error deleting: {}", e)),
            }
            return;
        }
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push_back((content, id));
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
            role: Role::Thought,
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
        self.last_tool_index = Some(self.messages.len());
        self.messages.push(ChatMessage {
            role: Role::Tool,
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
            if let Some(idx) = self.last_tool_index.take() {
                if let Some(last) = self.messages.get_mut(idx) {
                    if last.role == Role::Tool {
                        last.content = tool_done(&name, duration_secs);
                    }
                }
            }
        }
        self.mark_dirty();
    }

    fn append_response(&mut self, id: String, content: String) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant && last.id == id {
                last.content.push_str(&content);
                self.mark_dirty();
                return;
            }
        }
        self.messages.push(ChatMessage {
            role: Role::Assistant,
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
                role: Role::TurnComplete,
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
        self.inflight = self.inflight.saturating_sub(1);
        if self.inflight == 0 && self.request_queue.is_empty() {
            self.streaming = false;
            self.thinking_started_at = None;
        }
    }

    fn add_error(&mut self, id: String, message: String) {
        self.streaming = false;
        self.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
        });
        self.mark_dirty();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        self.current_provider = provider.clone();
        self.current_model = model.clone();
        self.add_system_msg(format!("Switched to {}/{}", provider, model));
    }

    fn show_help(&mut self) {
        self.add_system_msg(
            "Commands:\n/model provider/model — switch model\n/save name — save session\n/load name — load session\n/sessions — list sessions\n/delete name — delete session\n/reset — clear state\n/help — show this".to_string(),
        );
    }

    fn add_system_msg(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: Role::System,
            content,
            timestamp: now(),
            id: "system".to_string(),
        });
        self.mark_dirty();
    }

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.request_queue.pop_front()
    }
}
