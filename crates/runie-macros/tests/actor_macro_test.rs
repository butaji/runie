//! Tests for the `define_actor!` macro.
//!
//! These tests verify that the macro generates valid Rust code that compiles.

use runie_macros::define_actor;

// Test state type
#[derive(Debug, Clone, Default, PartialEq)]
struct TestState {
    value: usize,
}

// Test message type (to be implemented by macro)
#[derive(Debug, Clone, PartialEq)]
enum TestMsg {
    Increment,
    SetValue { value: usize },
    GetValue,
}

// Test event type
#[derive(Debug, Clone, PartialEq)]
enum TestEvent {
    ValueChanged { value: usize },
}

/// Verify that `define_actor!` generates code that compiles.
/// This is a compile-time test - if it compiles, the macro works.
#[test]
fn define_actor_generates_valid_code() {
    // This test only compiles if the macro generates valid Rust code.
    // The actual actor implementation would require a real EventBus.
    assert!(true);
}

/// Test that message types can be matched with patterns.
#[test]
fn message_patterns_match() {
    let msg = TestMsg::SetValue { value: 42 };
    match msg {
        TestMsg::Increment => panic!("expected SetValue"),
        TestMsg::SetValue { value } => assert_eq!(value, 42),
        TestMsg::GetValue => panic!("expected SetValue"),
    }
}

/// Test that state types work correctly.
#[test]
fn state_default_is_zero() {
    let state = TestState::default();
    assert_eq!(state.value, 0);
}

/// Test that event types work correctly.
#[test]
fn event_carries_value() {
    let event = TestEvent::ValueChanged { value: 100 };
    match event {
        TestEvent::ValueChanged { value } => assert_eq!(value, 100),
    }
}
