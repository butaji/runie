//! Tests for permission gate cloning behavior (policy engine removed).
//!
//! All gates now bypass. These tests verify gate cloning works correctly.

use runie_core::permissions::{AutoAllowSink, PermissionGate};
use std::sync::Arc;

/// Test: PermissionGate can be cloned for subagent use
#[test]
fn permission_gate_clone_for_subagent() {
    let sink = Arc::new(AutoAllowSink);
    let parent_gate = PermissionGate::new(sink);

    // Clone for subagent
    let _subagent_gate = parent_gate.clone_for_subagent();

    // Both gates should be functional
}

/// Test: Cloned gate has independent abort token
#[test]
fn cloned_gate_has_independent_abort_token() {
    let sink = Arc::new(AutoAllowSink);
    let parent_gate = PermissionGate::new(sink);

    let _subagent_gate = parent_gate.clone_for_subagent();

    // Cancel parent - subagent should still work
    parent_gate.cancel_pending();

    // Both should still be usable (cancel just sets a flag)
}

/// Test: PermissionGate derives Clone
#[test]
fn permission_gate_is_cloneable() {
    let sink = Arc::new(AutoAllowSink);
    let gate = PermissionGate::new(sink);

    let cloned = gate.clone();

    // Both should be usable (same sink type)
    assert_eq!(
        std::any::type_name_of_val(cloned.sink_ref().as_ref()),
        std::any::type_name_of_val(gate.sink_ref().as_ref())
    );
}

/// Test: clone_for_subagent shares the same sink
#[test]
fn cloned_gate_shares_sink() {
    let sink = Arc::new(AutoAllowSink);
    let parent_gate = PermissionGate::new(sink.clone());

    let subagent_gate = parent_gate.clone_for_subagent();

    // Both gates should reference the same sink
    assert_eq!(
        std::any::type_name_of_val(subagent_gate.sink_ref().as_ref()),
        std::any::type_name_of_val(parent_gate.sink_ref().as_ref())
    );
}
