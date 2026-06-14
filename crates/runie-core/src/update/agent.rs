use super::{content_has_tool_markers, now, strip_tool_markers};
use crate::labels::{thought_with_time, tool_done, tool_running};
use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

pub fn agent_event(state: &mut AppState, event: Event) {
    match event {
        Event::AgentThinking { id } => {
            state.set_thinking(id);
            state.ensure_turn_complete_last();
        }
        Event::AgentThoughtDone { id } => {
            state.add_thought(id);
            state.ensure_turn_complete_last();
        }
        Event::AgentToolStart { id, name } => {
            state.start_tool(id, name);
            state.ensure_turn_complete_last();
        }
        Event::AgentToolEnd {
            duration_secs,
            output,
        } => {
            state.end_tool(duration_secs, output);
            state.ensure_turn_complete_last();
        }
        Event::AgentResponse { id, content } => {
            state.append_response(id, content);
            state.ensure_turn_complete_last();
        }
        Event::AgentTurnComplete { id, duration_secs } => {
            state.complete_turn(id, duration_secs);
            state.ensure_turn_complete_last();
        }
        Event::AgentDone { id } => state.finish_turn(id),
        Event::AgentError { id, message } => {
            state.add_error(id, message);
            state.ensure_turn_complete_last();
        }
        _ => {}
    }
}

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        self.agent.streaming = true;
        self.agent.current_request_id = Some(id);
        self.agent.thinking_started_at = Some(std::time::Instant::now());
        self.agent.turn_active = true;
        self.agent.current_action = Some("Thinking".to_string());
        self.agent
            .turn_started_at
            .get_or_insert_with(std::time::Instant::now);
        // Init speed tracking for this turn
        self.agent.turn_tokens_out = 0;
        self.agent.last_speed_update = Some(std::time::Instant::now());
        self.agent.tokens_at_last_speed = self.agent.tokens_out;
        self.agent.speed_tps = 0.0;
        // Keep existing rolling window - it auto-evicts to 1000 tokens
        self.agent.speed_window.record(self.agent.tokens_out);
        self.messages_changed();
    }

    pub(crate) fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.agent.current_action = None;
        self.agent.thinking_started_at = None;
        let mut insert_idx = self.session.messages.len();
        let thought_content = if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
        {
            let assistant = &self.session.messages[idx];
            let has_tools = content_has_tool_markers(&assistant.content);
            let stripped = strip_tool_markers(&assistant.content);
            if has_tools && !stripped.trim().is_empty() {
                self.session.messages.remove(idx);
                insert_idx = idx;
                format!("{}\n{}", thought_with_time(duration), stripped)
            } else {
                insert_idx = idx;
                thought_with_time(duration)
            }
        } else {
            thought_with_time(duration)
        };
        let thought_id = format!("{}#thought.{}", id, self.agent.thought_seq);
        self.agent.thought_seq += 1;
        let thought = ChatMessage {
            role: Role::Thought,
            content: thought_content,
            timestamp: now(),
            id: thought_id,
            ..Default::default()
        };
        self.session.messages.insert(insert_idx, thought);
        self.messages_changed();
    }

    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        self.agent.current_request_id = Some(id.clone());
        self.agent.current_tool_name = Some(name.clone());
        self.agent.tool_started_at = Some(std::time::Instant::now());
        self.agent.intermediate_step_count += 1;
        self.agent.current_action = Some(format!("Running {}", name));
        let tool_id = format!("tool.{}.{}", id, self.agent.intermediate_step_count);
        self.session.messages.push(ChatMessage {
            role: Role::Tool,
            content: tool_running(&name),
            timestamp: now(),
            id: tool_id,
            ..Default::default()
        });
        self.config.telemetry.track_event("tool_usage", {
            let mut m = std::collections::HashMap::new();
            m.insert("tool".into(), name);
            m
        });
        self.messages_changed();
    }

    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.agent.current_action = None;
        self.agent.tool_started_at = None;
        if let Some(name) = self.agent.current_tool_name.take() {
            if let Some(idx) = self
                .session
                .messages
                .iter()
                .rposition(|m| m.role == Role::Tool && m.content.contains("⠋ Running "))
            {
                if let Some(last) = self.session.messages.get_mut(idx) {
                    last.content = if output.trim().is_empty() {
                        tool_done(&name, duration_secs)
                    } else {
                        format!("{}\n{}", tool_done(&name, duration_secs), output)
                    };
                    last.timestamp = now();
                }
            }
        }
        self.messages_changed();
    }

    pub(crate) fn append_response(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        if let Some(idx) = self.find_cached_assistant_index(&id) {
            self.append_to_message(idx, &content);
            return;
        }
        if let Some(idx) = self.find_assistant_by_id(&id) {
            self.agent.last_assistant_index = Some(idx);
            self.append_to_message(idx, &content);
            return;
        }
        self.create_assistant_message(id, content);
    }

    fn track_response_tokens(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        let n = self.agent.token_tracker.estimate_output(content);
        self.agent.tokens_out += n;
        self.agent.turn_tokens_out += n;
    }

    fn find_cached_assistant_index(&self, id: &str) -> Option<usize> {
        let idx = self.agent.last_assistant_index?;
        let msg = self.session.messages.get(idx)?;
        if msg.role == Role::Assistant && msg.id == id {
            Some(idx)
        } else {
            None
        }
    }

    fn find_assistant_by_id(&self, id: &str) -> Option<usize> {
        self.session
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
    }

    fn append_to_message(&mut self, idx: usize, content: &str) {
        if let Some(msg) = self.session.messages.get_mut(idx) {
            if !content.is_empty() {
                msg.content.push_str(content);
            }
            msg.timestamp = now();
            self.messages_changed();
        }
    }

    fn create_assistant_message(&mut self, id: String, content: String) {
        if content.is_empty() {
            return;
        }
        let idx = self.session.messages.len();
        self.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content,
            timestamp: now(),
            id: id.clone(),
            provider: self.config.current_provider.clone(),
        });
        self.agent.current_request_id = Some(id);
        self.agent.last_assistant_index = Some(idx);
        self.messages_changed();
    }

    pub(crate) fn complete_turn(&mut self, id: String, duration_secs: f64) {
        let content = format!("Turn completed in {:.1}s", duration_secs);
        let ts = now();
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            self.session.messages[idx].content = content;
            self.session.messages[idx].timestamp = ts;
        } else {
            self.session.messages.push(ChatMessage {
                role: Role::TurnComplete,
                content,
                timestamp: ts,
                id,
                ..Default::default()
            });
        }
        self.messages_changed();
        self.agent.turn_started_at = None;
    }

    pub(crate) fn finish_turn(&mut self, id: String) {
        self.strip_tools_from_assistant();
        self.remove_empty_assistant();
        self.clear_turn_state(&id);
        self.deliver_queued();
        self.maybe_end_streaming();
        self.reorder_agent_after_tools();
        self.move_turn_complete_to_end(&id);
        self.messages_changed();
    }

    fn strip_tools_from_assistant(&mut self) {
        for msg in self.session.messages.iter_mut() {
            if msg.role == Role::Assistant {
                msg.content = strip_tool_markers(&msg.content);
            }
        }
    }

    fn remove_empty_assistant(&mut self) {
        self.session
            .messages
            .retain(|msg| !(msg.role == Role::Assistant && msg.content.trim().is_empty()));
    }

    fn clear_turn_state(&mut self, id: &str) {
        if self.agent.current_request_id.as_deref() == Some(id) {
            self.agent.current_request_id = None;
        }
        self.agent.current_tool_name = None;
        self.agent.current_action = None;
        self.agent.intermediate_step_count = 0;
        self.agent.thought_seq = 0;
        self.agent.turn_active = false;
        self.agent.turn_started_at = None;
        self.vim_nav_pending = false;
        self.agent.inflight = self.agent.inflight.saturating_sub(1);
        // Reset per-turn speed tracking (but keep speed_window for continuity)
        self.agent.turn_tokens_out = 0;
        self.agent.speed_tps = 0.0;
        self.agent.last_speed_update = None;
    }

    fn maybe_end_streaming(&mut self) {
        if self.agent.inflight == 0 && self.agent.request_queue.is_empty() {
            self.agent.streaming = false;
            if self.agent.current_request_id.is_none() {
                self.agent.thinking_started_at = None;
            }
        }
    }

    fn reorder_agent_after_tools(&mut self) {
        let last_assistant = self
            .session
            .messages
            .iter()
            .rposition(|m| m.role == Role::Assistant);
        let last_tool = self
            .session
            .messages
            .iter()
            .rposition(|m| m.role == Role::Tool);
        if let (Some(a_idx), Some(t_idx)) = (last_assistant, last_tool) {
            if a_idx < t_idx {
                let mut agent = self.session.messages.remove(a_idx);
                agent.timestamp = now();
                self.session.messages.insert(t_idx, agent);
                // Update cached index if it was affected
                if self.agent.last_assistant_index == Some(a_idx) {
                    self.agent.last_assistant_index = Some(t_idx);
                }
            }
        }
    }

    fn move_turn_complete_to_end(&mut self, id: &str) {
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            let mut turn_complete = self.session.messages.remove(idx);
            turn_complete.timestamp = now();
            self.session.messages.push(turn_complete);
            // Update cached index if it was affected
            if let Some(last_idx) = self.agent.last_assistant_index {
                if last_idx >= idx {
                    self.agent.last_assistant_index = Some(last_idx.saturating_sub(1));
                }
            }
        }
    }

    pub(crate) fn add_error(&mut self, id: String, message: String) {
        self.agent.streaming = false;
        let mut error = ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
            provider: self.config.current_provider.clone(),
        };
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete)
        {
            error.timestamp = self.session.messages[idx].timestamp;
            self.session.messages.insert(idx, error);
        } else {
            self.session.messages.push(error);
        }
        self.messages_changed();
    }
}
