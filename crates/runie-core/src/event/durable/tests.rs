//! Tests for durable event types.

use super::*;

// ── Event → DurableCoreEvent ─────────────────────────────────────────────

#[test]
fn durable_from_response() {
    let event = Event::Response {
        id: "r1".into(),
        content: "hello".into(),
        role: "assistant".into(),
        timestamp: 1000.0,
        provider: "openai".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    let durable = durable.unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::MessageSent { role, content, .. }
        if role == "assistant" && content == "hello"
    ));
}

#[test]
fn durable_from_tool_start() {
    let event = Event::ToolStart {
        id: "t1".into(),
        name: "bash".into(),
        input: serde_json::json!({"cmd": "ls"}),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    let durable = durable.unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::ToolCalled { id, name, .. } if id == "t1" && name == "bash"
    ));
}

#[test]
fn durable_from_tool_end() {
    let event = Event::ToolEnd {
        id: "t1".into(),
        input: None,
        duration_secs: 1.5,
        output: "done".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    let durable = durable.unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::ToolResult { id, output, success: true, duration_secs: 1.5 }
        if id == "t1" && output == "done"
    ));
}

#[test]
fn durable_from_switch_model() {
    let event = Event::SwitchModel {
        provider: "anthropic".into(),
        model: "claude-3".into(),
        explicit: true,
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    assert!(matches!(
        durable.unwrap(),
        DurableCoreEvent::ModelSwitched { provider, model }
        if provider == "anthropic" && model == "claude-3"
    ));
}

#[test]
fn durable_from_run_name_command() {
    let event = Event::RunNameCommand { name: "my session".into() };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    assert!(matches!(
        durable.unwrap(),
        DurableCoreEvent::SessionRenamed { name } if name == "my session"
    ));
}

#[test]
fn durable_from_switch_theme() {
    let event = Event::SwitchTheme { name: "dark".into() };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    assert!(matches!(
        durable.unwrap(),
        DurableCoreEvent::ThemeSwitched { name } if name == "dark"
    ));
}

#[test]
fn durable_from_set_thinking_level() {
    let event = Event::SetThinkingLevel(crate::model::ThinkingLevel::Medium);
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    assert!(matches!(
        durable.unwrap(),
        DurableCoreEvent::ThinkingLevelSet { level: crate::model::ThinkingLevel::Medium }
    ));
}

#[test]
fn durable_from_message_replayed() {
    let event = Event::MessageReplayed {
        id: "r1".into(),
        role: "user".into(),
        content: "hello".into(),
        timestamp: 1234.5,
        provider: "anthropic".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    assert!(matches!(
        durable.unwrap(),
        DurableCoreEvent::MessageSent {
            id,
            role,
            content,
            timestamp,
            provider
        }
        if id == "r1" && role == "user" && content == "hello"
            && timestamp == 1234.5 && provider == "anthropic"
    ));
}

#[test]
fn transient_events_return_none() {
    let cases: Vec<Event> = vec![
        Event::ResponseDelta { id: "".into(), content: "x".into() },
        Event::TextStart { id: "".into() },
        Event::TextEnd { id: "".into() },
        Event::ThinkingDelta { id: "".into(), content: "".into() },
        Event::ThinkingStart { id: "".into() },
        Event::ThinkingEnd { id: "".into() },
        Event::Thinking { id: "".into() },
        Event::ThoughtDone { id: "".into() },
        Event::TokenStatsUpdated { tokens_in: 0, tokens_out: 0, speed_tps: 0.0 },
        Event::Quit,
        Event::Input('x'),
    ];
    for event in cases {
        assert!(
            DurableCoreEvent::try_from_event(&event).is_none(),
            "{:?} should not become durable",
            event
        );
    }
}

// ── DurableCoreEvent → Event (reverse) ───────────────────────────────────

#[test]
fn event_from_message_sent() {
    let durable = DurableCoreEvent::MessageSent {
        id: "r1".into(),
        role: "assistant".into(),
        content: "hello".into(),
        timestamp: 100.0,
        provider: "openai".into(),
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_ok());
    let event = event.unwrap();
    assert!(matches!(
        event,
        Event::MessageReplayed { id, role, content, timestamp, provider }
        if id == "r1" && role == "assistant" && content == "hello"
            && timestamp == 100.0 && provider == "openai"
    ));
}

#[test]
fn event_from_tool_called() {
    let durable = DurableCoreEvent::ToolCalled {
        id: "t1".into(),
        name: "read".into(),
        input: serde_json::json!({"path": "/tmp"}),
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_ok());
    assert!(matches!(
        event.unwrap(),
        Event::ToolStart { id, name, .. } if id == "t1" && name == "read"
    ));
}

#[test]
fn event_from_tool_result() {
    let durable = DurableCoreEvent::ToolResult {
        id: "t1".into(),
        output: "result".into(),
        success: true,
        duration_secs: 0.0,
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_ok());
    assert!(matches!(
        event.unwrap(),
        Event::ToolEnd { id, output, .. } if id == "t1" && output == "result"
    ));
}

#[test]
fn event_from_model_switched() {
    let durable = DurableCoreEvent::ModelSwitched {
        provider: "openai".into(),
        model: "gpt-4".into(),
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_ok());
    assert!(matches!(
        event.unwrap(),
        Event::SwitchModel { provider, model, explicit: false }
        if provider == "openai" && model == "gpt-4"
    ));
}

#[test]
fn event_from_session_renamed_is_err() {
    let durable = DurableCoreEvent::SessionRenamed { name: "x".into() };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_err()); // handled directly in replay_event
}

#[test]
fn event_from_read_only_set_is_err() {
    let durable = DurableCoreEvent::ReadOnlySet { read_only: true };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_err()); // handled directly in replay_event
}

// ── SessionTreeSnapshot round-trip ─────────────────────────────────────────

#[test]
fn durable_from_session_tree_snapshot() {
    let snapshot = crate::session::tree::SessionTreeSnapshot {
        current_branch: vec!["msg1".into(), "msg2".into()],
        root_id: "msg1".into(),
        nodes: vec![],
        edges: vec![],
    };
    let event = Event::SessionTreeSnapshot { snapshot: snapshot.clone() };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    let durable = durable.unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::TreeSnapshot { snapshot } if snapshot.current_branch == ["msg1", "msg2"]
    ));
}

#[test]
fn event_from_tree_snapshot() {
    let durable = DurableCoreEvent::TreeSnapshot {
        snapshot: crate::session::tree::SessionTreeSnapshot {
            current_branch: vec!["a".into()],
            root_id: "a".into(),
            nodes: vec![],
            edges: vec![],
        },
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_ok());
    let event = event.unwrap();
    assert!(matches!(
        event,
        Event::SessionTreeSnapshot { snapshot } if snapshot.current_branch == ["a"]
    ));
}

#[test]
fn tree_snapshot_roundtrip() {
    // Event → DurableCoreEvent → Event (full round-trip)
    let snapshot = crate::session::tree::SessionTreeSnapshot {
        current_branch: vec!["root".into(), "branch".into()],
        root_id: "root".into(),
        nodes: vec![],
        edges: vec![],
    };
    let original = Event::SessionTreeSnapshot { snapshot: snapshot.clone() };
    let durable = DurableCoreEvent::try_from_event(&original).unwrap();
    let recovered: Event = Event::try_from(&durable).unwrap();
    match (original, recovered) {
        (
            Event::SessionTreeSnapshot { snapshot: s1 },
            Event::SessionTreeSnapshot { snapshot: s2 },
        ) => assert_eq!(s1, s2),
        _ => panic!("round-trip mismatch"),
    }
}

#[test]
fn tool_result_roundtrip() {
    // Event::ToolEnd → DurableCoreEvent::ToolResult → Event::ToolEnd (preserves duration_secs)
    let original = Event::ToolEnd {
        id: "t1".into(),
        input: None,
        duration_secs: 3.5,
        output: "result".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&original).unwrap();
    let recovered: Event = Event::try_from(&durable).unwrap();
    match (original, recovered) {
        (
            Event::ToolEnd { duration_secs: d1, .. },
            Event::ToolEnd { duration_secs: d2, .. },
        ) => assert_eq!(d1, d2),
        _ => panic!("round-trip mismatch"),
    }
}

// ── Serde roundtrip for DurableCoreEvent ─────────────────────────────────

#[test]
fn durable_message_sent_roundtrips_through_json() {
    let durable = DurableCoreEvent::MessageSent {
        id: "r1".into(),
        role: "user".into(),
        content: "test".into(),
        timestamp: 999.0,
        provider: "".into(),
    };
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, durable);
}

#[test]
fn durable_tool_called_roundtrips_through_json() {
    let durable = DurableCoreEvent::ToolCalled {
        id: "t1".into(),
        name: "bash".into(),
        input: serde_json::json!({"cmd": "ls"}),
    };
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, durable);
}

#[test]
fn durable_preserves_existing_jsonl_format() {
    // Simulate a JSONL line from an existing session file
    let json = r#"{"event":"modelSwitched","provider":"openai","model":"gpt-4"}"#;
    let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::ModelSwitched { provider, model }
        if provider == "openai" && model == "gpt-4"
    ));
}

#[test]
fn durable_tool_result_preserves_duration() {
    let durable = DurableCoreEvent::ToolResult {
        id: "t1".into(),
        output: "done".into(),
        success: true,
        duration_secs: 2.5,
    };
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
    match parsed {
        DurableCoreEvent::ToolResult { duration_secs, .. } => {
            assert_eq!(duration_secs, 2.5);
        }
        _ => panic!("Expected ToolResult"),
    }
}

#[test]
fn durable_tool_result_backward_compatible() {
    // Old JSON without duration_secs should default to 0.0
    let json = r#"{"event":"toolResult","id":"t1","output":"done","success":true}"#;
    let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(
        durable,
        DurableCoreEvent::ToolResult { duration_secs, .. } if duration_secs == 0.0
    ));
}
