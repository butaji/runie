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
