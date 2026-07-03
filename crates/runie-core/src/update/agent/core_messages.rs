use crate::message::{now, Part};
use crate::model::state::AgentState;
use crate::model::{AppState, ChatMessage, Role};
use crate::update::strip_tool_markers;

impl AppState {
    pub(crate) fn flush_buffered_response(&mut self, id: &str) {
        let buffered = self.turn_state.streaming_buffer.force_flush().join("");
        if buffered.is_empty() {
            return;
        }
        if let Some(idx) = self.find_cached_assistant_index(id) {
            self.append_to_message(idx, &buffered);
        } else if let Some(idx) = self.find_assistant_by_id(id) {
            self.turn_state_mut().last_assistant_index = Some(idx);
            *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
        if let Some(idx) = self.turn_state().last_assistant_index {
            if idx < self.session_mut().messages.len()
                && self.session_mut().messages[idx].role == Role::Assistant
            {
                self.session_mut().messages[idx] = message;
                self.messages_changed();
                return;
            }
        }
        self.session_mut().messages.push(message);
        self.turn_state_mut().last_assistant_index = Some(self.session_mut().messages.len() - 1);
        *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
        self.turn_state_mut().current_request_id = Some(id);
        self.turn_state_mut().last_assistant_index = Some(idx);
        *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
        self.turn_state_mut().turn_started_at = None;
        *self.agent_state_mut() = AgentState::from(&self.turn_state);
    }

    pub(crate) fn finish_turn(&mut self, id: String) {
        // Read from TurnState (authoritative source), not derived agent.
        let assistant_idx = self.turn_state().last_assistant_index;
        let remaining_tail = self
            .turn_state_mut()
            .streaming_buffer
            .force_flush()
            .join("");
        if !remaining_tail.is_empty() {
            if let Some(idx) = assistant_idx {
                self.append_to_message(idx, &remaining_tail);
            }
        }
        self.turn_state_mut().streaming_buffer.reset();
        self.close_open_parts(assistant_idx, &remaining_tail);
        self.strip_tools_from_assistant();
        self.remove_empty_assistant();
        self.clear_turn_state(&id);
        // Set on turn_state, then sync to agent.
        self.turn_state_mut().last_assistant_index = None;
        *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
                let stripped = strip_tool_markers(&msg.content());
                let visible = crate::update::strip_thinking_tags(&stripped);
                msg.set_text_part(visible);
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
        // Mutate authoritative TurnState only — do NOT sync here.
        // Fields managed by other functions (like last_assistant_index, streaming)
        // are synced by those functions. Clearing them here would overwrite changes.
        if self.turn_state().current_request_id.as_deref() == Some(id) {
            self.turn_state_mut().current_request_id = None;
        }
        self.turn_state_mut().current_tool_name = None;
        self.turn_state_mut().current_action = None;
        self.turn_state_mut().intermediate_step_count = 0;
        self.turn_state_mut().thought_seq = 0;
        self.turn_state_mut().turn_active = false;
        self.turn_state_mut().turn_started_at = None;
        self.turn_state_mut().inflight = self.turn_state().inflight.saturating_sub(1);
        // Reset per-turn speed tracking (but keep speed_window for continuity)
        self.turn_state_mut().turn_tokens_out = 0;
        self.turn_state_mut().speed_tps = 0.0;
        self.turn_state_mut().last_speed_update = None;
        self.view_mut().vim_nav_pending = false;
    }

    fn maybe_end_streaming(&mut self) {
        // Read from TurnState, mutate through turn_state_mut, sync to AgentState.
        if self.turn_state().inflight == 0 && self.turn_state().request_queue.is_empty() {
            self.turn_state_mut().streaming = false;
            if self.turn_state().current_request_id.is_none() {
                self.turn_state_mut().thinking_started_at = None;
            }
            *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
                if self.turn_state().last_assistant_index == Some(a_idx) {
                    self.turn_state_mut().last_assistant_index = Some(t_idx);
                    *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
            if let Some(last_idx) = self.turn_state().last_assistant_index {
                if last_idx >= idx {
                    self.turn_state_mut().last_assistant_index = Some(last_idx.saturating_sub(1));
                    *self.agent_state_mut() = AgentState::from(&self.turn_state);
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
        // Mutate authoritative TurnState, then sync to AgentState projection.
        self.turn_state_mut().streaming = false;
        self.turn_state_mut().turn_active = false;
        self.turn_state_mut().current_request_id = None;
        self.turn_state_mut().inflight = 0;
        self.turn_state_mut().turn_started_at = None;
        self.turn_state_mut().thinking_started_at = None;
        self.turn_state_mut().tool_started_at = None;
        self.turn_state_mut().current_tool_name = None;
        self.turn_state_mut().current_action = None;
        self.turn_state_mut().turn_tokens_out = 0;
        self.turn_state_mut().intermediate_step_count = 0;
        self.turn_state_mut().thought_seq = 0;
        self.turn_state_mut().last_assistant_index = None;
        self.turn_state_mut().streaming_buffer.reset();
        // Sync authoritative fields to AgentState projection.
        *self.agent_state_mut() = AgentState::from(&self.turn_state);
        self.view_mut().vim_nav_pending = false;
    }
}
