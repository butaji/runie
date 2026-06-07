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

pub(crate) fn strip_tool_markers(content: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TOOL:") {
            continue;
        }
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
            Event::AgentToolEnd { duration_secs, output } => self.end_tool(duration_secs, output),
            Event::AgentResponse { id, content } => self.append_response(id, content),
            Event::AgentTurnComplete { id, duration_secs } => self.complete_turn(id, duration_secs),
            Event::AgentDone { .. } => self.finish_turn(),
            Event::AgentError { id, message } => self.add_error(id, message),
            Event::SwitchModel { provider, model } => self.switch_model(provider, model),
            Event::FollowUp => self.queue_follow_up(),
            Event::Abort => self.abort_queue(),
            Event::SpawnAgent => {}
            Event::ToggleExpand => self.toggle_last_expand(),
        }
    }

    fn toggle_last_expand(&mut self) {
        if let Some(msg) = self.messages.iter().rfind(|m| {
            (m.role == Role::Thought) || (m.role == Role::Tool && !m.content.contains("Running"))
        }) {
            let id = msg.id.clone();
            if self.collapsed.contains(&id) {
                self.collapsed.remove(&id);
            } else {
                self.collapsed.insert(id);
            }
            self.messages_changed();
        }
    }

    pub fn hint_text(&self) -> String {
        let mut parts = Vec::new();
        parts.push("Ctrl+Shift+E=expand/collapse".to_string());
        if self.at_suggestions.is_some() {
            parts.push("Tab=cycle".to_string());
            parts.push("Enter=insert".to_string());
            parts.push("Esc=close".to_string());
        } else if self.turn_active {
            parts.push("Enter=steer".to_string());
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=abort".to_string());
        } else if !self.input.is_empty() {
            parts.push("Enter=send".to_string());
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=clear".to_string());
        } else {
            parts.push("Alt+Enter=follow-up".to_string());
            parts.push("Esc=clear".to_string());
        }
        parts.push("Ctrl+C=quit".to_string());
        parts.join(" | ")
    }

    fn push_input(&mut self, c: char) {
        if c == '\t' {
            if self.input.contains('@') || self.at_suggestions.is_some() {
                self.cycle_at_suggestions();
            }
            return;
        }
        self.input.push(c);
        self.mark_dirty();
        if self.input.contains('@') {
            let query = self.input.split('@').last().unwrap_or("").to_string();
            let should_refresh = self.last_at_query.as_ref() != Some(&query);
            if should_refresh {
                self.last_at_query = Some(query);
                self.refresh_at_suggestions();
            }
        } else {
            self.at_suggestions = None;
            self.at_selected = None;
            self.last_at_query = None;
        }
    }

    fn refresh_at_suggestions(&mut self) {
        let mut suggestions = crate::file_refs::complete_at_ref(&self.input, ".", 10);
        if suggestions.is_empty() {
            suggestions = crate::file_refs::find_files("", ".", 10);
        }
        if suggestions.is_empty() {
            self.at_suggestions = None;
            self.at_selected = None;
            return;
        }
        self.at_selected = self.at_selected.map(|i| i.min(suggestions.len() - 1)).or(Some(0));
        self.at_suggestions = Some(suggestions);
        self.mark_dirty();
    }

    fn pop_input(&mut self) {
        self.input.pop();
        self.mark_dirty();
    }

    fn submit(&mut self) {
        if self.at_suggestions.is_some() {
            self.insert_at_suggestion();
            return;
        }
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        if content.is_empty() {
            return;
        }
        if let Some(response) = self.handle_slash(&content) {
            self.add_system_msg(response);
            return;
        }
        if self.turn_active {
            self.message_queue.push(crate::model::QueuedMessage {
                content,
                kind: crate::model::QueuedMessageKind::Steering,
            });
            self.mark_dirty();
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
        self.messages_changed();
    }

    fn set_thinking(&mut self, id: String) {
        self.streaming = true;
        self.current_request_id = Some(id);
        self.thinking_started_at = Some(std::time::Instant::now());
        self.turn_active = true;
        self.current_action = Some("Thinking".to_string());
        self.turn_started_at.get_or_insert_with(std::time::Instant::now);
        self.messages_changed();
    }

    fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.current_action = None;
        self.thinking_started_at = None;
        let mut insert_idx = self.messages.len();
        let thought_content = if let Some(idx) = self.messages.iter().position(|m| m.role == Role::Assistant && m.id == id) {
            let assistant = &self.messages[idx];
            let stripped = strip_tool_markers(&assistant.content);
            let has_tools = stripped != assistant.content;
            if has_tools && !stripped.trim().is_empty() {
                self.messages.remove(idx);
                insert_idx = idx;
                format!("{}\n{}", thought_with_time(duration), stripped)
            } else {
                insert_idx = idx;
                thought_with_time(duration)
            }
        } else {
            thought_with_time(duration)
        };
        let thought = ChatMessage {
            role: Role::Thought,
            content: thought_content,
            timestamp: now(),
            id: id.clone(),
        };
        self.messages.insert(insert_idx, thought);
        self.messages_changed();
    }

    fn start_tool(&mut self, id: String, name: String) {
        self.current_request_id = Some(id.clone());
        self.current_tool_name = Some(name.clone());
        self.tool_started_at = Some(std::time::Instant::now());
        self.intermediate_step_count += 1;
        self.current_action = Some(format!("Running {}", name));
        self.last_tool_index = Some(self.messages.len());
        self.messages.push(ChatMessage {
            role: Role::Tool,
            content: tool_running(&name),
            timestamp: now(),
            id,
        });
        self.messages_changed();
    }

    fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.current_action = None;
        self.tool_started_at = None;
        if let Some(name) = self.current_tool_name.take() {
            if let Some(idx) = self.last_tool_index.take() {
                if let Some(last) = self.messages.get_mut(idx) {
                    if last.role == Role::Tool {
                        last.content = if output.trim().is_empty() {
                            tool_done(&name, duration_secs)
                        } else {
                            format!("{}\n{}", tool_done(&name, duration_secs), output)
                        };
                        last.timestamp = now();
                    }
                }
            }
        }
        self.messages_changed();
    }

    fn append_response(&mut self, id: String, content: String) {
        if content.is_empty() {
            if let Some(last) = self.messages.last_mut() {
                if last.role == Role::Assistant && last.id == id {
                    last.timestamp = now();
                }
            }
            self.messages_changed();
            return;
        }
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant && last.id == id {
                last.content.push_str(&content);
                last.timestamp = now();
                self.messages_changed();
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
        self.messages_changed();
    }

    fn complete_turn(&mut self, id: String, duration_secs: f64) {
        self.messages.push(ChatMessage {
            role: Role::TurnComplete,
            content: format!("Turn completed in {:.1}s", duration_secs),
            timestamp: now(),
            id,
        });
        self.messages_changed();
        self.turn_started_at = None;
    }

    fn finish_turn(&mut self) {
        for msg in self.messages.iter_mut() {
            if msg.role == Role::Assistant {
                msg.content = strip_tool_markers(&msg.content);
            }
        }
        self.messages.retain(|msg| {
            !(msg.role == Role::Assistant && msg.content.trim().is_empty())
        });
        self.current_request_id = None;
        self.current_tool_name = None;
        self.intermediate_step_count = 0;
        self.turn_active = false;
        self.turn_started_at = None;
        self.inflight = self.inflight.saturating_sub(1);
        self.deliver_queued();
        if self.inflight == 0 && self.request_queue.is_empty() {
            self.streaming = false;
            self.thinking_started_at = None;
        }
        self.messages_changed();
    }

    fn add_error(&mut self, id: String, message: String) {
        self.streaming = false;
        self.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
        });
        self.messages_changed();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        self.current_provider = provider.clone();
        self.current_model = model.clone();
        self.add_system_msg(format!("Switched to {}/{}", provider, model));
    }

    fn handle_slash(&mut self, content: &str) -> Option<String> {
        match content {
            "/reset" => {
                *self = AppState::default();
                Some("State cleared.".to_string())
            }
            "/help" => Some(self.help_text()),
            "/model" => Some(self.model_usage()),
            "/save" => Some("Usage: /save name".to_string()),
            "/load" => Some("Usage: /load name".to_string()),
            "/delete" => Some("Usage: /delete name".to_string()),
            "/sessions" => Some(self.sessions_list()),
            "/compact" => Some(self.handle_compact(None)),
            _ => self.handle_slash_with_arg(content),
        }
    }

    fn handle_slash_with_arg(&mut self, content: &str) -> Option<String> {
        if let Some(rest) = content.strip_prefix("/model ") {
            return Some(self.handle_model(rest));
        }
        if let Some(name) = content.strip_prefix("/save ") {
            return Some(self.handle_save(name));
        }
        if let Some(name) = content.strip_prefix("/load ") {
            return Some(self.handle_load(name));
        }
        if let Some(name) = content.strip_prefix("/delete ") {
            return Some(self.handle_delete(name));
        }
        if let Some(rest) = content.strip_prefix("/compact ") {
            return Some(self.handle_compact(Some(rest)));
        }
        None
    }

    fn model_usage(&self) -> String {
        format!(
            "Current model: {}/{}. Usage: /model provider/model or /model model",
            self.current_provider, self.current_model
        )
    }

    fn handle_model(&mut self, rest: &str) -> String {
        let rest = rest.trim();
        if rest.is_empty() {
            return self.model_usage();
        }
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        match parts.len() {
            2 => {
                self.current_provider = parts[0].to_string();
                self.current_model = parts[1].to_string();
                format!("Switched to {}/{}", self.current_provider, self.current_model)
            }
            1 => {
                self.current_model = parts[0].to_string();
                format!("Switched to {}/{}", self.current_provider, self.current_model)
            }
            _ => self.model_usage(),
        }
    }

    fn handle_save(&self, name: &str) -> String {
        let session = crate::session::Session {
            name: name.to_string(),
            created_at: now(),
            updated_at: now(),
            messages: self.messages.clone(),
            provider: self.current_provider.clone(),
            model: self.current_model.clone(),
        };
        match crate::session::save(name, &session) {
            Ok(()) => format!("Session '{}' saved.", name),
            Err(e) => format!("Could not save '{}': {}", name, e),
        }
    }

    fn handle_load(&mut self, name: &str) -> String {
        match crate::session::load(name) {
            Ok(session) => {
                self.messages = session.messages;
                self.current_provider = session.provider;
                self.current_model = session.model;
                self.messages_changed();
                format!("Session '{}' loaded.", name)
            }
            Err(_) => format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ),
        }
    }

    fn handle_delete(&self, name: &str) -> String {
        match crate::session::delete(name) {
            Ok(()) => format!("Session '{}' deleted.", name),
            Err(_) => format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ),
        }
    }

    fn sessions_list(&self) -> String {
        match crate::session::list() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    "No saved sessions. Use /save name to create one.".to_string()
                } else {
                    format!("Saved sessions:\n{}", sessions.join("\n"))
                }
            }
            Err(e) => format!("Could not list sessions: {}", e),
        }
    }

    fn handle_compact(&mut self, custom: Option<&str>) -> String {
        let keep = 2000usize;
        let msg = self.compact(keep);
        if let Some(instruction) = custom {
            format!("{} (focus: {})", msg, instruction)
        } else {
            msg
        }
    }

    fn help_text(&self) -> String {
        format!(
            "Commands:\n\
            /model [provider/model|model] — switch model (current: {}/{})\n\
            /save name — save current session\n\
            /load name — load a saved session\n\
            /sessions — list saved sessions\n\
            /delete name — delete a saved session\n\
            /compact [prompt] — compact older messages\n\
            /reset — clear all state\n\
            /help — show this help\n\
            Enter — send | Alt+Enter — follow-up | Esc — abort | Ctrl+S — steer",
            self.current_provider, self.current_model
        )
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

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.request_queue.pop_front()
    }

    fn queue_follow_up(&mut self) {
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        if content.is_empty() {
            return;
        }
        self.message_queue.push(crate::model::QueuedMessage {
            content,
            kind: crate::model::QueuedMessageKind::FollowUp,
        });
        self.mark_dirty();
    }

    fn abort_queue(&mut self) {
        if self.at_suggestions.take().is_some() {
            self.at_selected = None;
            self.last_at_query = None;
            self.mark_dirty();
            return;
        }
        for msg in self.message_queue.drain(..).rev() {
            if !self.input.is_empty() {
                self.input.push('\n');
            }
            self.input.push_str(&msg.content);
        }
        self.mark_dirty();
    }

    fn cycle_at_suggestions(&mut self) {
        let suggestions = match self.at_suggestions.as_mut() {
            Some(s) => s,
            None => {
                self.refresh_at_suggestions();
                return;
            }
        };
        let idx = self.at_selected.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
        self.at_selected = Some(idx);
        self.mark_dirty();
    }

    fn insert_at_suggestion(&mut self) {
        if let Some(idx) = self.at_selected {
            if let Some(suggestions) = self.at_suggestions.take() {
                if let Some(selected) = suggestions.get(idx) {
                    self.input = crate::file_refs::insert_at_ref(&self.input, selected);
                }
            }
            self.at_selected = None;
            self.last_at_query = None;
            self.mark_dirty();
        }
    }

    fn deliver_queued(&mut self) {
        if self.message_queue.is_empty() {
            return;
        }
        let steering: Vec<_> = self.message_queue.iter().enumerate()
            .filter(|(_, m)| m.kind == crate::model::QueuedMessageKind::Steering)
            .map(|(i, _)| i)
            .collect();
        if !steering.is_empty() {
            let idx = steering[0];
            let msg = self.message_queue.remove(idx);
            let id = self.next_id();
            self.messages.push(ChatMessage {
                role: Role::User,
                content: msg.content.clone(),
                timestamp: now(),
                id: id.clone(),
            });
            self.request_queue.push_back((msg.content, id));
            self.messages_changed();
            return;
        }
        let follow_up: Vec<_> = self.message_queue.iter().enumerate()
            .filter(|(_, m)| m.kind == crate::model::QueuedMessageKind::FollowUp)
            .map(|(i, _)| i)
            .collect();
        if !follow_up.is_empty() {
            let idx = follow_up[0];
            let msg = self.message_queue.remove(idx);
            let id = self.next_id();
            self.messages.push(ChatMessage {
                role: Role::User,
                content: msg.content.clone(),
                timestamp: now(),
                id: id.clone(),
            });
            self.request_queue.push_back((msg.content, id));
            self.messages_changed();
        }
    }
}
