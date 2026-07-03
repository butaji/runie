//! Tests for durable event types.

use super::*;

// ── TurnJournalPhase tests ────────────────────────────────────────────────

#[test]
fn turn_phase_serialization() {
    // Test TurnPhase roundtrips through JSON
    let phases = vec![
        (TurnPhase::TurnStarted, "turnStarted"),
        (TurnPhase::ProviderCalled, "providerCalled"),
        (TurnPhase::ToolRequestsRecorded, "toolRequestsRecorded"),
        (TurnPhase::ResponseDelta, "responseDelta"),
        (TurnPhase::TurnCommitted, "turnCommitted"),
        (TurnPhase::TurnAborted, "turnAborted"),
    ];
    for (phase, expected) in phases {
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
        let parsed: TurnPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, phase);
    }
}

#[test]
fn durable_turn_phase_changed_roundtrips_through_json() {
    let durable = DurableCoreEvent::TurnPhaseChanged {
        phase: TurnPhase::TurnStarted,
        request_id: "req.1".into(),
    };
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
    match parsed {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::TurnStarted);
            assert_eq!(request_id, "req.1");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn durable_turn_phase_changed_all_phases_roundtrip() {
    let phases = vec![
        TurnPhase::TurnStarted,
        TurnPhase::ProviderCalled,
        TurnPhase::ToolRequestsRecorded,
        TurnPhase::ResponseDelta,
        TurnPhase::TurnCommitted,
        TurnPhase::TurnAborted,
    ];
    for phase in phases {
        let durable = DurableCoreEvent::TurnPhaseChanged {
            phase,
            request_id: "req.test".into(),
        };
        let json = serde_json::to_string(&durable).unwrap();
        let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
        if let DurableCoreEvent::TurnPhaseChanged { phase: p, .. } = parsed {
            assert_eq!(p, phase);
        } else {
            panic!("Expected TurnPhaseChanged");
        }
    }
}

#[test]
fn event_turn_started_converts_to_phase_changed() {
    let event = Event::TurnStarted {
        id: "t1".into(),
        request_id: "req.1".into(),
        content: "hello".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    match durable.unwrap() {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::TurnStarted);
            assert_eq!(request_id, "req.1");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn event_turn_complete_converts_to_phase_changed() {
    let event = Event::TurnComplete {
        id: "t1".into(),
        duration_secs: 1.0,
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    match durable.unwrap() {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::TurnCommitted);
            assert_eq!(request_id, ""); // Empty for committed phases
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn event_turn_aborted_converts_to_phase_changed() {
    let event = Event::TurnAborted;
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    match durable.unwrap() {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::TurnAborted);
            assert_eq!(request_id, "");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn event_tool_requests_recorded_converts_to_phase_changed() {
    let event = Event::ToolRequestsRecorded {
        request_id: "req.1".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    match durable.unwrap() {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::ToolRequestsRecorded);
            assert_eq!(request_id, "req.1");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn event_response_delta_started_converts_to_phase_changed() {
    let event = Event::ResponseDeltaStarted {
        request_id: "req.1".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&event);
    assert!(durable.is_some());
    match durable.unwrap() {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::ResponseDelta);
            assert_eq!(request_id, "req.1");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn durable_turn_phase_changed_reverse_conversion_returns_err() {
    // TurnPhaseChanged doesn't convert back to Event (handled separately for crash recovery)
    let durable = DurableCoreEvent::TurnPhaseChanged {
        phase: TurnPhase::TurnStarted,
        request_id: "req.1".into(),
    };
    let event: Result<Event, _> = Event::try_from(&durable);
    assert!(event.is_err());
}

#[test]
fn turn_phase_changed_request_id_default_empty() {
    // Test backward compatibility: missing request_id defaults to empty string
    let json = r#"{"event":"turnPhaseChanged","phase":"turnStarted"}"#;
    let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
    match durable {
        DurableCoreEvent::TurnPhaseChanged { request_id, .. } => {
            assert_eq!(request_id, "");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

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
    // duration_secs is NOT stored in the durable form.
    assert!(matches!(
        durable,
        DurableCoreEvent::ToolResult { id, output, success: true }
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
            provider,
            ..
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
        Event::TokenStatsUpdated { tokens_in: 0, tokens_out: 0 },
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
        parts: Vec::new(),
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
    // Event::ToolEnd → DurableCoreEvent::ToolResult → Event::ToolEnd.
    // duration_secs is NOT preserved through the durable layer (timing data lost in storage).
    let original = Event::ToolEnd {
        id: "t1".into(),
        input: None,
        duration_secs: 3.5,
        output: "result".into(),
    };
    let durable = DurableCoreEvent::try_from_event(&original).unwrap();
    let recovered: Event = Event::try_from(&durable).unwrap();
    // Recovered duration_secs is 0.0 (not available during replay).
    assert!(matches!(
        recovered,
        Event::ToolEnd { id, output, duration_secs: 0.0, .. }
        if id == "t1" && output == "result"
    ));
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
        parts: Vec::new(),
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
fn durable_tool_result_roundtrips_through_json() {
    let durable = DurableCoreEvent::ToolResult {
        id: "t1".into(),
        output: "done".into(),
        success: true,
    };
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        parsed,
        DurableCoreEvent::ToolResult { id, output, success: true }
        if id == "t1" && output == "done"
    ));
}

#[test]
fn durable_message_sent_preserves_parts() {
    use crate::proto::message::Part;

    let parts = vec![
        Part::Text { content: "Hello".into() },
        Part::Reasoning { content: "thinking".into() },
        Part::tool_call("call_1", "bash", serde_json::json!({"cmd": "ls"})),
    ];
    let durable = DurableCoreEvent::MessageSent {
        id: "m1".into(),
        role: "assistant".into(),
        content: "Hello".into(),
        timestamp: 100.0,
        provider: "openai".into(),
        parts: parts.clone(),
    };

    // Serialize and deserialize
    let json = serde_json::to_string(&durable).unwrap();
    let parsed: DurableCoreEvent = serde_json::from_str(&json).unwrap();

    match parsed {
        DurableCoreEvent::MessageSent { parts: parsed_parts, .. } => {
            assert_eq!(parsed_parts.len(), 3);
            assert!(matches!(&parsed_parts[0], Part::Text { content } if content == "Hello"));
            assert!(matches!(&parsed_parts[1], Part::Reasoning { content } if content == "thinking"));
            assert!(matches!(&parsed_parts[2], Part::ToolCall { name, .. } if name == "bash"));
        }
        _ => panic!("Expected MessageSent"),
    }
}

#[test]
fn durable_turn_phase_changed_in_jsonl_format() {
    // Simulate a JSONL line with turn phase
    let json = r#"{"event":"turnPhaseChanged","phase":"turnStarted","requestId":"req.1"}"#;
    let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
    match durable {
        DurableCoreEvent::TurnPhaseChanged { phase, request_id } => {
            assert_eq!(phase, TurnPhase::TurnStarted);
            assert_eq!(request_id, "req.1");
        }
        _ => panic!("Expected TurnPhaseChanged"),
    }
}

#[test]
fn durable_message_sent_backward_compatible() {
    // Old JSON without parts should default to empty Vec
    let json = r#"{"event":"messageSent","id":"m1","role":"assistant","content":"Hello","timestamp":100.0,"provider":"openai"}"#;
    let durable: DurableCoreEvent = serde_json::from_str(json).unwrap();
    match durable {
        DurableCoreEvent::MessageSent { parts, .. } => {
            assert!(parts.is_empty());
        }
        _ => panic!("Expected MessageSent"),
    }

    // Content should still be preserved for backward compatibility
    let durable2: DurableCoreEvent = serde_json::from_str(json).unwrap();
    let event: Result<Event, _> = Event::try_from(&durable2);
    assert!(event.is_ok());
    match event.unwrap() {
        Event::MessageReplayed { content, .. } => {
            assert_eq!(content, "Hello");
        }
        _ => panic!("Expected MessageReplayed"),
    }
}

#[test]
fn durable_message_sent_reconstructs_content_from_parts() {
    use crate::proto::message::Part;

    // When parts are present, content should be reconstructed from text parts
    let durable = DurableCoreEvent::MessageSent {
        id: "m1".into(),
        role: "assistant".into(),
        content: "".into(), // Old field might be empty
        timestamp: 100.0,
        provider: "openai".into(),
        parts: vec![
            Part::Text { content: "Part 1".into() },
            Part::Text { content: " Part 2".into() },
            Part::Reasoning { content: "hidden".into() },
        ],
    };

    let event: Event = Event::try_from(&durable).unwrap();
    match event {
        Event::MessageReplayed { content, .. } => {
            // Content should be reconstructed from text parts only (not reasoning)
            assert_eq!(content, "Part 1 Part 2");
        }
        _ => panic!("Expected MessageReplayed"),
    }
}
