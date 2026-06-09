use crate::model::{AppState, ChatMessage, DeliveryMode, Role};
use super::now;

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

    pub(crate) fn abort_queue(&mut self) {
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
            if self.follow_up_mode == DeliveryMode::All && self.has_follow_ups() {
                self.try_deliver_follow_ups_all();
            }
            return;
        }
        // No steering in queue - try follow-ups
        self.try_deliver_follow_up();
    }

    fn has_follow_ups(&self) -> bool {
        self.agent.message_queue.iter().any(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
    }

    /// Deliver all follow-ups in batch mode.
    fn try_deliver_follow_ups_all(&mut self) {
        let follow_ups: Vec<String> = self.agent.message_queue.iter()
            .filter(|m| m.kind == crate::model::QueuedMessageKind::FollowUp)
            .map(|m| m.content.clone())
            .collect();

        if follow_ups.is_empty() {
            return;
        }

        let content = follow_ups.join("\n");

        // Remove all follow-ups from queue
        self.agent.message_queue.retain(|m| m.kind != crate::model::QueuedMessageKind::FollowUp);

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
        match self.steering_mode {
            DeliveryMode::OneAtATime => {
                if let Some(idx) = self.agent.message_queue.iter().position(|m| m.kind == crate::model::QueuedMessageKind::Steering) {
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
                let steerings: Vec<String> = self.agent.message_queue.iter()
                    .filter(|m| m.kind == crate::model::QueuedMessageKind::Steering)
                    .map(|m| m.content.clone())
                    .collect();
                if steerings.is_empty() {
                    return false;
                }
                let content = steerings.join("\n");
                self.agent.message_queue.retain(|m| m.kind != crate::model::QueuedMessageKind::Steering);
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
        if let Some(idx) = self.agent.message_queue.iter().position(|m| m.kind == crate::model::QueuedMessageKind::FollowUp) {
            let content = self.agent.message_queue.remove(idx).content;
            self.push_user_message(content);
            self.view.scroll = 0;
            self.messages_changed();
        }
    }
}
