use crate::model::{AppState, ChatMessage, Role};
use super::now;

impl AppState {
    pub(crate) fn queue_follow_up(&mut self) {
        if self.input.is_empty() {
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
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
        if self.try_deliver_steering() {
            return;
        }
        self.try_deliver_follow_up();
    }

    fn try_deliver_steering(&mut self) -> bool {
        let steering: Vec<_> = self.message_queue.iter().enumerate()
            .filter(|(_, m)| m.kind == crate::model::QueuedMessageKind::Steering)
            .map(|(i, _)| i)
            .collect();
        if steering.is_empty() {
            return false;
        }
        let idx = steering[0];
        let msg = self.message_queue.remove(idx);
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: Role::User,
            content: msg.content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push_back((msg.content, id));
        self.scroll = 0;
        self.messages_changed();
        true
    }

    fn try_deliver_follow_up(&mut self) {
        let follow_up: Vec<_> = self.message_queue.iter().enumerate()
            .filter(|(_, m)| m.kind == crate::model::QueuedMessageKind::FollowUp)
            .map(|(i, _)| i)
            .collect();
        if follow_up.is_empty() {
            return;
        }
        let idx = follow_up[0];
        let msg = self.message_queue.remove(idx);
        let id = self.next_id();
        self.messages.push(ChatMessage {
            role: Role::User,
            content: msg.content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        self.request_queue.push_back((msg.content, id));
        self.scroll = 0;
        self.messages_changed();
    }
}
