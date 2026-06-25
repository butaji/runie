use super::dialog::open_session_tree_dialog;
use crate::model::AppState;

impl AppState {
    // === Session Event Handler ===

    pub(super) fn toggle_session_tree_dialog(&mut self) {
        use crate::commands::DialogState;
        if matches!(self.open_dialog, Some(DialogState::SessionTree(_))) {
            *self.open_dialog_mut() = None;
            self.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            self.view_mut().dirty = true;
        } else {
            self.view_mut().cached_session_tree_valid = false;
            open_session_tree_dialog(self);
        }
    }

    pub(super) fn cycle_session_tree_filter(&mut self) {
        use crate::commands::DialogState;
        if let Some(DialogState::SessionTree(stack)) = &mut *self.open_dialog_mut() {
            if let Some(_panel) = stack.current_mut() {
                // cycle through filter variants based on panel id or custom logic
                // For now just mark dirty so the panel re-renders
                self.view_mut().dirty = true;
            }
        }
    }

    pub(super) fn fork_session_at(&mut self, message_index: usize) {
        if let Some(ref mut tree) = self.session_mut().session_tree {
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        } else {
            let mut tree =
                crate::session::tree::SessionTree::from_messages(&self.session_mut().messages);
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.session_mut().session_tree = Some(tree);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        }
    }

    pub(super) fn clone_session(&mut self) {
        let tree = self.session_mut().session_tree.clone().unwrap_or_else(|| {
            crate::session::tree::SessionTree::from_messages(&self.session_mut().messages)
        });
        self.session_mut().session_tree = Some(tree);
        self.add_system_msg("Session cloned at current position.".into());
    }

    pub(super) fn session_tree_select(&mut self, id: &str) {
        let navigated = self
            .session
            .session_tree
            .as_mut()
            .and_then(|tree| tree.find_path_by_id(id))
            .map(|path| {
                self.session
                    .session_tree
                    .as_mut()
                    .unwrap()
                    .navigate_to(&path);
                true
            })
            .unwrap_or(false);
        if navigated {
            *self.open_dialog_mut() = None;
            self.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            self.add_system_msg("Switched to selected branch.".into());
        }
    }

    /// Replay a persisted message into the session without side effects.
    pub(crate) fn replay_message(
        &mut self,
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        provider: String,
    ) {
        let role = crate::model::Role::parse(&role).unwrap_or(crate::model::Role::Assistant);
        self.session_mut().messages.push(crate::model::ChatMessage {
            role,
            timestamp,
            id,
            provider,
            parts: vec![runie_core::message::Part::Text { content }],
            ..Default::default()
        });
        self.messages_changed();
    }
}

// ── Message queue (merged from queue.rs) ─────────────────────────────────────

use super::now;
use crate::model::{ChatMessage, DeliveryMode, Role};

impl AppState {
    pub(crate) fn queue_follow_up(&mut self) {
        if self.input_mut().input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input_mut().input)
            .trim().to_owned();
        self.input_mut().cursor_pos = 0;
        if content.is_empty() {
            return;
        }
        self.agent_state_mut()
            .message_queue
            .push(crate::model::QueuedMessage {
                content,
                kind: crate::model::QueuedMessageKind::FollowUp,
            });
        self.view_mut().scroll = 0;
        self.view_mut().dirty = true;
    }

    pub(super) fn abort_queue(&mut self) {
        if self.completion_mut().at_suggestions.take().is_some() {
            self.completion_mut().at_selected = None;
            self.completion_mut().last_at_query = None;
            self.view_mut().dirty = true;
            return;
        }
        let msgs: Vec<_> = self
            .agent_state_mut()
            .message_queue
            .drain(..)
            .rev()
            .collect();
        for msg in msgs {
            if !self.input_mut().input.is_empty() {
                self.input_mut().input.push('\n');
            }
            self.input_mut().input.push_str(&msg.content);
        }
        self.view_mut().dirty = true;
    }

    pub(crate) fn deliver_queued(&mut self) {
        if self.agent_state_mut().message_queue.is_empty() {
            return;
        }
        // Try to deliver steering (batch or single depending on mode)
        if self.try_deliver_steering() {
            // Steering delivered - in "All" mode, also try follow-ups
            if self.config_mut().follow_up_mode == DeliveryMode::All && self.has_follow_ups() {
                self.try_deliver_follow_ups_all();
            }
            return;
        }
        // No steering in queue - try follow-ups
        self.try_deliver_follow_up();
    }

    fn has_follow_ups(&self) -> bool {
        self.agent
            .message_queue
            .iter()
            .any(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
    }

    /// Deliver all follow-ups in batch mode.
    fn try_deliver_follow_ups_all(&mut self) {
        let follow_ups: Vec<String> = self
            .agent
            .message_queue
            .iter()
            .filter(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
            .map(|m| m.content.clone())
            .collect();

        if follow_ups.is_empty() {
            return;
        }

        let content = follow_ups.join("\n");

        // Remove all follow-ups from queue
        self.agent
            .message_queue
            .retain(|m| m.kind != crate::model::QueuedMessageKind::FollowUp);

        self.push_user_message(content);
        self.view_mut().scroll = 0;
        self.messages_changed();
    }

    fn push_user_message(&mut self, content: String) {
        let id = self.next_id();
        self.session_mut().messages.push(ChatMessage {
            role: Role::User,
            timestamp: now(),
            id: id.clone(),
            parts: vec![runie_core::message::Part::Text {
                content: content.clone(),
            }],
            ..Default::default()
        });
        self.agent_state_mut()
            .request_queue
            .push_back((content, id));
    }

    fn try_deliver_steering(&mut self) -> bool {
        match self.config_mut().steering_mode {
            DeliveryMode::OneAtATime => self.try_steering_one(),
            DeliveryMode::All => self.try_steering_all(),
        }
    }

    fn try_steering_one(&mut self) -> bool {
        let kind = crate::model::QueuedMessageKind::Steering;
        let idx = match self
            .agent_state()
            .message_queue
            .iter()
            .position(|m| m.kind == kind)
        {
            Some(idx) => idx,
            None => return false,
        };
        let content = self.agent_state_mut().message_queue.remove(idx).content;
        self.push_user_message(content);
        self.view_mut().scroll = 0;
        self.messages_changed();
        true
    }

    fn try_steering_all(&mut self) -> bool {
        let kind = crate::model::QueuedMessageKind::Steering;
        let steerings: Vec<String> = self
            .agent_state()
            .message_queue
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.content.clone())
            .collect();
        if steerings.is_empty() {
            return false;
        }
        let content = steerings.join("\n");
        self.agent_state_mut()
            .message_queue
            .retain(|m| m.kind != kind);
        self.push_user_message(content);
        self.view_mut().scroll = 0;
        self.messages_changed();
        true
    }

    pub(crate) fn dequeue(&mut self) {
        if let Some(msg) = self.agent_state_mut().message_queue.pop() {
            self.input_mut().input = msg.content;
            self.input_mut().cursor_pos = self.input_mut().input.len();
            self.view_mut().dirty = true;
        } else {
            self.input_mut().input_flash = 3;
            self.view_mut().dirty = true;
        }
    }

    fn try_deliver_follow_up(&mut self) {
        if let Some(idx) = self
            .agent
            .message_queue
            .iter()
            .position(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
        {
            let content = self.agent_state_mut().message_queue.remove(idx).content;
            self.push_user_message(content);
            self.view_mut().scroll = 0;
            self.messages_changed();
        }
    }
}

// ── Session event dispatcher ─────────────────────────────────────────────────

pub(super) fn handle_session_event(state: &mut AppState, event: crate::Event) {
    match event {
        crate::Event::ForkSession { message_index } => {
            state.fork_session_at(message_index);
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::CloneSession => {
            state.clone_session();
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::ToggleSessionTree => {
            state.toggle_session_tree_dialog();
            state.view_mut().cached_session_tree_valid = false;
        }
        crate::Event::SessionTreeFilterCycle => {
            state.cycle_session_tree_filter();
        }
        crate::Event::SessionTreeSelect { id } => {
            state.session_tree_select(&id);
        }
        // intentionally ignored: other session events fall through
        _ => {}
    }
}
