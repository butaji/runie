//! Snapshot optimization tests (Layer 1–3)

use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::snapshot::Snapshot;
use crate::Event;
use std::sync::Arc;

// Layer 1 — State/Logic

#[test]
fn test_snapshot_contains_expected_fields() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap = state.snapshot();
    assert!(!snap.elements.is_empty(), "elements should be present");
    assert!(
        !snap.line_counts.is_empty(),
        "line_counts should be present"
    );
    assert_eq!(snap.input, state.input.input);
    assert_eq!(snap.cursor_pos, state.input.cursor_pos);
    assert_eq!(snap.provider, state.config.current_provider);
    assert_eq!(snap.model, state.config.current_model);
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_snapshot_is_send_sync() {
    assert_send_sync::<Snapshot>();
}

// Layer 2 — Event Handling

#[test]
fn test_event_triggers_snapshot_update() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let count1 = snap1.elements.len();

    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello back".into(),
    });
    state.ensure_fresh();

    let snap2 = state.snapshot();
    let count2 = snap2.elements.len();

    assert!(
        count2 > count1,
        "Snapshot should reflect new elements after agent response"
    );
}

// Layer 3 — Rendering

#[test]
fn test_render_receives_valid_snapshot() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    let snap = state.snapshot();
    let region = snap.visible(0, 5);
    assert!(
        region.len() <= snap.elements.len(),
        "Visible region should not exceed total elements"
    );
}

// Arc-specific optimization tests

#[test]
fn test_elements_uses_arc() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();

    // Both snapshots should share the same Arc allocation
    assert!(
        Arc::ptr_eq(&snap1.elements, &snap2.elements),
        "Elements should be Arc-shared between snapshots when unchanged"
    );
    assert!(
        Arc::ptr_eq(&snap1.line_counts, &snap2.line_counts),
        "Line counts should be Arc-shared between snapshots when unchanged"
    );
}

#[test]
fn test_arc_pointer_stability_after_state_mutation() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "first".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    let snap1 = state.snapshot();
    let ptr1 = Arc::as_ptr(&snap1.elements);

    // Mutate state but NOT messages (e.g., cursor move)
    state.update(crate::Event::Input('x'));
    state.ensure_fresh();

    let snap2 = state.snapshot();
    let ptr2 = Arc::as_ptr(&snap2.elements);

    // Elements Arc should still point to same allocation because messages didn't change
    assert_eq!(
        ptr1, ptr2,
        "Elements Arc should be stable when messages are unchanged"
    );
}

#[test]
fn test_auth_providers_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.auth_providers, &snap2.auth_providers),
        "auth_providers should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_settings_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.settings_items, &snap2.settings_items),
        "settings_items should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_session_tree_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.session_tree_items, &snap2.session_tree_items),
        "session_tree_items should be Arc-shared across unchanged snapshots"
    );
}

#[test]
fn test_palette_items_cached() {
    let mut state = AppState::default();
    state.ensure_fresh();

    let snap1 = state.snapshot();
    let snap2 = state.snapshot();
    assert!(
        Arc::ptr_eq(&snap1.palette_items, &snap2.palette_items),
        "palette_items should be Arc-shared across unchanged snapshots"
    );
}

// Layer 1 — State/Logic

#[test]
fn test_refresh_after_message_change_updates_flags() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });

    let gen_before = state.view().message_gen;
    state.refresh_after_message_change();

    // Verify messages_changed behavior: message_gen should be bumped
    assert!(state.view().message_gen > gen_before);

    // Verify ensure_fresh was called: cache should be rebuilt, dirty should be cleared
    assert!(!state.view().dirty);
    // View cache is now built into Snapshot, not stored in ViewState
    let snap = state.snapshot();
    assert!(!snap.elements.is_empty());
}
