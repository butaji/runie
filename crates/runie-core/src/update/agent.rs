use crate::labels::{thought_with_time, tool_running, tool_done};
use crate::model::{AppState, ChatMessage, Role};
use super::{content_has_tool_markers, now, strip_tool_markers};

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        self.streaming = true;
        self.current_request_id = Some(id);
        self.thinking_started_at = Some(std::time::Instant::now());
        self.turn_active = true;
        self.current_action = Some("Thinking".to_string());
        self.turn_started_at.get_or_insert_with(std::time::Instant::now);
        self.messages_changed();
    }

    pub(crate) fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.current_action = None;
        self.thinking_started_at = None;
        let mut insert_idx = self.messages.len();
        let thought_content = if let Some(idx) = self.messages.iter().position(|m| m.role == Role::Assistant && m.id == id) {
            let assistant = &self.messages[idx];
            let has_tools = content_has_tool_markers(&assistant.content);
            let stripped = strip_tool_markers(&assistant.content);
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
        let thought_id = format!("{}#thought.{}", id, self.thought_seq);
        self.thought_seq += 1;
        let thought = ChatMessage {
            role: Role::Thought,
            content: thought_content,
            timestamp: now(),
            id: thought_id,
            ..Default::default()
        };
        self.messages.insert(insert_idx, thought);
        self.messages_changed();
    }

    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        self.current_request_id = Some(id.clone());
        self.current_tool_name = Some(name.clone());
        self.tool_started_at = Some(std::time::Instant::now());
        self.intermediate_step_count += 1;
        self.current_action = Some(format!("Running {}", name));
        let tool_id = format!("tool.{}.{}", id, self.intermediate_step_count);
        self.messages.push(ChatMessage {
            role: Role::Tool,
            content: tool_running(&name),
            timestamp: now(),
            id: tool_id,
            ..Default::default()
        });
        self.messages_changed();
    }

    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.current_action = None;
        self.tool_started_at = None;
        if let Some(name) = self.current_tool_name.take() {
            if let Some(idx) = self.messages.iter().rposition(|m| {
                m.role == Role::Tool && m.content.contains("⠋ Running ")
            }) {
                if let Some(last) = self.messages.get_mut(idx) {
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
            if let Some(msg) = self.messages.get_mut(idx) {
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
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::Assistant && m.id == id) {
            if let Some(msg) = self.messages.get_mut(idx) {
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
            let idx = self.messages.len();
            self.messages.push(ChatMessage {
                role: Role::Assistant,
                content,
                timestamp: now(),
                id: id.clone(),
                provider: self.current_provider.clone(),
            });
            self.current_request_id = Some(id);
            self.last_assistant_index = Some(idx);
            self.messages_changed();
        }
    }

    pub(crate) fn complete_turn(&mut self, id: String, duration_secs: f64) {
        let content = format!("Turn completed in {:.1}s", duration_secs);
        let ts = now();
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::TurnComplete && m.id == id) {
            self.messages[idx].content = content;
            self.messages[idx].timestamp = ts;
        } else {
            self.messages.push(ChatMessage {
                role: Role::TurnComplete,
                content,
                timestamp: ts,
                id,
                ..Default::default()
            });
        }
        self.messages_changed();
        self.turn_started_at = None;
    }

    pub(crate) fn finish_turn(&mut self, id: String) {
        self.strip_tools_from_assistant();
        self.remove_empty_assistant();
        self.clear_turn_state(&id);
        self.deliver_queued();
        self.maybe_end_streaming();
        self.reorder_agent_after_tools();
        self.move_turn_complete_to_end();
        self.messages_changed();
    }

    fn strip_tools_from_assistant(&mut self) {
        for msg in self.messages.iter_mut() {
            if msg.role == Role::Assistant {
                msg.content = strip_tool_markers(&msg.content);
            }
        }
    }

    fn remove_empty_assistant(&mut self) {
        self.messages.retain(|msg| {
            !(msg.role == Role::Assistant && msg.content.trim().is_empty())
        });
    }

    fn clear_turn_state(&mut self, id: &str) {
        if self.current_request_id.as_deref() == Some(id) {
            self.current_request_id = None;
        }
        self.current_tool_name = None;
        self.current_action = None;
        self.intermediate_step_count = 0;
        self.thought_seq = 0;
        self.turn_active = false;
        self.turn_started_at = None;
        self.inflight = self.inflight.saturating_sub(1);
    }

    fn maybe_end_streaming(&mut self) {
        if self.inflight == 0 && self.request_queue.is_empty() {
            self.streaming = false;
            if self.current_request_id.is_none() {
                self.thinking_started_at = None;
            }
        }
    }

    fn reorder_agent_after_tools(&mut self) {
        let last_assistant = self.messages.iter().rposition(|m| m.role == Role::Assistant);
        let last_tool = self.messages.iter().rposition(|m| m.role == Role::Tool);
        if let (Some(a_idx), Some(t_idx)) = (last_assistant, last_tool) {
            if a_idx < t_idx {
                let mut agent = self.messages.remove(a_idx);
                agent.timestamp = now();
                self.messages.insert(t_idx, agent);
                // Update cached index if it was affected
                if self.last_assistant_index == Some(a_idx) {
                    self.last_assistant_index = Some(t_idx);
                }
            }
        }
    }

    fn move_turn_complete_to_end(&mut self) {
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::TurnComplete) {
            let mut turn_complete = self.messages.remove(idx);
            turn_complete.timestamp = now();
            self.messages.push(turn_complete);
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
            provider: self.current_provider.clone(),
        };
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::TurnComplete) {
            error.timestamp = self.messages[idx].timestamp;
            self.messages.insert(idx, error);
        } else {
            self.messages.push(error);
        }
        self.messages_changed();
    }
}
