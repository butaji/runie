//! Focus/idle gating for terminal notifications.

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct FocusTracker {
    focused: bool,
    lost_at: Option<Instant>,
    idle_threshold: Duration,
}

impl FocusTracker {
    pub fn new(idle_threshold: Duration) -> Self {
        Self { focused: true, lost_at: None, idle_threshold }
    }

    pub fn on_focus_lost(&mut self) {
        if self.focused {
            self.focused = false;
            self.lost_at = Some(Instant::now());
        }
    }

    pub fn on_focus_gained(&mut self) {
        self.focused = true;
        self.lost_at = None;
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    /// Grok's `unfocused` condition: do not notify while focused, and wait
    /// for the configured idle threshold after focus is lost.
    pub fn should_notify(&self) -> bool {
        !self.focused
            && self
                .lost_at
                .is_some_and(|at| at.elapsed() >= self.idle_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_tracker_suppresses_notifications() {
        assert!(!FocusTracker::new(Duration::ZERO).should_notify());
    }

    #[test]
    fn unfocused_tracker_waits_for_threshold() {
        let mut tracker = FocusTracker::new(Duration::from_secs(60));
        tracker.on_focus_lost();
        assert!(!tracker.should_notify());
        tracker.on_focus_gained();
        assert!(!tracker.should_notify());
    }

    #[test]
    fn focus_gain_resets_idle_window() {
        let mut tracker = FocusTracker::new(Duration::ZERO);
        tracker.on_focus_lost();
        assert!(tracker.should_notify());
        tracker.on_focus_gained();
        assert!(!tracker.should_notify());
    }
}
