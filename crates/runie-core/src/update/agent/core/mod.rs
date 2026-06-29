use crate::labels::{tool_done, tool_running};
use crate::message::{now, Part};
use crate::model::{AppState, ChatMessage, Role};
use crate::update::agent::thought::{plan_thought, ThoughtPlan};

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        self.agent_state_mut().streaming = true;
        self.agent_state_mut().current_request_id = Some(id);
        self.agent_state_mut().thinking_started_at = Some(std::time::Instant::now());
        self.agent_state_mut().turn_active = true;
        self.agent_state_mut().current_action = Some("Thinking".to_owned());
        self.agent
            .turn_started_at
            .get_or_insert_with(std::time::Instant::now);
        // Reset streaming buffer for new turn
        self.agent_state_mut().streaming_buffer.reset();
        // Init speed tracking for this turn
        self.agent_state_mut().turn_tokens_out = 0;
        self.agent_state_mut().last_speed_update = Some(std::time::Instant::now());
        self.agent_state_mut().tokens_at_last_speed = self.agent_state_mut().tokens_out;
        self.agent_state_mut().speed_tps = 0.0;
        // Keep existing rolling window - it auto-evicts to 1000 tokens
        let tokens = self.agent_state().tokens_out;
        self.agent_state_mut().speed_window.record(tokens);
        self.messages_changed();
    }
    pub(crate) fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.agent_state_mut().current_action = None;
        self.agent_state_mut().thinking_started_at = None;
        self.flush_buffered_response(&id);
        let (insert_idx, plan) = if let Some(idx) = self.find_assistant_by_id(&id) {
            let plan = plan_thought(&self.session_mut().messages[idx].content(), duration);
            if plan.remove_assistant {
                self.session_mut().messages.remove(idx);
                self.agent_state_mut().last_assistant_index = None;
            } else if let Some(visible) = &plan.visible_content {
                self.session_mut().messages[idx].set_text_part(visible.clone());
            }
            (idx, plan)
        } else {
            (
                self.session_mut().messages.len(),
                ThoughtPlan::plain(duration),
            )
        };
        let thought_id = format!("{}#thought.{}", id, self.agent_state_mut().thought_seq);
        self.agent_state_mut().thought_seq += 1;
        self.session_mut().messages.insert(
            insert_idx,
            ChatMessage {
                role: Role::Thought,
                timestamp: now(),
                id: thought_id,
                parts: vec![Part::Text {
                    content: plan.thought_content,
                }],
                ..Default::default()
            },
        );
        if !plan.remove_assistant && self.agent_state_mut().last_assistant_index == Some(insert_idx)
        {
            self.agent_state_mut().last_assistant_index = Some(insert_idx + 1);
        }
        self.messages_changed();
    }
    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        self.agent_state_mut().current_request_id = Some(id.clone());
        self.agent_state_mut().current_tool_name = Some(name.clone());
        self.agent_state_mut().thinking_started_at = None;
        self.agent_state_mut().tool_started_at = Some(std::time::Instant::now());
        self.agent_state_mut().intermediate_step_count += 1;
        self.agent_state_mut().current_action = Some(format!("Running {}", name));
        let tool_id = format!(
            "tool.{}.{}",
            id,
            self.agent_state_mut().intermediate_step_count
        );
        self.session_mut().messages.push(ChatMessage {
            role: Role::Tool,
            timestamp: now(),
            id: tool_id,
            parts: vec![Part::Text {
                content: tool_running(&name),
            }],
            ..Default::default()
        });
        if self.config().telemetry_enabled() {
            tracing::info!(tool = %name, "tool_usage");
        }
        self.messages_changed();
    }
    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.agent_state_mut().current_action = None;
        self.agent_state_mut().tool_started_at = None;
        if let Some(name) = self.agent_state_mut().current_tool_name.take() {
            if let Some(idx) = self
                .session
                .messages
                .iter()
                .rposition(|m| m.role == Role::Tool && m.content().contains("⠋ Running "))
            {
                let max_bytes = self.config().truncation.max_bytes;
                let max_lines = self.config().truncation.max_lines;
                if let Some(last) = self.session_mut().messages.get_mut(idx) {
                    let output = crate::tool::truncate_output(&output, max_bytes, max_lines);
                    last.set_text_part(if output.trim().is_empty() {
                        tool_done(&name, duration_secs)
                    } else {
                        format!("{}\n{}", tool_done(&name, duration_secs), output)
                    });
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
            self.agent_state_mut().last_assistant_index = Some(idx);
            self.append_to_message(idx, &content);
            return;
        }
        self.create_assistant_message(id, content);
    }
    /// Test-only helper — production uses streaming pipeline.
    #[allow(dead_code)]
    pub(crate) fn append_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        self.agent_state_mut().streaming_buffer.push_delta(&content);
        // Find existing assistant message and append stable content, or create new one
        let stable_lines = self.agent_state_mut().streaming_buffer.flush();
        if !stable_lines.is_empty() {
            let stable = stable_lines.join("");
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent_state_mut().last_assistant_index = Some(idx);
                self.append_to_message(idx, &stable);
            } else {
                self.create_assistant_message(id.clone(), stable);
            }
        }
    }
    fn track_response_tokens(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        let n = self
            .agent_state_mut()
            .token_tracker
            .estimate_output(content);
        self.agent_state_mut().tokens_out += n;
        self.agent_state_mut().turn_tokens_out += n;
    }
    pub(crate) fn find_cached_assistant_index(&mut self, id: &str) -> Option<usize> {
        let idx = self.agent_state_mut().last_assistant_index?;
        let msg = self.session_mut().messages.get(idx)?;
        if msg.role == Role::Assistant && msg.id == id {
            Some(idx)
        } else {
            None
        }
    }
    pub(crate) fn find_assistant_by_id(&self, id: &str) -> Option<usize> {
        self.session()
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
    }
    /// Returns a mutable reference to the last assistant message, if any.
    pub(crate) fn current_assistant_message_mut(&mut self) -> Option<&mut ChatMessage> {
        self.agent
            .last_assistant_index
            .and_then(|idx| self.session_mut().messages.get_mut(idx))
            .filter(|m| m.role == Role::Assistant)
    }
    /// Handle LLM lifecycle events to populate `parts` during streaming.
    pub(crate) fn handle_llm_event(&mut self, event: crate::event::Event) {
        use crate::event::Event as E;
        match event {
            E::TextStart { .. } => self.on_text_start(),
            E::ResponseDelta { id, content } => self.on_response_delta(id, content),
            E::TextEnd { .. } => {}
            E::ThinkingStart { .. } => self.on_thinking_start(),
            E::ThinkingDelta { content, .. } => self.on_thinking_delta(content),
            E::ThinkingEnd { .. } => {} // intentionally ignored: thinking content is accumulated
            E::AssistantMessageReady { message } => self.on_assistant_message_ready(message),
            // intentionally ignored: other events fall through
            _ => {}
        }
    }
    fn on_text_start(&mut self) {
        if let Some(msg) = self.current_assistant_message_mut() {
            msg.parts.push(Part::Text {
                content: String::new(),
            });
        } else {
            self.start_assistant_message(Part::Text {
                content: String::new(),
            });
        }
    }
    fn on_thinking_start(&mut self) {
        if let Some(msg) = self.current_assistant_message_mut() {
            msg.parts.push(Part::Reasoning {
                content: String::new(),
            });
        } else {
            self.start_assistant_message(Part::Reasoning {
                content: String::new(),
            });
        }
    }
    /// Push a new assistant message for a new request cycle.
    fn start_assistant_message(&mut self, part: Part) {
        let id = self
            .agent_state_mut()
            .current_request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let provider = self.config().current_provider.clone();
        self.agent_state_mut().current_request_id = Some(id.clone());
        let idx = self.session_mut().messages.len();
        self.session_mut().messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id,
            provider,
            parts: vec![part],
            ..Default::default()
        });
        self.agent_state_mut().last_assistant_index = Some(idx);
        self.messages_changed();
    }
    fn on_thinking_delta(&mut self, content: String) {
        self.track_response_tokens(&content);
        if let Some(msg) = self.current_assistant_message_mut() {
            if let Some(Part::Reasoning { content: reasoning }) = msg.parts.last_mut() {
                reasoning.push_str(&content);
            }
        }
    }
    fn on_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        let has_open_text = self
            .current_assistant_message_mut()
            .is_some_and(|msg| matches!(msg.parts.last(), Some(Part::Text { .. })));
        if has_open_text {
            self.append_delta_to_text_part(&content);
            return;
        }
        self.agent_state_mut().streaming_buffer.push_delta(&content);
        let stable = self.agent_state_mut().streaming_buffer.flush().join("");
        if !stable.is_empty() {
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent_state_mut().last_assistant_index = Some(idx);
                self.append_to_message(idx, &stable);
            } else {
                self.create_assistant_message(id.clone(), stable);
            }
        }
    }
}

#[cfg(test)]
mod tests;
