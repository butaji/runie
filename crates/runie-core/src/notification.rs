//! Notification DSL — fluent API for transient messages in the hints line.
//!
//! # Usage
//!
//! ```
//! use runie_core::notification;
//! use runie_core::AppState;
//!
//! let mut state = AppState::default();
//!
//! // Simple one-liners
//! notification::success("Theme switched", &mut state);
//! notification::warn("Read-only mode", &mut state);
//! notification::error("Connection failed", &mut state);
//!
//! // With duration (seconds)
//! notification::success_with_duration("Saved", 3.0).show(&mut state);
//! notification::error_persistent("Failed").show(&mut state); // shown until dismissed
//! ```

use crate::event::TransientLevel;
use crate::AppState;

pub struct Notification {
    message: String,
    level: TransientLevel,
    duration_secs: Option<f64>,
}

impl Notification {
    /// Set how long the notification stays visible.
    /// Default is determined by the current theme.
    pub fn duration(mut self, secs: f64) -> Self {
        self.duration_secs = Some(secs);
        self
    }

    /// Make the notification persist until user dismisses it.
    pub fn persistent(mut self) -> Self {
        self.duration_secs = None;
        self
    }

    /// Show this notification on the given state.
    pub fn show(self, state: &mut AppState) {
        *state.transient_message_mut() = Some(self.message);
        *state.transient_level_mut() = Some(self.level);
        *state.transient_until_mut() = self
            .duration_secs
            .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs_f64(secs));
        state.view_mut().dirty = true;
    }
}

// ─── Notification constructors ─────────────────────────────────────────────

fn make_notification(msg: impl Into<String>, level: TransientLevel) -> Notification {
    Notification { message: msg.into(), level, duration_secs: None }
}

/// Show a success notification (green, {ok} badge).
pub fn success(msg: impl Into<String>, state: &mut AppState) {
    make_notification(msg, TransientLevel::Success).show(state);
}

/// Show a warning notification (amber, {warn} badge).
pub fn warn(msg: impl Into<String>, state: &mut AppState) {
    make_notification(msg, TransientLevel::Warning).show(state);
}

/// Show an error notification (red, {error} badge).
pub fn error(msg: impl Into<String>, state: &mut AppState) {
    make_notification(msg, TransientLevel::Error).show(state);
}

/// Show a neutral notification (panel bg, no badge).
pub fn info(msg: impl Into<String>, state: &mut AppState) {
    make_notification(msg, TransientLevel::Info).show(state);
}

/// Create a success notification for fluent API usage.
pub fn success_with_duration(msg: impl Into<String>, secs: f64) -> Notification {
    make_notification(msg, TransientLevel::Success).duration(secs)
}

/// Create an error notification for fluent API usage.
pub fn error_persistent(msg: impl Into<String>) -> Notification {
    make_notification(msg, TransientLevel::Error).persistent()
}

/// Dismiss any active notification.
pub fn dismiss(state: &mut AppState) {
    *state.transient_message_mut() = None;
    *state.transient_level_mut() = None;
    *state.transient_until_mut() = None;
    state.view_mut().dirty = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_notification_sets_correct_level() {
        let mut state = AppState::default();
        success("Done", &mut state);
        assert_eq!(state.transient_message(), Some(&"Done".to_string()));
        assert_eq!(state.transient_level(), Some(TransientLevel::Success));
    }

    #[test]
    fn warn_notification_sets_correct_level() {
        let mut state = AppState::default();
        warn("Careful", &mut state);
        assert_eq!(state.transient_level(), Some(TransientLevel::Warning));
    }

    #[test]
    fn error_notification_sets_correct_level() {
        let mut state = AppState::default();
        error("Oops", &mut state);
        assert_eq!(state.transient_level(), Some(TransientLevel::Error));
    }

    #[test]
    fn dismiss_clears_notification() {
        let mut state = AppState::default();
        success("Done", &mut state);
        dismiss(&mut state);
        assert!(state.transient_message().is_none());
        assert!(state.transient_level().is_none());
    }

    #[test]
    fn notification_with_duration_sets_until() {
        let mut state = AppState::default();
        success_with_duration("Saved", 5.0).show(&mut state);
        assert!(state.transient_until().is_some());
    }

    #[test]
    fn fluent_api_chain() {
        let mut state = AppState::default();
        success_with_duration("Saved", 2.0).show(&mut state);
        assert_eq!(state.transient_message(), Some(&"Saved".to_string()));
    }
}
