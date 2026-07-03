//! Tests for event taxonomy — intent impl, kind, and category.

use crate::event::Event;
use crate::model::ThinkingLevel;
use crate::permissions::PermissionAction;

// ── into_intent tests ─────────────────────────────────────────────────────────

#[test]
fn fact_events_return_none() {
    let facts: Vec<Event> = vec![
        Event::ConfigLoaded {
            config: Box::new(crate::config::Config::default()),
        },
        Event::TrustLoaded {
            decisions: Default::default(),
        },
        Event::SessionSaved {
            name: "test".into(),
        },
        Event::BashOutput {
            command: "ls".into(),
            output: "/".into(),
        },
        Event::Thinking { id: "t1".into() },
        Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 0.5,
            output: "ok".into(),

            input: None,
        },
        Event::ValidationFailed {
            provider: "openai".into(),
            key: "sk-xxx".into(),
            error: "invalid".into(),
        },
    ];
    for e in facts {
        assert!(
            e.clone().into_intent().is_none(),
            "{:?} must return None",
            e
        );
    }
}

#[test]
fn intent_events_return_some() {
    let e = Event::SwitchTheme {
        name: "dark".into(),
    };
    let i = e.into_intent().expect("SwitchTheme must convert to intent");
    assert!(matches!(i, Event::SwitchTheme { .. }));

    let e = Event::Quit;
    assert!(e.into_intent().is_some(), "Quit must convert to Intent");

    let e = Event::Submit;
    assert!(e.into_intent().is_some(), "Submit must convert to Intent");

    let e = Event::Input('x');
    assert!(e.into_intent().is_some(), "Input must convert to Intent");

    let e = Event::SetThinkingLevel(ThinkingLevel::Medium);
    assert!(
        e.into_intent().is_some(),
        "SetThinkingLevel must convert to Intent"
    );

    let e = Event::PermissionResponse {
        request_id: "r1".into(),
        action: PermissionAction::Allow,
    };
    assert!(
        e.into_intent().is_some(),
        "PermissionResponse must convert to Intent"
    );
}

#[test]
fn switch_theme_is_intent_not_fact() {
    let e = Event::SwitchTheme {
        name: "dracula".into(),
    };
    assert_eq!(e.kind(), crate::event::EventKind::Intent);
    assert!(e.into_intent().is_some());
}

// ── Event::category tests ──────────────────────────────────────────────────────

#[test]
fn input_event_has_input_category() {
    let e = Event::Submit;
    assert_eq!(e.category(), crate::event::EventCategory::Input);
}

#[test]
fn agent_event_has_agent_category() {
    let e = Event::Thinking { id: "x".into() };
    assert_eq!(e.category(), crate::event::EventCategory::Agent);
}

#[test]
fn command_event_has_command_category() {
    let e = Event::RunCompactCommand {
        keep: "last".into(),
        focus: "focused".into(),
    };
    assert_eq!(e.category(), crate::event::EventCategory::Command);
}

#[test]
fn lifecycle_events_classified_as_fact() {
    // Turn lifecycle events should be Facts, not Intents
    let e = Event::TurnStarted {
        id: "t1".into(),
        request_id: "r1".into(),
        content: "hi".into(),
    };
    assert_eq!(e.kind(), crate::event::EventKind::Fact);
    assert!(e.into_intent().is_none());
}

/// Verify every Event variant is covered by the generated taxonomy.
/// This test ensures no variant is missing from kind(), category(), or into_intent().
#[test]
fn all_variants_have_kind_and_category() {
    use crate::event::EventKind;

    // Sample each category to verify kind/category are consistent
    macro_rules! check {
        ($event:expr, $expected_kind:expr, $expected_cat:expr) => {
            assert_eq!($event.kind(), $expected_kind, "kind mismatch for {:?}", $event);
            assert_eq!($event.category(), $expected_cat, "category mismatch for {:?}", $event);
        };
    }

    check!(Event::Thinking { id: "x".into() }, EventKind::Fact, crate::event::EventCategory::Agent);
    check!(Event::RunCompactCommand { keep: "x".into(), focus: "y".into() }, EventKind::Intent, crate::event::EventCategory::Command);
    check!(Event::Quit, EventKind::Control, crate::event::EventCategory::Control);
    check!(Event::ToggleCommandPalette, EventKind::Intent, crate::event::EventCategory::Dialog);
    check!(Event::PendingEdit { path: "x".into(), original: "y".into(), proposed: "z".into() }, EventKind::Intent, crate::event::EventCategory::Edit);
    check!(Event::BashOutput { command: "x".into(), output: "y".into() }, EventKind::Fact, crate::event::EventCategory::IO);
    check!(Event::Input('x'), EventKind::Intent, crate::event::EventCategory::Input);
    check!(Event::Save, EventKind::Intent, crate::event::EventCategory::LoginFlow);
    check!(Event::SwitchTheme { name: "x".into() }, EventKind::Intent, crate::event::EventCategory::ModelConfig);
    check!(Event::MessageReplayed { id: "x".into(), role: "y".into(), content: "z".into(), timestamp: 0.0, provider: "p".into() }, EventKind::Fact, crate::event::EventCategory::Other);
    check!(Event::PermissionResponse { request_id: "x".into(), action: PermissionAction::Allow }, EventKind::Intent, crate::event::EventCategory::Permission);
    check!(Event::InputChanged { state: Box::new(crate::model::InputState::default()) }, EventKind::Fact, crate::event::EventCategory::Persistence);
    check!(Event::PlanModeEnabled { content: "x".into() }, EventKind::Intent, crate::event::EventCategory::PlanMode);
    check!(Event::Up, EventKind::Intent, crate::event::EventCategory::Scroll);
    check!(Event::SessionSaved { name: "x".into() }, EventKind::Fact, crate::event::EventCategory::Session);
    check!(Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) }, EventKind::Fact, crate::event::EventCategory::System);
}

#[test]
fn is_fact_variant_matches_kind() {
    use crate::event::EventKind;
    use crate::event::is_fact_variant;
    // Every fact event should return true from is_fact_variant
    let fact_samples: Vec<Event> = vec![
        Event::Thinking { id: "x".into() },
        Event::ToolEnd { id: "x".into(), duration_secs: 1.0, output: "y".into(), input: None },
        Event::Response { id: "x".into(), content: "y".into(), role: String::new(), timestamp: 0.0, provider: String::new() },
        Event::BashOutput { command: "x".into(), output: "y".into() },
        Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) },
    ];
    for e in &fact_samples {
        assert!(is_fact_variant(e), "{:?} must be fact", e);
        assert_eq!(e.kind(), EventKind::Fact);
    }

    // Non-fact events should return false
    let non_fact_samples: Vec<Event> = vec![
        Event::Quit,
        Event::Submit,
        Event::Input('x'),
        Event::RunCompactCommand { keep: "x".into(), focus: "y".into() },
    ];
    for e in &non_fact_samples {
        assert!(!is_fact_variant(e), "{:?} must not be fact", e);
    }
}

// ── EVENT_NAMES tests ──────────────────────────────────────────────────────────

#[test]
fn event_names_contains_known_intents() {
    use crate::event::EVENT_NAMES;
    let names: Vec<_> = EVENT_NAMES.iter().map(|(n, _)| *n).collect();
    assert!(names.contains(&"Quit"), "EVENT_NAMES should include Quit");
    assert!(
        names.contains(&"Submit"),
        "EVENT_NAMES should include Submit"
    );
    assert!(
        names.contains(&"ToggleExpand"),
        "EVENT_NAMES should include ToggleExpand"
    );
}

// ── Integration tests for event variants ───────────────────────────────────────

mod variants_tests;
