use crate::labels::{thought_with_time, tool_running, tool_done};
use crate::model::{AppState, ChatMessage, Role};
use super::{content_has_tool_markers, now, strip_tool_markers};

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        self.streaming = true;
        self.agent.current_request_id = Some(id);
        self.thinking_started_at = Some(std::time::Instant::now());
        self.agent.turn_active = true;
        self.current_action = Some("Thinking".to_string());
        self.agent.turn_started_at.get_or_insert_with(std::time::Instant::now);
        self.messages_changed();
    }

    pub(crate) fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.current_action = None;
        self.thinking_started_at = None;
        let mut insert_idx = self.session.messages.len();
        let thought_content = if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::Assistant && m.id == id) {
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
        let thought_id = format!("{}#thought.{}", id, self.thought_seq);
        self.thought_seq += 1;
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
        self.intermediate_step_count += 1;
        self.current_action = Some(format!("Running {}", name));
        let tool_id = format!("tool.{}.{}", id, self.intermediate_step_count);
        self.session.messages.push(ChatMessage {
            role: Role::Tool,
            content: tool_running(&name),
            timestamp: now(),
            id: tool_id,
            ..Default::default()
        });
        self.telemetry.track_event(
            "tool_usage",
            {
                let mut m = std::collections::HashMap::new();
                m.insert("tool".into(), name);
                m
            },
        );
        self.messages_changed();
    }

    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.current_action = None;
        self.agent.tool_started_at = None;
        if let Some(name) = self.agent.current_tool_name.take() {
            if let Some(idx) = self.session.messages.iter().rposition(|m| {
                m.role == Role::Tool && m.content.contains("⠋ Running ")
            }) {
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
        // O(1) lookup using cached last_assistant_index
        if let Some(idx) = self.last_assistant_index {
            if let Some(msg) = self.session.messages.get_mut(idx) {
                if msg.role == Role::Assistant && msg.id == id {
                    if !content.is_empty() {
                        msg.content.push_str(&content);
                    }
                    msg.timestamp = now();
                    self.messages_changed();
                    return;
                }
            }
        }
        // Fallback: search for existing message
        if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::Assistant && m.id == id) {
            if let Some(msg) = self.session.messages.get_mut(idx) {
                if !content.is_empty() {
                    msg.content.push_str(&content);
                }
                msg.timestamp = now();
                self.last_assistant_index = Some(idx);
                self.messages_changed();
                return;
            }
        }
        // New message
        if !content.is_empty() {
            let idx = self.session.messages.len();
            self.session.messages.push(ChatMessage {
                role: Role::Assistant,
                content,
                timestamp: now(),
                id: id.clone(),
                provider: self.config.current_provider.clone(),
            });
            self.agent.current_request_id = Some(id);
            self.last_assistant_index = Some(idx);
            self.messages_changed();
        }
    }

    pub(crate) fn complete_turn(&mut self, id: String, duration_secs: f64) {
        let content = format!("Turn completed in {:.1}s", duration_secs);
        let ts = now();
        if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::TurnComplete && m.id == id) {
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
        self.session.messages.retain(|msg| {
            !(msg.role == Role::Assistant && msg.content.trim().is_empty())
        });
    }

    fn clear_turn_state(&mut self, id: &str) {
        if self.agent.current_request_id.as_deref() == Some(id) {
            self.agent.current_request_id = None;
        }
        self.agent.current_tool_name = None;
        self.current_action = None;
        self.intermediate_step_count = 0;
        self.thought_seq = 0;
        self.agent.turn_active = false;
        self.agent.turn_started_at = None;
        self.agent.inflight = self.agent.inflight.saturating_sub(1);
    }

    fn maybe_end_streaming(&mut self) {
        if self.agent.inflight == 0 && self.agent.request_queue.is_empty() {
            self.streaming = false;
            if self.agent.current_request_id.is_none() {
                self.thinking_started_at = None;
            }
        }
    }

    fn reorder_agent_after_tools(&mut self) {
        let last_assistant = self.session.messages.iter().rposition(|m| m.role == Role::Assistant);
        let last_tool = self.session.messages.iter().rposition(|m| m.role == Role::Tool);
        if let (Some(a_idx), Some(t_idx)) = (last_assistant, last_tool) {
            if a_idx < t_idx {
                let mut agent = self.session.messages.remove(a_idx);
                agent.timestamp = now();
                self.session.messages.insert(t_idx, agent);
                // Update cached index if it was affected
                if self.last_assistant_index == Some(a_idx) {
                    self.last_assistant_index = Some(t_idx);
                }
            }
        }
    }

    fn move_turn_complete_to_end(&mut self, id: &str) {
        if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::TurnComplete && m.id == id) {
            let mut turn_complete = self.session.messages.remove(idx);
            turn_complete.timestamp = now();
            self.session.messages.push(turn_complete);
            // Update cached index if it was affected
            if let Some(last_idx) = self.last_assistant_index {
                if last_idx >= idx {
                    self.last_assistant_index = Some(last_idx.saturating_sub(1));
                }
            }
        }
    }

    pub(crate) fn add_error(&mut self, id: String, message: String) {
        self.streaming = false;
        let mut error = ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
            provider: self.config.current_provider.clone(),
        };
        if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::TurnComplete) {
            error.timestamp = self.session.messages[idx].timestamp;
            self.session.messages.insert(idx, error);
        } else {
            self.session.messages.push(error);
        }
        self.messages_changed();
    }
}
