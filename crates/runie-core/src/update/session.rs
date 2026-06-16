use super::dialog::open_session_tree_dialog;
use crate::model::AppState;

impl AppState {
    // === Session Event Handler ===

    pub(super) fn toggle_session_tree_dialog(&mut self) {
        use crate::commands::DialogState;
        if matches!(self.open_dialog, Some(DialogState::SessionTree(_))) {
            self.open_dialog = None;
            self.mark_dirty();
        } else {
            self.view.cached_session_tree_valid = false;
            open_session_tree_dialog(self);
        }
    }

    pub(super) fn cycle_session_tree_filter(&mut self) {
        use crate::commands::DialogState;
        if let Some(DialogState::SessionTree(stack)) = &mut self.open_dialog {
            if let Some(_panel) = stack.current_mut() {
                // cycle through filter variants based on panel id or custom logic
                // For now just mark dirty so the panel re-renders
                self.mark_dirty();
            }
        }
    }

    pub(super) fn fork_session_at(&mut self, message_index: usize) {
        if let Some(ref mut tree) = self.session.session_tree {
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        } else {
            let mut tree = crate::session_tree::SessionTree::from_messages(&self.session.messages);
            if let Some(path) = tree.fork_at(message_index) {
                tree.navigate_to(&path);
                self.session.session_tree = Some(tree);
                self.add_system_msg(format!("Forked at message {}.", message_index));
            }
        }
    }

    pub(super) fn clone_session(&mut self) {
        let tree = self.session.session_tree.clone().unwrap_or_else(|| {
            crate::session_tree::SessionTree::from_messages(&self.session.messages)
        });
        self.session.session_tree = Some(tree);
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
            self.open_dialog = None;
            self.add_system_msg("Switched to selected branch.".into());
        }
    }
}

// ── Message queue (merged from queue.rs) ─────────────────────────────────────

use super::now;
use crate::model::{ChatMessage, DeliveryMode, Role};

impl AppState {
    pub(crate) fn queue_follow_up(&mut self) {
        if self.input.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input.input).trim().to_string();
        self.input.cursor_pos = 0;
        if content.is_empty() {
            return;
        }
        self.agent.message_queue.push(crate::model::QueuedMessage {
            content,
            kind: crate::model::QueuedMessageKind::FollowUp,
        });
        self.view.scroll = 0;
        self.mark_dirty();
    }

    pub(super) fn abort_queue(&mut self) {
        if self.completion.at_suggestions.take().is_some() {
            self.completion.at_selected = None;
            self.completion.last_at_query = None;
            self.mark_dirty();
            return;
        }
        for msg in self.agent.message_queue.drain(..).rev() {
            if !self.input.input.is_empty() {
                self.input.input.push('\n');
            }
            self.input.input.push_str(&msg.content);
        }
        self.mark_dirty();
    }

    pub(crate) fn deliver_queued(&mut self) {
        if self.agent.message_queue.is_empty() {
            return;
        }
        // Try to deliver steering (batch or single depending on mode)
        if self.try_deliver_steering() {
            // Steering delivered - in "All" mode, also try follow-ups
            if self.config.follow_up_mode == DeliveryMode::All && self.has_follow_ups() {
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
        self.view.scroll = 0;
        self.messages_changed();
    }

    fn push_user_message(&mut self, content: String) {
        let id = self.next_id();
        self.session.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
            ..Default::default()
        });
        self.agent.request_queue.push_back((content, id));
    }

    fn try_deliver_steering(&mut self) -> bool {
        match self.config.steering_mode {
            DeliveryMode::OneAtATime => {
                if let Some(idx) = self
                    .agent
                    .message_queue
                    .iter()
                    .position(|m| m.kind == crate::model::QueuedMessageKind::Steering)
                {
                    let content = self.agent.message_queue.remove(idx).content;
                    self.push_user_message(content);
                    self.view.scroll = 0;
                    self.messages_changed();
                    true
                } else {
                    false
                }
            }
            DeliveryMode::All => {
                let steerings: Vec<String> = self
                    .agent
                    .message_queue
                    .iter()
                    .filter(|m| m.kind == crate::model::QueuedMessageKind::Steering)
                    .map(|m| m.content.clone())
                    .collect();
                if steerings.is_empty() {
                    return false;
                }
                let content = steerings.join("\n");
                self.agent
                    .message_queue
                    .retain(|m| m.kind != crate::model::QueuedMessageKind::Steering);
                self.push_user_message(content);
                self.view.scroll = 0;
                self.messages_changed();
                true
            }
        }
    }

    pub(crate) fn dequeue(&mut self) {
        if let Some(msg) = self.agent.message_queue.pop() {
            self.input.input = msg.content;
            self.input.cursor_pos = self.input.input.len();
            self.mark_dirty();
        } else {
            self.input.input_flash = 3;
            self.mark_dirty();
        }
    }

    fn try_deliver_follow_up(&mut self) {
        if let Some(idx) = self
            .agent
            .message_queue
            .iter()
            .position(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
        {
            let content = self.agent.message_queue.remove(idx).content;
            self.push_user_message(content);
            self.view.scroll = 0;
            self.messages_changed();
        }
    }
}
