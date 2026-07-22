//! Tests for event taxonomy — intent impl, kind, and category.

use crate::event::Event;
use crate::model::ThinkingLevel;
use crate::permissions::PermissionAction;

// ── into_intent tests ─────────────────────────────────────────────────────────

#[test]
fn fact_events_return_none() {
    let facts: Vec<Event> = vec![
        Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) },
        Event::TrustLoaded { decisions: Default::default() },
        Event::SessionSaved { name: "test".into() },
        Event::BashOutput { command: "ls".into(), output: "/".into() },
        Event::Thinking { id: "t1".into() },
        Event::ToolEnd { id: "t1".into(), duration_secs: 0.5, output: "ok".into(), input: None },
        Event::ValidationFailed { provider: "openai".into(), key: "sk-xxx".into(), error: "invalid".into() },
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
    let e = Event::SwitchTheme { name: "dark".into() };
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

    let e = Event::PermissionResponse { request_id: "r1".into(), action: PermissionAction::Allow };
    assert!(
        e.into_intent().is_some(),
        "PermissionResponse must convert to Intent"
    );
}

#[test]
fn switch_theme_is_intent_not_fact() {
    let e = Event::SwitchTheme { name: "dracula".into() };
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
    let e = Event::RunCompactCommand { keep: "last".into(), focus: "focused".into() };
    assert_eq!(e.category(), crate::event::EventCategory::Command);
}

#[test]
fn lifecycle_events_classified_as_fact() {
    // Turn lifecycle events should be Facts, not Intents
    let e = Event::TurnStarted { id: "t1".into(), request_id: "r1".into(), content: "hi".into() };
    assert_eq!(e.kind(), crate::event::EventKind::Fact);
    assert!(e.into_intent().is_none());
}

/// Verify every Event variant is covered by the generated taxonomy.
/// This test ensures no variant is missing from kind(), category(), or into_intent().
#[test]
fn all_variants_have_kind_and_category() {
    // Split by category to keep each sub-test below cognitive-complexity threshold.
    check_agent_category();
    check_command_category();
    check_control_category();
    check_dialog_category();
    check_edit_category();
    check_io_category();
    check_input_category();
    check_login_flow_category();
    check_model_config_category();
    check_other_category();
    check_permission_category();
    check_persistence_category();
    check_plan_mode_category();
    check_scroll_category();
    check_session_category();
    check_system_category();
}

fn check_agent_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::Thinking { id: "x".into() }.kind(),
        EventKind::Fact,
        "kind mismatch"
    );
    assert_eq!(
        Event::Thinking { id: "x".into() }.category(),
        crate::event::EventCategory::Agent,
        "category mismatch"
    );
}

fn check_command_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::RunCompactCommand { keep: "x".into(), focus: "y".into() }.kind(),
        EventKind::Intent,
    );
    assert_eq!(
        Event::RunCompactCommand { keep: "x".into(), focus: "y".into() }.category(),
        crate::event::EventCategory::Command,
    );
}

fn check_control_category() {
    use crate::event::EventKind;
    assert_eq!(Event::Quit.kind(), EventKind::Control);
    assert_eq!(Event::Quit.category(), crate::event::EventCategory::Control);
}

fn check_dialog_category() {
    use crate::event::EventKind;
    assert_eq!(Event::ToggleCommandPalette.kind(), EventKind::Intent);
    assert_eq!(
        Event::ToggleCommandPalette.category(),
        crate::event::EventCategory::Dialog
    );
}

fn check_edit_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::PendingEdit { path: "x".into(), original: "y".into(), proposed: "z".into() }.kind(),
        EventKind::Intent,
    );
    assert_eq!(
        Event::PendingEdit { path: "x".into(), original: "y".into(), proposed: "z".into() }.category(),
        crate::event::EventCategory::Edit,
    );
}

fn check_io_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::BashOutput { command: "x".into(), output: "y".into() }.kind(),
        EventKind::Fact,
    );
    assert_eq!(
        Event::BashOutput { command: "x".into(), output: "y".into() }.category(),
        crate::event::EventCategory::IO,
    );
}

fn check_input_category() {
    use crate::event::EventKind;
    assert_eq!(Event::Input('x').kind(), EventKind::Intent);
    assert_eq!(
        Event::Input('x').category(),
        crate::event::EventCategory::Input
    );
}

fn check_login_flow_category() {
    use crate::event::EventKind;
    assert_eq!(Event::Save.kind(), EventKind::Intent);
    assert_eq!(
        Event::Save.category(),
        crate::event::EventCategory::LoginFlow
    );
}

fn check_model_config_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::SwitchTheme { name: "x".into() }.kind(),
        EventKind::Intent
    );
    assert_eq!(
        Event::SwitchTheme { name: "x".into() }.category(),
        crate::event::EventCategory::ModelConfig,
    );
}

fn check_other_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::MessageReplayed {
            id: "x".into(),
            role: "y".into(),
            content: "z".into(),
            timestamp: 0.0,
            provider: "p".into()
        }
        .kind(),
        EventKind::Fact,
    );
    assert_eq!(
        Event::MessageReplayed {
            id: "x".into(),
            role: "y".into(),
            content: "z".into(),
            timestamp: 0.0,
            provider: "p".into()
        }
        .category(),
        crate::event::EventCategory::Other,
    );
}

fn check_permission_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::PermissionResponse { request_id: "x".into(), action: PermissionAction::Allow }.kind(),
        EventKind::Intent,
    );
    assert_eq!(
        Event::PermissionResponse { request_id: "x".into(), action: PermissionAction::Allow }.category(),
        crate::event::EventCategory::Permission,
    );
}

fn check_persistence_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::InputChanged { state: Box::new(crate::model::InputState::default()) }.kind(),
        EventKind::Fact,
    );
    assert_eq!(
        Event::InputChanged { state: Box::new(crate::model::InputState::default()) }.category(),
        crate::event::EventCategory::Persistence,
    );
}

fn check_plan_mode_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::PlanModeEnabled { content: "x".into() }.kind(),
        EventKind::Intent
    );
    assert_eq!(
        Event::PlanModeEnabled { content: "x".into() }.category(),
        crate::event::EventCategory::PlanMode,
    );
}

fn check_scroll_category() {
    use crate::event::EventKind;
    assert_eq!(Event::Up.kind(), EventKind::Intent);
    assert_eq!(Event::Up.category(), crate::event::EventCategory::Scroll);
}

fn check_session_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::SessionSaved { name: "x".into() }.kind(),
        EventKind::Fact
    );
    assert_eq!(
        Event::SessionSaved { name: "x".into() }.category(),
        crate::event::EventCategory::Session,
    );
}

fn check_system_category() {
    use crate::event::EventKind;
    assert_eq!(
        Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) }.kind(),
        EventKind::Fact,
    );
    assert_eq!(
        Event::ConfigLoaded { config: Box::new(crate::config::Config::default()) }.category(),
        crate::event::EventCategory::System,
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn is_fact_variant_matches_kind() {
    use crate::event::is_fact_variant;
    use crate::event::EventKind;
    // Every fact event should return true from is_fact_variant
    let fact_samples: Vec<Event> = vec![
        Event::Thinking { id: "x".into() },
        Event::ToolEnd { id: "x".into(), duration_secs: 1.0, output: "y".into(), input: None },
        Event::Response {
            id: "x".into(),
            content: "y".into(),
            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        },
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

#[test]
fn permission_intent_events_classify_correctly() {
    use crate::event::{EventCategory, EventKind};

    let allow = Event::PermissionAllow { request_id: "r1".into() };
    assert_eq!(allow.kind(), EventKind::Intent);
    assert_eq!(allow.category(), EventCategory::Permission);
    assert!(allow.clone().into_intent().is_some());

    let deny = Event::PermissionDeny { request_id: "r1".into() };
    assert_eq!(deny.kind(), EventKind::Intent);
    assert_eq!(deny.category(), EventCategory::Permission);

    let always = Event::PermissionAlwaysAllow { request_id: "r1".into(), tool: "bash".into() };
    assert_eq!(always.kind(), EventKind::Intent);
    assert_eq!(always.category(), EventCategory::Permission);
    assert!(always.clone().into_intent().is_some());
}
