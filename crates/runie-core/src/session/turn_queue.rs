//! Pure turn queue for steering/follow-up delivery.
//!
//! Deduplicates delivery logic that previously existed in both
//! `RactorTurnActor` and `AppState` sync methods.
//!
//! Each pop method returns `Option<(content, count)>`. The caller uses
//! `count` to decide whether to emit events (count > 0) and uses `content`
//! to push into the request queue.

use crate::model::{DeliveryMode, QueuedMessage, QueuedMessageKind};

/// Result of a queue pop operation.
#[derive(Clone, Debug)]
pub struct PopResult {
    /// Concatenated content of all popped messages, joined by "\n".
    pub content: String,
    /// Number of messages actually popped (0 if queue was empty).
    pub count: usize,
}

impl PopResult {
    fn new(content: String, count: usize) -> Self {
        Self { content, count }
    }

    /// Returns `None` if nothing was popped (empty queue).
    pub fn maybe(content: String, count: usize) -> Option<Self> {
        if count == 0 {
            None
        } else {
            Some(Self::new(content, count))
        }
    }
}

/// Pure queue for steering and follow-up messages.
///
/// Replaces duplicate `try_deliver_steering` / `try_deliver_follow_up` /
/// `deliver_follow_ups_all` logic in `RactorTurnActor` and `AppState`.
#[derive(Clone, Debug)]
pub struct TurnQueue {
    messages: Vec<QueuedMessage>,
}

impl TurnQueue {
    /// Wrap an existing queue slice.
    pub fn new(messages: Vec<QueuedMessage>) -> Self {
        Self { messages }
    }

    /// Consumes self and returns the inner queue (for syncing back to caller).
    pub fn into_inner(self) -> Vec<QueuedMessage> {
        self.messages
    }

    /// Extract all messages, leaving self empty. Use for sync-back to caller.
    pub fn drain(&mut self) -> Vec<QueuedMessage> {
        std::mem::take(&mut self.messages)
    }

    /// Pop steering messages from the queue.
    ///
    /// - `OneAtATime`: pops the first steering message (count ≤ 1).
    /// - `All`: pops all steering messages and joins them (count ≥ 0).
    pub fn pop_steering(&mut self, mode: DeliveryMode) -> Option<PopResult> {
        use DeliveryMode::*;
        let kind = QueuedMessageKind::Steering;
        match mode {
            OneAtATime => {
                if let Some(idx) = self.messages.iter().position(|m| m.kind == kind) {
                    let content = self.messages.remove(idx).content;
                    PopResult::maybe(content, 1)
                } else {
                    None
                }
            }
            All => {
                let steerings: Vec<_> = self
                    .messages
                    .iter()
                    .filter(|m| m.kind == kind)
                    .map(|m| m.content.clone())
                    .collect();
                if steerings.is_empty() {
                    return None;
                }
                let count = steerings.len();
                let content = steerings.join("\n");
                self.messages.retain(|m| m.kind != kind);
                Some(PopResult::new(content, count))
            }
        }
    }

    /// Pop a single follow-up message from the queue.
    ///
    /// - `OneAtATime`: pops the first follow-up (count ≤ 1).
    /// - `All`: pops all follow-ups and joins them (count ≥ 0).
    pub fn pop_follow_up(&mut self, mode: DeliveryMode) -> Option<PopResult> {
        use DeliveryMode::*;
        let kind = QueuedMessageKind::FollowUp;
        match mode {
            OneAtATime => {
                if let Some(idx) = self.messages.iter().position(|m| m.kind == kind) {
                    let content = self.messages.remove(idx).content;
                    PopResult::maybe(content, 1)
                } else {
                    None
                }
            }
            All => self.pop_all_follow_ups(),
        }
    }

    /// Pop all follow-up messages from the queue.
    pub fn pop_all_follow_ups(&mut self) -> Option<PopResult> {
        let kind = QueuedMessageKind::FollowUp;
        let follow_ups: Vec<_> = self
            .messages
            .iter()
            .filter(|m| m.kind == kind)
            .map(|m| m.content.clone())
            .collect();
        if follow_ups.is_empty() {
            return None;
        }
        let count = follow_ups.len();
        let content = follow_ups.join("\n");
        self.messages.retain(|m| m.kind != kind);
        Some(PopResult::new(content, count))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::QueuedMessageKind::*;

    fn steering(content: &str) -> QueuedMessage {
        QueuedMessage {
            content: content.into(),
            kind: Steering,
        }
    }
    fn follow_up(content: &str) -> QueuedMessage {
        QueuedMessage {
            content: content.into(),
            kind: FollowUp,
        }
    }

    #[test]
    fn pop_steering_one_at_a_time_returns_one() {
        let mut q = TurnQueue::new(vec![steering("a"), steering("b")]);
        let r = q.pop_steering(DeliveryMode::OneAtATime);
        assert!(r.is_some());
        assert_eq!(r.unwrap().content, "a");
        assert_eq!(q.messages.len(), 1); // "b" remains
    }

    #[test]
    fn pop_steering_one_at_a_time_empty() {
        let mut q = TurnQueue::new(vec![follow_up("x")]);
        let r = q.pop_steering(DeliveryMode::OneAtATime);
        assert!(r.is_none());
        assert_eq!(q.messages.len(), 1); // unchanged
    }

    #[test]
    fn pop_steering_all_returns_all_joined() {
        let mut q = TurnQueue::new(vec![steering("a"), follow_up("x"), steering("b")]);
        let r = q.pop_steering(DeliveryMode::All);
        let r = r.unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.content, "a\nb");
        assert_eq!(q.messages.len(), 1); // only follow-up remains
    }

    #[test]
    fn pop_steering_all_empty() {
        let mut q = TurnQueue::new(vec![follow_up("x")]);
        let r = q.pop_steering(DeliveryMode::All);
        assert!(r.is_none());
    }

    #[test]
    fn pop_follow_up_one_at_a_time() {
        let mut q = TurnQueue::new(vec![follow_up("x"), follow_up("y")]);
        let r = q.pop_follow_up(DeliveryMode::OneAtATime);
        assert_eq!(r.unwrap().content, "x");
        assert_eq!(q.messages.len(), 1);
    }

    #[test]
    fn pop_follow_up_one_at_a_time_empty() {
        let mut q = TurnQueue::new(vec![steering("a")]);
        let r = q.pop_follow_up(DeliveryMode::OneAtATime);
        assert!(r.is_none());
    }

    #[test]
    fn pop_follow_up_all_returns_all() {
        let mut q = TurnQueue::new(vec![follow_up("x"), steering("a"), follow_up("y")]);
        let r = q.pop_follow_up(DeliveryMode::All);
        let r = r.unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.content, "x\ny");
        assert_eq!(q.messages.len(), 1); // only steering remains
    }

    #[test]
    fn pop_all_follow_ups_clears_follow_ups() {
        let mut q = TurnQueue::new(vec![follow_up("x"), follow_up("y"), steering("a")]);
        let r = q.pop_all_follow_ups().unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.content, "x\ny");
        assert_eq!(q.messages.len(), 1); // steering remains
    }

    #[test]
    fn pop_all_follow_ups_empty() {
        let mut q = TurnQueue::new(vec![steering("a")]);
        let r = q.pop_all_follow_ups();
        assert!(r.is_none());
    }

    #[test]
    fn drain_returns_all() {
        let mut q = TurnQueue::new(vec![steering("a"), follow_up("b")]);
        let msgs = q.drain();
        assert_eq!(msgs.len(), 2);
        assert!(q.messages.is_empty());
    }

    #[test]
    fn queue_empty_after_clear() {
        let mut q = TurnQueue::new(vec![steering("a"), follow_up("b")]);
        let _ = q.pop_steering(DeliveryMode::All);
        let _ = q.pop_follow_up(DeliveryMode::All);
        assert!(q.messages.is_empty());
    }
}
