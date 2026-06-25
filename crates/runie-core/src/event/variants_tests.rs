//! Tests for Event variants.

use crate::event::{intent::Intent, EventKind};
use crate::Event;

// Re-export submodules for organization
mod durable;
mod dispatch;

use crate::event::DurableCoreEvent;
use crate::event::EVENT_NAMES;

/// Pre-optimization size of `Event` before boxing large orchestrator payloads.
const EVENT_BASELINE_SIZE: usize = 288;

#[test]
fn event_size_reduced() {
    let size = std::mem::size_of::<Event>();
    assert!(
        size < EVENT_BASELINE_SIZE,
        "Event size {} should be smaller than baseline {}",
        size,
        EVENT_BASELINE_SIZE
    );
}

#[test]
fn event_input_equality() {
    assert_eq!(Event::Input('a'), Event::Input('a'));
    assert_ne!(Event::Input('a'), Event::Input('b'));
}

#[test]
fn event_agent_equality() {
    let id = "test.1".to_string();
    assert_eq!(
        Event::Thinking { id: id.clone() },
        Event::Thinking {
            id: "test.1".to_string()
        },
    );
}

#[test]
fn event_scroll_equality() {
    assert_eq!(Event::Up, Event::Up);
    assert_ne!(Event::Up, Event::Down);
}

#[test]
fn all_sub_enums_have_variants() {
    let _ = Event::Submit;
    let _ = Event::Done { id: "x".into() };
    let _ = Event::Up;
    let _ = Event::Quit;
    let _ = Event::ForceQuit;
    let _ = Event::SwitchModel {
        provider: "openai".into(),
        model: "gpt-4".into(),
        explicit: false,
    };
    let _ = Event::ToggleCommandPalette;
    let _ = Event::PendingEdit {
        path: "x".into(),
        original: "a".into(),
        proposed: "b".into(),
    };
    let _ = Event::ClearTransient;
    let _ = Event::CloneSession;
    let _ = Event::RunNameCommand {
        name: "test".into(),
    };
}

#[test]
fn convenience_constructors() {
    assert!(matches!(Event::input('x'), Event::Input('x')));
    assert!(matches!(Event::submit(), Event::Submit));
    assert!(matches!(Event::scroll_up(), Event::Up));
    assert!(matches!(Event::quit(), Event::Quit));
    assert!(matches!(Event::force_quit(), Event::ForceQuit));
    assert!(matches!(
        Event::switch_model("anthropic".into(), "claude-3".into()),
        Event::SwitchModel { .. }
    ));
    assert!(matches!(
        Event::switch_theme("dracula".into()),
        Event::SwitchTheme { .. }
    ));
    assert!(matches!(
        Event::agent_thinking("x".into()),
        Event::Thinking { .. }
    ));
    assert!(matches!(
        Event::agent_tool_start("t1".into(), "bash".into(), serde_json::Value::Null),
        Event::ToolStart { .. }
    ));
    assert!(matches!(
        Event::agent_response("r1".into(), "hi".into()),
        Event::Response { .. }
    ));
}

/// Layer 1: every event that claims a name round-trips correctly.
#[test]
fn event_name_round_trip() {
    for (name, ctor) in EVENT_NAMES {
        let evt = ctor();
        if let Some(got) = evt.name() {
            assert_eq!(got, *name, "{}: Event::name() returned wrong name", name);
        }
        let roundtrip = Event::from_name(name);
        assert!(
            roundtrip.is_some(),
            "{}: Event::from_name({:?}) returned None",
            name,
            name
        );
    }
}

/// Layer 2: verify that Intent events have a path to typed Intent via into_intent().
/// This test ensures the taxonomy is used for typed intent conversion.
#[test]
fn intent_events_have_typed_intent_conversion() {
    // Verify key intent events convert to typed Intent
    let test_cases: Vec<(Event, fn(Intent) -> bool)> = vec![
        (Event::Input('x'), |i| matches!(i, Intent::Input('x'))),
        (Event::Submit, |i| matches!(i, Intent::Submit)),
        (
            Event::SwitchModel {
                provider: "openai".into(),
                model: "gpt-4".into(),
                explicit: true,
            },
            |i| matches!(i, Intent::SwitchModel { provider, model, explicit } if provider == "openai" && model == "gpt-4" && explicit),
        ),
        (
            Event::RunSaveCommand {
                name: "test".into(),
            },
            |i| matches!(i, Intent::RunSaveCommand { name } if name == "test"),
        ),
        (Event::ToggleCommandPalette, |i| {
            matches!(i, Intent::ToggleCommandPalette)
        }),
        (Event::ForkSession { message_index: 5 }, |i| {
            matches!(i, Intent::ForkSession { message_index: 5 })
        }),
    ];

    for (event, check) in test_cases {
        // Verify the event is classified as Intent
        assert_eq!(
            event.kind(),
            EventKind::Intent,
            "{:?} should be classified as Intent",
            event
        );
        // Verify it converts to typed Intent
        let intent = event.clone().into_intent();
        assert!(
            intent.is_some(),
            "{:?} should convert to Some(Intent)",
            event
        );
        let intent = intent.unwrap();
        assert!(
            check(intent),
            "{:?} converted to wrong Intent variant",
            event
        );
    }
}

/// Layer 2: verify that Fact events do NOT convert to typed Intent.
/// Facts come from actors and are projected into AppState, not converted to Intent.
#[test]
fn fact_events_do_not_convert_to_intent() {
    let fact_events = vec![
        Event::Thinking { id: "1".into() },
        Event::ToolStart {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({}),
        },
        Event::ToolEnd {
            id: "t1".into(),
            duration_secs: 1.0,
            output: "ok".into(),
        },
        Event::Response {
            id: "r1".into(),
            content: "hello".into(),
        },
        Event::TurnComplete {
            id: "1".into(),
            duration_secs: 1.0,
        },
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
    ];

    for event in fact_events {
        assert_eq!(
            event.kind(),
            EventKind::Fact,
            "{:?} should be classified as Fact",
            event
        );
        assert!(
            event.clone().into_intent().is_none(),
            "{:?} should NOT convert to Intent",
            event
        );
    }
}
