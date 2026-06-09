use crate::model::{AppState, ChatMessage, DeliveryMode, Role};
use super::now;

impl AppState {
    pub(crate) fn queue_follow_up(&mut self) {
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        self.cursor_pos = 0;
        if content.is_empty() {
            return;
        }
        self.message_queue.push(crate::model::QueuedMessage {
            content,
            kind: crate::model::QueuedMessageKind::FollowUp,
        });
        self.scroll = 0;
        self.mark_dirty();
    }

    pub(crate) fn abort_queue(&mut self) {
        if self.at_suggestions.take().is_some() {
            self.at_selected = None;
            self.last_at_query = None;
            self.mark_dirty();
            return;
        }
        for msg in self.message_queue.drain(..).rev() {
            if !self.input.is_empty() {
                self.input.push('\n');
            }
            self.input.push_str(&msg.content);
        }
        self.mark_dirty();
    }

    pub(crate) fn deliver_queued(&mut self) {
        if self.message_queue.is_empty() {
            return;
        }
        // Try to deliver steering (batch or single depending on mode)
        if self.try_deliver_steering() {
            // Steering delivered - in "All" mode, also try follow-ups
            if self.follow_up_mode == DeliveryMode::All && self.has_follow_ups() {
                self.try_deliver_follow_ups_all();
            }
            return;
        }
        // No steering in queue - try follow-ups
        self.try_deliver_follow_up();
    }

    fn has_follow_ups(&self) -> bool {
        self.message_queue.iter().any(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
    }

    /// Deliver all follow-ups in batch mode.
    fn try_deliver_follow_ups_all(&mut self) {
        let follow_ups: Vec<String> = self.message_queue.iter()
            .filter(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
            .map(|m| m.content.clone())
            .collect();

        if follow_ups.is_empty() {
            return;
        }

        let content = follow_ups.join("\n");

        // Remove all follow-ups from queue
        self.message_queue.retain(|m| m.kind != crate::model::QueuedMessageKind::FollowUp);

        self.push_user_message(content);
        self.scroll = 0;
        self.messages_changed();
    }

    fn push_user_message(&mut self, content: String) {
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: Role::User,
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push_back((content, id));
    }

    fn try_deliver_steering(&mut self) -> bool {
        match self.steering_mode {
            DeliveryMode::OneAtATime => {
                if let Some(idx) = self.message_queue.iter().position(|m| m.kind == crate::model::QueuedMessageKind::Steering) {
                    let content = self.message_queue.remove(idx).content;
                    self.push_user_message(content);
                    self.scroll = 0;
                    self.messages_changed();
                    true
                } else {
                    false
                }
            }
            DeliveryMode::All => {
                let steerings: Vec<String> = self.message_queue.iter()
                    .filter(|m| m.kind == crate::model::QueuedMessageKind::Steering)
                    .map(|m| m.content.clone())
                    .collect();
                if steerings.is_empty() {
                    return false;
                }
                let content = steerings.join("\n");
                self.message_queue.retain(|m| m.kind != crate::model::QueuedMessageKind::Steering);
                self.push_user_message(content);
                self.scroll = 0;
                self.messages_changed();
                true
            }
        }
    }

    fn try_deliver_follow_up(&mut self) {
        if let Some(idx) = self.message_queue.iter().position(|m| m.kind == crate::model::QueuedMessageKind::FollowUp) {
            let content = self.message_queue.remove(idx).content;
            self.push_user_message(content);
            self.scroll = 0;
            self.messages_changed();
        }
    }
}
