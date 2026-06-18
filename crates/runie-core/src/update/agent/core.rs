use crate::labels::{tool_done, tool_running};
use crate::message::now;
use crate::model::{AppState, ChatMessage, Role};
use crate::update::agent::thought::{plan_thought, ThoughtPlan};
use crate::update::strip_tool_markers;

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
        // Reset streaming buffer for new turn
        self.agent.streaming_buffer.reset();
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
        self.flush_buffered_response(&id);

        let (insert_idx, plan) = if let Some(idx) = self.find_assistant_by_id(&id) {
            let plan = plan_thought(&self.session.messages[idx].content, duration);
            if plan.remove_assistant {
                self.session.messages.remove(idx);
                self.agent.last_assistant_index = None;
            } else if let Some(visible) = &plan.visible_content {
                self.session.messages[idx].content = visible.clone();
            }
            (idx, plan)
        } else {
            (self.session.messages.len(), ThoughtPlan::plain(duration))
        };

        let thought_id = format!("{}#thought.{}", id, self.agent.thought_seq);
        self.agent.thought_seq += 1;
        self.session.messages.insert(
            insert_idx,
            ChatMessage {
                role: Role::Thought,
                content: plan.thought_content,
                timestamp: now(),
                id: thought_id,
                ..Default::default()
            },
        );

        if !plan.remove_assistant && self.agent.last_assistant_index == Some(insert_idx) {
            self.agent.last_assistant_index = Some(insert_idx + 1);
        }
        self.messages_changed();
    }

    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        self.agent.current_request_id = Some(id.clone());
        self.agent.current_tool_name = Some(name.clone());
        self.agent.thinking_started_at = None;
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
                    let output = crate::tool::truncate_output(
                        &output,
                        self.config.truncation.max_bytes,
                        self.config.truncation.max_lines,
                    );
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

    pub(crate) fn append_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        self.agent.streaming_buffer.push_delta(&content);
        // Find existing assistant message and append stable content, or create new one
        let stable_lines = self.agent.streaming_buffer.flush();
        if !stable_lines.is_empty() {
            let stable = stable_lines.join("");
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent.last_assistant_index = Some(idx);
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

    fn flush_buffered_response(&mut self, id: &str) {
        let buffered = self.agent.streaming_buffer.force_flush().join("");
        if buffered.is_empty() {
            return;
        }
        if let Some(idx) = self.find_cached_assistant_index(id) {
            self.append_to_message(idx, &buffered);
        } else if let Some(idx) = self.find_assistant_by_id(id) {
            self.agent.last_assistant_index = Some(idx);
            self.append_to_message(idx, &buffered);
        }
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
            ..Default::default()
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
        // Flush any remaining tail content from streaming buffer
        let remaining_tail = self.agent.streaming_buffer.force_flush().join("");
        if !remaining_tail.is_empty() {
            if let Some(idx) = self.agent.last_assistant_index {
                self.append_to_message(idx, &remaining_tail);
            }
        }
        self.agent.streaming_buffer.reset();
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
        self.view.vim_nav_pending = false;
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
        self.agent.turn_active = false;
        self.agent.current_request_id = None;
        self.agent.inflight = 0;
        self.agent.turn_started_at = None;
        self.agent.thinking_started_at = None;
        self.agent.tool_started_at = None;
        self.agent.current_tool_name = None;
        self.agent.current_action = None;
        self.agent.turn_tokens_out = 0;
        self.agent.intermediate_step_count = 0;
        self.agent.thought_seq = 0;
        self.agent.last_assistant_index = None;
        self.agent.streaming_buffer.reset();
        self.view.vim_nav_pending = false;

        let mut error = ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
            provider: self.config.current_provider.clone(),
            ..Default::default()
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

