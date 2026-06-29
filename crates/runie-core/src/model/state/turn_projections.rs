#![allow(clippy::all)]
//! Turn fact projections — event handlers for TurnActor facts.
//!
//! These methods project TurnActor facts into AppState.

use super::AppState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_turn_started_sets_active() {
        let mut state = AppState::default();
        state.apply_turn_started();
        assert!(state.agent_state().turn_active);
        assert_eq!(state.agent_state().inflight, 1);
        assert!(state.agent_state().streaming);
        assert!(state.agent_state().turn_started_at.is_some());
    }

    #[test]
    fn apply_turn_completed_clears_flags() {
        let mut state = AppState::default();
        state.agent_state_mut().turn_active = true;
        state.agent_state_mut().inflight = 1;
        state.agent_state_mut().streaming = true;
        state.agent_state_mut().current_tool_name = Some("bash".to_owned());

        state.apply_turn_completed();

        assert!(!state.agent_state().turn_active);
        assert!(!state.agent_state().streaming);
        assert_eq!(state.agent_state().inflight, 0);
        assert!(state.agent_state().current_tool_name.is_none());
    }

    #[test]
    fn apply_turn_errored_resets_inflight() {
        let mut state = AppState::default();
        state.agent_state_mut().turn_active = true;
        state.agent_state_mut().inflight = 2;

        state.apply_turn_errored();

        assert!(!state.agent_state().turn_active);
        assert!(!state.agent_state().streaming);
        assert_eq!(state.agent_state().inflight, 0);
    }

    #[test]
    fn apply_token_stats_updates_stats() {
        let mut state = AppState::default();
        state.apply_token_stats(100, 200, 50.0);
        assert_eq!(state.agent_state().tokens_in, 100);
        assert_eq!(state.agent_state().tokens_out, 200);
        assert_eq!(state.agent_state().speed_tps, 50.0);
        assert_eq!(state.agent_state().turn_tokens_out, 200);
    }

    #[test]
    fn apply_user_message_submitted_adds_to_session_and_queue() {
        let mut state = AppState::default();
        state.apply_user_message_submitted("req.1".to_owned(), "Hello".to_owned());

        // Check session messages
        assert_eq!(state.session().messages.len(), 1);
        let msg = &state.session().messages[0];
        assert_eq!(msg.id, "req.1");
        assert_eq!(msg.role, crate::model::Role::User);
        assert_eq!(msg.content(), "Hello");

        // Check request queue
        assert_eq!(state.agent_state().request_queue.len(), 1);
        let (content, id) = &state.agent_state().request_queue[0];
        assert_eq!(content, "Hello");
        assert_eq!(id, "req.1");
    }
}

impl AppState {
    /// Project TurnStarted fact into AppState.
    pub(crate) fn apply_turn_started(&mut self) {
        self.agent_state_mut().turn_active = true;
        self.agent_state_mut().inflight += 1;
        self.agent_state_mut().streaming = true;
        self.agent_state_mut().turn_started_at = Some(std::time::Instant::now());
    }

    /// Project TurnCompleted fact into AppState.
    pub(crate) fn apply_turn_completed(&mut self) {
        self.agent_state_mut().streaming = false;
        self.agent_state_mut().turn_active = false;
        self.agent_state_mut().inflight = self.agent_state_mut().inflight.saturating_sub(1);
        self.agent_state_mut().current_tool_name = None;
    }

    /// Project TurnErrored fact into AppState.
    pub(crate) fn apply_turn_errored(&mut self) {
        self.agent_state_mut().streaming = false;
        self.agent_state_mut().turn_active = false;
        self.agent_state_mut().inflight = 0;
    }

    /// Project TokenStatsUpdated fact into AppState.
    pub(crate) fn apply_token_stats(
        &mut self,
        tokens_in: usize,
        tokens_out: usize,
        speed_tps: f64,
    ) {
        self.agent_state_mut().tokens_in = tokens_in;
        self.agent_state_mut().tokens_out = tokens_out;
        self.agent_state_mut().speed_tps = speed_tps;
        self.agent_state_mut().turn_tokens_out = tokens_out;
    }

    /// Project UserMessageSubmitted fact into AppState.
    /// Adds the message to session.messages and pushes to request_queue.
    pub(crate) fn apply_user_message_submitted(&mut self, id: String, content: String) {
        use crate::message::{now, ChatMessage, Part, Role};
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
        self.messages_changed();
    }

    /// Project SteeringDelivered fact into AppState.
    /// Removes from message_queue and adds to session.messages.
    pub(crate) fn apply_steering_delivered(&mut self, content: String, id: String) {
        use crate::message::{now, ChatMessage, Part, Role};
        // Remove from AppState mirror of message_queue (if present)
        self.agent_state_mut().message_queue.retain(|m| {
            !(m.kind == crate::model::QueuedMessageKind::Steering && m.content != content)
        });
        // Add to session
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        // Add to request_queue (for agent to pick up)
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
        self.messages_changed();
    }

    /// Project FollowUpDelivered fact into AppState.
    /// Removes from message_queue and adds to session.messages.
    pub(crate) fn apply_follow_up_delivered(&mut self, content: String, id: String) {
        use crate::message::{now, ChatMessage, Part, Role};
        // Remove from AppState mirror of message_queue (if present)
        self.agent_state_mut().message_queue.retain(|m| {
            !(m.kind == crate::model::QueuedMessageKind::FollowUp && m.content != content)
        });
        // Add to session
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        // Add to request_queue (for agent to pick up)
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
        self.messages_changed();
    }

    /// Project MessageDequeued fact into AppState.
    /// Pops the last message from message_queue back to input.
    pub(crate) fn apply_message_dequeued(&mut self, content: String) {
        // Update input with dequeued content
        self.input_mut().input = content;
        self.input_mut().cursor_pos = self.input().input.len();
        self.view_mut().dirty = true;
    }
}
