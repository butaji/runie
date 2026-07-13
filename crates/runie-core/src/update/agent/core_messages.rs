use crate::message::{now, Part};
use crate::model::{AppState, ChatMessage, Role};
use crate::update::strip_tool_markers;

impl AppState {
    pub(crate) fn flush_buffered_response(&mut self, id: &str) {
        let buffered = self
            .agent_state_mut()
            .streaming_buffer
            .force_flush()
            .join("");
        if buffered.is_empty() {
            return;
        }
        if let Some(idx) = self.find_cached_assistant_index(id) {
            self.append_to_message(idx, &buffered);
        } else if let Some(idx) = self.find_assistant_by_id(id) {
            self.agent_state_mut().last_assistant_index = Some(idx);
            self.append_to_message(idx, &buffered);
        } else {
            self.create_assistant_message(id.to_owned(), buffered);
        }
    }

    pub(crate) fn append_delta_to_text_part(&mut self, content: &str) {
        if let Some(msg) = self.current_assistant_message_mut() {
            if let Some(Part::Text { content: text }) = msg.parts.last_mut() {
                text.push_str(content);
            }
        }
    }

    pub(crate) fn on_assistant_message_ready(&mut self, message: ChatMessage) {
        if let Some(idx) = self.agent_state().last_assistant_index {
            if idx < self.session_mut().messages.len()
                && self.session_mut().messages[idx].role == Role::Assistant
            {
                self.session_mut().messages[idx] = message;
                self.messages_changed();
                return;
            }
        }
        self.session_mut().messages.push(message);
        self.agent_state_mut().last_assistant_index = Some(self.session_mut().messages.len() - 1);
        self.messages_changed();
    }

    pub(crate) fn append_to_message(&mut self, idx: usize, content: &str) {
        if let Some(msg) = self.session_mut().messages.get_mut(idx) {
            msg.push_text_part(content);
            msg.timestamp = now();
            self.messages_changed();
        }
    }

    pub(crate) fn create_assistant_message(&mut self, id: String, content: String) {
        if content.is_empty() {
            return;
        }
        let idx = self.session_mut().messages.len();
        let provider = self.config().current_provider.clone();
        self.session_mut().messages.push(ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: id.clone(),
            provider,
            parts: vec![Part::Text { content }],
            ..Default::default()
        });
        self.agent_state_mut().current_request_id = Some(id);
        self.agent_state_mut().last_assistant_index = Some(idx);
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
            self.session_mut().messages[idx].set_text_part(content);
            self.session_mut().messages[idx].timestamp = ts;
        } else {
            self.session_mut().messages.push(ChatMessage {
                role: Role::TurnComplete,
                timestamp: ts,
                id,
                parts: vec![Part::Text { content }],
                ..Default::default()
            });
        }
        self.messages_changed();
        self.agent_state_mut().turn_started_at = None;
    }

    pub(crate) fn finish_turn(&mut self, id: String) {
        // Read from AgentState (the projection target).
        let assistant_idx = self.agent_state().last_assistant_index;
        // Release any partial-tag tail still held by the think filter, so a
        // turn that ends without a ThoughtDone event does not lose text.
        let think_tail = self.agent_state_mut().think_filter.finish();
        if !think_tail.visible.is_empty() {
            self.agent_state_mut()
                .streaming_buffer
                .push_delta(&think_tail.visible);
        }
        let remaining_tail = self
            .agent_state_mut()
            .streaming_buffer
            .force_flush()
            .join("");
        if !remaining_tail.is_empty() {
            if let Some(idx) = assistant_idx {
                self.append_to_message(idx, &remaining_tail);
            }
        }
        self.agent_state_mut().streaming_buffer.reset();
        self.agent_state_mut().think_filter.reset();
        self.close_open_parts(assistant_idx, &remaining_tail);
        self.strip_tools_from_assistant();
        self.remove_empty_assistant();
        self.clear_turn_state(&id);
        // Clear last_assistant_index.
        self.agent_state_mut().last_assistant_index = None;
        self.deliver_queued();
        // NOTE: Do NOT clear message_queue here. In production mode, TurnActor
        // emits SteeringDelivered/FollowUpDelivered which sync the queue. In test
        // mode, apply_queue_delivery_sync already removes delivered items and
        // puts remaining items back. Clearing would wipe undelivered messages.
        self.maybe_end_streaming();
        self.reorder_agent_after_tools();
        self.move_turn_complete_to_end(&id);
        self.messages_changed();
    }

    fn close_open_parts(&mut self, assistant_idx: Option<usize>, remaining_tail: &str) {
        let Some(idx) = assistant_idx else {
            return;
        };
        let Some(msg) = self.session_mut().messages.get_mut(idx) else {
            return;
        };
        if msg.role != Role::Assistant {
            return;
        }
        if !remaining_tail.is_empty() {
            if let Some(Part::Text { content }) = msg.parts.last_mut() {
                content.push_str(remaining_tail);
            } else if let Some(Part::Reasoning { content }) = msg.parts.last_mut() {
                content.push_str(remaining_tail);
            }
        }
        if msg.parts.is_empty() && !remaining_tail.is_empty() {
            msg.parts.push(Part::Text {
                content: remaining_tail.to_owned(),
            });
        }
    }

    fn strip_tools_from_assistant(&mut self) {
        for msg in self.session_mut().messages.iter_mut() {
            if msg.role == Role::Assistant {
                // Strip per Text part, never on the joined content: parts are
                // concatenated without a separator by `content()`, so a tool
                // marker ending one part would swallow the next part's text
                // during whole-string stripping, and writing the stripped
                // result back into only the last part left earlier parts
                // (with the marker) untouched — the feed then hid the whole
                // message via `should_skip_msg`.
                for part in msg.parts.iter_mut() {
                    if let Part::Text { content } = part {
                        let stripped = strip_tool_markers(content);
                        *content = crate::update::strip_thinking_tags(&stripped);
                    }
                }
                // A marker-only part strips to empty: drop it so it cannot
                // render as an empty assistant post.
                msg.parts
                    .retain(|p| !matches!(p, Part::Text { content } if content.is_empty()));
            }
        }
    }

    fn remove_empty_assistant(&mut self) {
        self.session_mut().messages.retain(|msg| {
            !(msg.role == Role::Assistant
                && msg.content().trim().is_empty()
                && msg.tool_calls().is_empty())
        });
    }

    fn clear_turn_state(&mut self, id: &str) {
        // Clear turn state fields on AgentState.
        // Only clear fields that belong to this specific request.
        if self.agent_state().current_request_id.as_deref() == Some(id) {
            self.agent_state_mut().current_request_id = None;
            // thinking_started_at belongs to this request — clear it.
            self.agent_state_mut().thinking_started_at = None;
        }
        self.agent_state_mut().current_tool_name = None;
        self.agent_state_mut().current_action = None;
        self.agent_state_mut().intermediate_step_count = 0;
        self.agent_state_mut().thought_seq = 0;
        self.agent_state_mut().turn_active = false;
        self.agent_state_mut().turn_started_at = None;
        self.agent_state_mut().inflight = self.agent_state().inflight.saturating_sub(1);
        // Reset per-turn speed tracking (but keep speed_window for continuity)
        self.agent_state_mut().turn_tokens_out = 0;
        self.agent_state_mut().speed_tps = 0.0;
        self.agent_state_mut().last_speed_update = None;
        self.view_mut().vim_nav_pending = false;
    }

    fn maybe_end_streaming(&mut self) {
        // Update streaming state on AgentState.
        if self.agent_state().inflight == 0 && self.agent_state().request_queue.is_empty() {
            self.agent_state_mut().streaming = false;
            if self.agent_state().current_request_id.is_none() {
                self.agent_state_mut().thinking_started_at = None;
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
                let mut agent = self.session_mut().messages.remove(a_idx);
                agent.timestamp = now();
                self.session_mut().messages.insert(t_idx, agent);
                // Update last_assistant_index if it was affected.
                if self.agent_state().last_assistant_index == Some(a_idx) {
                    self.agent_state_mut().last_assistant_index = Some(t_idx);
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
            let mut turn_complete = self.session_mut().messages.remove(idx);
            turn_complete.timestamp = now();
            self.session_mut().messages.push(turn_complete);
            // Update last_assistant_index if it was affected.
            if let Some(last_idx) = self.agent_state().last_assistant_index {
                if last_idx >= idx {
                    self.agent_state_mut().last_assistant_index = Some(last_idx.saturating_sub(1));
                }
            }
        }
    }

    pub(crate) fn add_error(&mut self, id: String, message: String) {
        self.reset_agent_state();

        let mut error = ChatMessage {
            role: Role::Assistant,
            timestamp: now(),
            id: format!("error.{}", id),
            provider: self.config_mut().current_provider.clone(),
            parts: vec![Part::Text {
                content: format!("Error: {}", message),
            }],
            ..Default::default()
        };
        if let Some(idx) = self
            .session()
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete)
        {
            error.timestamp = self.session_mut().messages[idx].timestamp;
            self.session_mut().messages.insert(idx, error);
        } else {
            self.session_mut().messages.push(error);
        }
        self.messages_changed();
        self.deliver_queued();
        self.maybe_end_streaming();
    }

    fn reset_agent_state(&mut self) {
        // Reset AgentState fields directly.
        let agent = self.agent_state_mut();
        agent.streaming = false;
        agent.turn_active = false;
        agent.current_request_id = None;
        agent.inflight = 0;
        agent.turn_started_at = None;
        agent.thinking_started_at = None;
        agent.tool_started_at = None;
        agent.current_tool_name = None;
        agent.current_action = None;
        agent.turn_tokens_out = 0;
        agent.intermediate_step_count = 0;
        agent.thought_seq = 0;
        agent.last_assistant_index = None;
        agent.streaming_buffer.reset();
        agent.think_filter.reset();
        self.view_mut().vim_nav_pending = false;
    }
}
