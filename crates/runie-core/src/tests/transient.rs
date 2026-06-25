//! Transient message tests (Layer 1: state logic)

use crate::event::SystemEvent;

use crate::model::Role;
use crate::tests::fresh_state;

#[test]
fn transient_message_sets_content_and_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "hello".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(state.transient_message, Some("hello".to_string()));
    assert!(
        state.transient_until.is_some(),
        "Transient message should have expiry"
    );
}

#[test]
fn transient_error_sets_content_without_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientError {
        content: "err".to_string(),
    });
    assert_eq!(state.transient_message, Some("err".to_string()));
    assert!(
        state.transient_until.is_none(),
        "Transient error should NOT have expiry"
    );
}

#[test]
fn clear_transient_unsets_message() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientError {
        content: "err".to_string(),
    });
    state.update(SystemEvent::ClearTransient);
    assert!(state.transient_message.is_none());
    assert!(state.transient_until.is_none());
}

#[test]
fn transient_message_overwrites_existing() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "first".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    state.update(SystemEvent::TransientMessage {
        content: "second".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(state.transient_message, Some("second".to_string()));
}

#[test]
fn transient_message_in_snapshot() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "snap".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    state.ensure_fresh();
    let snap = state.snapshot();
    assert_eq!(snap.transient_message, Some("snap".to_string()));
    assert_eq!(
        snap.transient_level,
        Some(crate::event::TransientLevel::Info)
    );
}

#[test]
fn transient_success_has_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "ok".to_string(),
        level: crate::event::TransientLevel::Success,
    });
    assert!(
        state.transient_until.is_some(),
        "Success message should have expiry"
    );
}

#[test]
fn transient_error_has_no_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientError {
        content: "error".to_string(),
    });
    assert!(
        state.transient_until.is_none(),
        "Error message should NOT have expiry"
    );
}

#[test]
fn transient_system_message_has_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::SystemMessage {
        content: "info".to_string(),
    });
    assert!(
        state.transient_until.is_some(),
        "System message should have expiry"
    );
}

#[test]
fn transient_message_does_not_add_to_feed() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "notify".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys.len(), 0, "notify should not add to message feed");
}

#[test]
fn transient_message_with_different_levels() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "info".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Info)
    );

    state.update(SystemEvent::ClearTransient);
    state.update(SystemEvent::TransientMessage {
        content: "warn".to_string(),
        level: crate::event::TransientLevel::Warning,
    });
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning)
    );
}

#[test]
fn transient_expiry_time_is_reasonable() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "test".to_string(),
        level: crate::event::TransientLevel::Info,
    });
    if let Some(expiry) = state.transient_until {
        let now = std::time::Instant::now();
        assert!(
            expiry.duration_since(now).as_secs() <= 10,
            "Expiry should be reasonable"
        );
    }
}
