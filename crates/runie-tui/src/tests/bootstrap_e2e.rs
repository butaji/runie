//! E2E tests for the testable TUI bootstrap.
//!
//! These tests verify that `TuiRuntime::builder()` with `BackendType::Test`
//! and keystroke sequences work correctly.

use ratatui::backend::TestBackend;
use runie_core::AppState;
use crate::bootstrap::{BackendType, Keystroke, TuiRuntime};
use std::sync::Arc;

// ─── Layer 2: Event Handling Tests ─────────────────────────────────────────

/// Verify keystroke DSL produces expected events through the keymap.
#[test]
fn keystroke_dsl_produces_expected_events() {
    let bindings = std::collections::HashMap::new();

    // Type "Hi" followed by Ctrl+C (quit)
    let keystrokes = vec![
        Keystroke::Char('H'),
        Keystroke::Char('i'),
        Keystroke::CtrlC,
    ];

    let events: Vec<_> = keystrokes
        .iter()
        .filter_map(|ks| ks.to_event(&bindings))
        .collect();

    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], runie_core::Event::Input('H')));
    assert!(matches!(events[1], runie_core::Event::Input('i')));
    assert!(matches!(events[2], runie_core::Event::Quit));
}

/// Verify raw events bypass keymap conversion.
#[test]
fn raw_event_bypasses_keymap() {
    use runie_core::Event;
    let bindings = std::collections::HashMap::new();

    // Submit event should be returned as-is
    let ks = Keystroke::RawEvent(Event::Submit);
    let event = ks.to_event(&bindings);
    assert!(matches!(event, Some(Event::Submit)));
}

// ─── Layer 3: Rendering Tests ───────────────────────────────────────────────

/// Verify the test backend can be configured correctly.
#[test]
fn test_backend_configured_correctly() {
    let backend = TestBackend::new(80, 24);
    let runtime = TuiRuntime::builder()
        .backend(BackendType::Test(backend))
        .build();

    assert!(matches!(runtime.backend(), BackendType::Test(_)));
}

/// Verify keystroke sequence is preserved in runtime.
#[test]
fn keystroke_sequence_preserved_in_runtime() {
    let keystrokes = vec![
        Keystroke::Char('t'),
        Keystroke::Char('e'),
        Keystroke::Char('s'),
        Keystroke::Char('t'),
        Keystroke::Enter,
    ];

    let runtime = TuiRuntime::builder()
        .keystrokes(keystrokes.clone())
        .build();

    assert_eq!(runtime.keystrokes().len(), 5);
}

// ─── Layer 4: E2E Bootstrap Test ────────────────────────────────────────────

/// Verify TuiRuntime can be built with all options.
///
/// Note: This is a compile-time verification test. Actual runtime execution
/// requires a more complex setup with actors and is tested separately in
/// integration tests.
#[test]
fn tui_runtime_builder_with_all_options() {
    let backend = TestBackend::new(120, 40);

    let runtime = TuiRuntime::builder()
        .backend(BackendType::Test(backend))
        .keystrokes(vec![
            Keystroke::Char('H'),
            Keystroke::Char('e'),
            Keystroke::Char('l'),
            Keystroke::Char('l'),
            Keystroke::Char('o'),
            Keystroke::Enter,
        ])
        .build();

    // Verify runtime was constructed correctly
    assert!(matches!(runtime.backend(), BackendType::Test(_)));
    assert_eq!(runtime.keystrokes().len(), 6);
}

/// Verify provider factory can be configured.
///
/// This test verifies that a custom provider factory can be passed to the runtime.
/// The actual factory behavior is tested in the provider tests.
#[test]
fn tui_runtime_with_custom_provider_factory() {
    use runie_provider::BuiltProviderFactory;

    let factory = Arc::new(BuiltProviderFactory::new());
    let backend = TestBackend::new(80, 24);

    let runtime = TuiRuntime::builder()
        .provider_factory(factory.clone())
        .backend(BackendType::Test(backend))
        .build();

    // The runtime was created successfully with the custom factory
    // Actual provider usage is tested in provider integration tests
    assert!(matches!(runtime.backend(), BackendType::Test(_)));
}
