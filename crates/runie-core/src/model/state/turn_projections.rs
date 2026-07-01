//! Turn fact projections — event handlers for TurnActor facts.
//!
//! These methods project TurnActor facts into AppState.

use super::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::QueuedMessageKind::{FollowUp, Steering};

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

    // ── Steering/FollowUp projection tests ───────────────────────────────────

    fn push_steering(state: &mut AppState, content: &str) {
        state.turn_state_mut().message_queue.push(crate::model::QueuedMessage {
            content: content.into(),
            kind: Steering,
        });
        *state.agent_state_mut() = crate::model::AgentState::from(&state.turn_state);
    }

    fn push_follow_up(state: &mut AppState, content: &str) {
        state.turn_state_mut().message_queue.push(crate::model::QueuedMessage {
            content: content.into(),
            kind: FollowUp,
        });
        *state.agent_state_mut() = crate::model::AgentState::from(&state.turn_state);
    }

    #[test]
    fn apply_steering_delivered_removes_steering_keeps_others() {
        let mut state = AppState::default();
        push_steering(&mut state, "guide");
        push_follow_up(&mut state, "extra");
        assert_eq!(state.agent_state().message_queue.len(), 2);

        state.apply_steering_delivered("guide".into(), "s.0".into());

        // Steering "guide" removed; follow-up "extra" kept
        assert_eq!(state.agent_state().message_queue.len(), 1);
        assert_eq!(state.agent_state().message_queue[0].content, "extra");
        assert!(matches!(state.agent_state().message_queue[0].kind, FollowUp));
        // Delivered message added to session and request queue
        assert_eq!(state.session().messages.len(), 1);
        assert_eq!(state.session().messages[0].content(), "guide");
        assert_eq!(state.agent_state().request_queue.len(), 1);
    }

    #[test]
    fn apply_steering_delivered_on_empty_queue_is_noop() {
        let mut state = AppState::default();
        state.apply_steering_delivered("nothing".into(), "s.0".into());
        assert!(state.agent_state().message_queue.is_empty());
        assert_eq!(state.session().messages.len(), 1); // message still added to session
        assert_eq!(state.agent_state().request_queue.len(), 1);
    }

    #[test]
    fn apply_steering_delivered_keeps_non_matching_steering() {
        let mut state = AppState::default();
        push_steering(&mut state, "guide a");
        push_steering(&mut state, "guide b");
        push_follow_up(&mut state, "extra");

        state.apply_steering_delivered("guide a".into(), "s.0".into());

        // Only "guide a" removed; "guide b" and "extra" kept
        assert_eq!(state.agent_state().message_queue.len(), 2);
        assert!(state.agent_state().message_queue.iter().all(|m| m.content != "guide a"));
    }

    #[test]
    fn apply_follow_up_delivered_removes_follow_up_keeps_others() {
        let mut state = AppState::default();
        push_steering(&mut state, "guide");
        push_follow_up(&mut state, "extra");
        assert_eq!(state.agent_state().message_queue.len(), 2);

        state.apply_follow_up_delivered("extra".into(), "f.0".into());

        // Follow-up "extra" removed; steering "guide" kept
        assert_eq!(state.agent_state().message_queue.len(), 1);
        assert_eq!(state.agent_state().message_queue[0].content, "guide");
        assert!(matches!(state.agent_state().message_queue[0].kind, Steering));
        assert_eq!(state.session().messages.len(), 1);
        assert_eq!(state.agent_state().request_queue.len(), 1);
    }

    #[test]
    fn apply_follow_up_delivered_on_empty_queue_is_noop() {
        let mut state = AppState::default();
        state.apply_follow_up_delivered("nothing".into(), "f.0".into());
        assert!(state.agent_state().message_queue.is_empty());
        assert_eq!(state.session().messages.len(), 1);
        assert_eq!(state.agent_state().request_queue.len(), 1);
    }

    #[test]
    fn apply_follow_up_delivered_keeps_non_matching_follow_ups() {
        let mut state = AppState::default();
        push_follow_up(&mut state, "a");
        push_follow_up(&mut state, "b");
        push_steering(&mut state, "guide");

        state.apply_follow_up_delivered("a".into(), "f.0".into());

        // Only "a" removed; "b" and "guide" kept
        assert_eq!(state.agent_state().message_queue.len(), 2);
        assert!(state.agent_state().message_queue.iter().all(|m| m.content != "a"));
    }

    #[test]
    fn apply_follow_up_delivered_multiple_same_content() {
        // Multiple follow-ups with same content: only the matching one is removed.
        // This is the key regression test for the retain bug where `!=` kept matching.
        let mut state = AppState::default();
        push_follow_up(&mut state, "hello");
        push_follow_up(&mut state, "hello");
        push_follow_up(&mut state, "world");
        assert_eq!(state.agent_state().message_queue.len(), 3);

        state.apply_follow_up_delivered("hello".into(), "f.0".into());

        // All "hello" follow-ups removed (both); "world" kept
        assert_eq!(state.agent_state().message_queue.len(), 1);
        assert_eq!(state.agent_state().message_queue[0].content, "world");
    }
}

impl AppState {
    /// Project TurnStarted fact into AppState.
    pub(crate) fn apply_turn_started(&mut self) {
        // Update authoritative TurnState
        self.turn_state_mut().turn_active = true;
        self.turn_state_mut().inflight += 1;
        self.turn_state_mut().streaming = true;
        self.turn_state_mut().turn_started_at = Some(std::time::Instant::now());
        // Copy turn state values into locals (releases immutable borrow of self).
        let turn_active = self.turn_state.turn_active;
        let inflight = self.turn_state.inflight;
        let streaming = self.turn_state.streaming;
        let turn_started_at = self.turn_state.turn_started_at;
        let current_request_id = self.turn_state.current_request_id.clone();
        let current_tool_name = self.turn_state.current_tool_name.clone();
        let current_action = self.turn_state.current_action.clone();
        let thinking_started_at = self.turn_state.thinking_started_at;
        let tool_started_at = self.turn_state.tool_started_at;
        // Sync authoritative fields to AgentState.
        let agent = self.agent_state_mut();
        agent.turn_active = turn_active;
        agent.inflight = inflight;
        agent.streaming = streaming;
        agent.turn_started_at = turn_started_at;
        agent.current_request_id = current_request_id;
        agent.current_tool_name = current_tool_name;
        agent.current_action = current_action;
        agent.thinking_started_at = thinking_started_at;
        agent.tool_started_at = tool_started_at;
    }

    /// Project TurnCompleted fact into AppState.
    pub(crate) fn apply_turn_completed(&mut self) {
        self.turn_state_mut().streaming = false;
        self.turn_state_mut().turn_active = false;
        self.turn_state_mut().inflight = self.turn_state_mut().inflight.saturating_sub(1);
        self.turn_state_mut().current_tool_name = None;
        // Copy turn state values into locals.
        let streaming = self.turn_state.streaming;
        let turn_active = self.turn_state.turn_active;
        let inflight = self.turn_state.inflight;
        let current_tool_name = self.turn_state.current_tool_name.clone();
        // Sync authoritative fields.
        let agent = self.agent_state_mut();
        agent.streaming = streaming;
        agent.turn_active = turn_active;
        agent.inflight = inflight;
        agent.current_tool_name = current_tool_name;
    }

    /// Project TurnErrored fact into AppState.
    pub(crate) fn apply_turn_errored(&mut self) {
        self.turn_state_mut().streaming = false;
        self.turn_state_mut().turn_active = false;
        self.turn_state_mut().inflight = 0;
        // Copy turn state values into locals.
        let streaming = self.turn_state.streaming;
        let turn_active = self.turn_state.turn_active;
        let inflight = self.turn_state.inflight;
        // Sync authoritative fields.
        let agent = self.agent_state_mut();
        agent.streaming = streaming;
        agent.turn_active = turn_active;
        agent.inflight = inflight;
    }

    /// Project TokenStatsUpdated fact into AppState.
    pub(crate) fn apply_token_stats(
        &mut self,
        tokens_in: usize,
        tokens_out: usize,
        speed_tps: f64,
    ) {
        self.turn_state_mut().tokens_in = tokens_in;
        self.turn_state_mut().tokens_out = tokens_out;
        self.turn_state_mut().speed_tps = speed_tps;
        self.turn_state_mut().turn_tokens_out = tokens_out;
        // Sync token fields to AgentState.
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
        self.turn_state_mut().request_queue.push_back((content, id));
        // Sync request_queue to AgentState.
        self.agent_state_mut().request_queue = self.turn_state.request_queue.clone();
        self.messages_changed();
    }

    /// Project SteeringDelivered fact into AppState.
    /// Removes from message_queue and adds to session.messages.
    pub(crate) fn apply_steering_delivered(&mut self, content: String, id: String) {
        use crate::message::{now, ChatMessage, Part, Role};
        // Remove the delivered steering message from the queue.
        self.turn_state_mut().message_queue.retain(|m| {
            !(m.kind == crate::model::QueuedMessageKind::Steering && m.content == content)
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
        self.turn_state_mut().request_queue.push_back((content, id));
        // Sync queues to AgentState.
        self.agent_state_mut().message_queue = self.turn_state.message_queue.clone();
        self.agent_state_mut().request_queue = self.turn_state.request_queue.clone();
        self.messages_changed();
    }

    /// Project FollowUpDelivered fact into AppState.
    /// Removes from message_queue and adds to session.messages.
    pub(crate) fn apply_follow_up_delivered(&mut self, content: String, id: String) {
        use crate::message::{now, ChatMessage, Part, Role};
        // Remove the delivered follow-up message from the queue.
        self.turn_state_mut().message_queue.retain(|m| {
            !(m.kind == crate::model::QueuedMessageKind::FollowUp && m.content == content)
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
        self.turn_state_mut().request_queue.push_back((content, id));
        // Sync queues to AgentState.
        self.agent_state_mut().message_queue = self.turn_state.message_queue.clone();
        self.agent_state_mut().request_queue = self.turn_state.request_queue.clone();
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
