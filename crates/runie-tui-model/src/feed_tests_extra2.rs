#[test]
fn bus_messages_for_event_emits_set_theme_for_theme_changed() {
    // Pin the theme projection: a `ThemeChanged` event emits a
    // `ScrollbackMsg::SetTheme` carrying the new theme.
    use runie_core::types::{AgentEvent, ThemeKind};
    let messages = super::bus_messages_for_event(&AgentEvent::ThemeChanged {
        theme: ThemeKind::GrokDay,
    });
    assert_eq!(
        messages,
        vec![super::ScrollbackMsg::SetTheme(ThemeKind::GrokDay)]
    );
}

#[test]
fn declarative_fact_reset_tables_clear_only_their_owned_fields() {
    let mut facts = super::FeedFacts {
        activity_dirs: 1,
        activity_files: 2,
        workflow_headers: std::collections::HashMap::from([("run".into(), "Build".into())]),
        workflow_phases: std::collections::HashMap::from([(
            "run".into(),
            vec![("compile".into(), "done".into())],
        )]),
        turn_started: true,
        ..Default::default()
    };

    facts.reset_activity();
    assert_eq!(facts.activity_dirs, 0);
    assert_eq!(facts.activity_files, 0);
    assert!(facts.turn_started);
    assert!(!facts.workflow_headers.is_empty());

    facts.reset_workflows();
    assert!(facts.workflow_headers.is_empty());
    assert!(facts.workflow_phases.is_empty());
}

#[test]
fn bus_messages_for_event_returns_empty_for_non_actor_feed() {
    // Pin the negative path: a non-actor-feed event returns an
    // empty vector so the renderer can rely on the bus projection
    // for delivery-or-skip semantics.
    use runie_core::types::AgentEvent;
    let event = AgentEvent::TurnStart;
    assert!(super::bus_messages_for_event(&event).is_empty());
}

#[test]
fn dense_tool_group_members_projects_member_positions() {
    // Pin the smoke path: a single contiguous group of three
    // members emits `(member_index, size)` triples for each
    // slot, with `None` for the separator slots.
    let tool_ids: [Option<&str>; 5] = [Some("a"), Some("b"), Some("c"), None, Some("d")];
    let positions = super::dense_tool_group_members(&tool_ids);
    assert_eq!(
        positions,
        vec![Some((0, 3)), Some((1, 3)), Some((2, 3)), None, Some((0, 1)),]
    );
}

#[test]
fn dense_tool_group_members_returns_empty_for_empty_input() {
    // Pin the negative path: an empty input returns an empty
    // projection so the renderer never has to handle a `None`
    // index into a missing slice.
    let positions = super::dense_tool_group_members(&[]);
    assert!(positions.is_empty());
}

#[test]
fn dense_tool_group_members_with_identity_separates_duplicate_call_ids() {
    let members = vec![
        Some((String::from("duplicate"), Some(1))),
        Some((String::from("duplicate"), Some(2))),
        Some((String::from("other"), Some(3))),
    ];
    assert_eq!(
        super::dense_tool_group_members_with_identity(&members),
        vec![Some((0, 3)), Some((1, 3)), Some((2, 3))]
    );
}
#[test]
fn scrollback_domain_table_classifies_all_declared_domains() {
    assert_eq!(
        super::ScrollbackMsg::TurnStart.domain(),
        super::ScrollbackDomain::Lifecycle
    );
    assert_eq!(
        super::ScrollbackMsg::Append(super::Line::new(super::LineKind::User, "")).domain(),
        super::ScrollbackDomain::Content
    );
    assert_eq!(
        super::ScrollbackMsg::SetToolName("id".into(), "tool".into()).domain(),
        super::ScrollbackDomain::Tool
    );
    assert_eq!(
        super::ScrollbackMsg::WorkflowEnd {
            run_id: "run".into(),
            status: "ok".into(),
            elapsed_ms: None,
        }
        .domain(),
        super::ScrollbackDomain::Workflow
    );
    assert_eq!(
        super::ScrollbackMsg::ClearSelection.domain(),
        super::ScrollbackDomain::Navigation
    );
}

#[test]
fn grouped_scrollback_events_bridge_to_legacy_messages() {
    let event = super::ScrollbackEvent::Tool(super::ScrollbackToolEvent::Updated {
        tool_call_id: "call-1".into(),
        header: Some("bash".into()),
        output: vec!["ok".into()],
    });
    assert_eq!(
        event.into_messages(),
        vec![super::ScrollbackMsg::ToolUpdate {
            tool_call_id: "call-1".into(),
            header: Some("bash".into()),
            output: vec!["ok".into()],
        }]
    );
}
#[test]
fn navigation_yaml_trace_replays_through_the_view_state_projection() {
    let state = runie_core::replay_yaml_state(
        include_str!("../fixtures/scrollback-navigation.yaml"),
        super::FeedState::default(),
        |state, event: &super::ScrollbackNavigationEvent| {
            for message in super::ScrollbackEvent::Navigation(event.clone()).into_messages() {
                state.reduce(message);
            }
        },
    )
    .expect("valid navigation fixture");

    let snapshot = state.snapshot();
    assert_eq!(snapshot.scroll_offset, 0);
    assert!(snapshot.autoscroll);
    assert!(snapshot.selection_anchor.is_none());
}

#[test]
fn content_yaml_trace_replays_through_transcript_and_assistant_projection() {
    let state = runie_core::replay_yaml_state(
        include_str!("../fixtures/scrollback-content.yaml"),
        super::FeedState::default(),
        |state, event: &super::ScrollbackContentEvent| {
            for message in super::ScrollbackEvent::Content(event.clone()).into_messages() {
                state.reduce(message);
            }
        },
    )
    .expect("valid content fixture");

    let snapshot = state.snapshot();
    assert_eq!(snapshot.lines.len(), 2);
    assert_eq!(snapshot.lines[0].kind, super::LineKind::User);
    assert_eq!(snapshot.lines[1].kind, super::LineKind::Assistant);
    assert!(snapshot.facts.settled_no_tool_phase);
}
#[test]
fn tool_card_summary_keeps_terminal_error_and_running_state() {
    let lines = vec![
        super::Line::new(super::LineKind::Tool, "Bash").for_tool("bash-1"),
        super::Line::new(super::LineKind::ToolOutput, "still running").for_tool("bash-1"),
        super::Line::new(super::LineKind::ToolError, "failed").for_tool("bash-1"),
    ];
    let names = std::collections::HashMap::from([("bash-1".to_owned(), "bash".to_owned())]);
    let summaries = super::tool_card_summaries(&lines, &names);
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_error);
    assert!(!summaries[0].is_running);
    assert_eq!(summaries[0].card_kind, super::ToolCardKind::Execute);
}

#[test]
fn tool_card_summaries_index_duplicate_call_ids_by_member() {
    let lines = vec![
        super::Line::new(super::LineKind::Tool, "first")
            .for_tool_row(1)
            .for_tool("same"),
        super::Line::new(super::LineKind::ToolOutput, "one")
            .for_tool_row(1)
            .for_tool("same"),
        super::Line::new(super::LineKind::Tool, "second")
            .for_tool_row(2)
            .for_tool("same"),
        super::Line::new(super::LineKind::ToolOutput, "two")
            .for_tool_row(2)
            .for_tool("same"),
    ];
    let names = std::collections::HashMap::from([("same".to_owned(), "read".to_owned())]);
    let summaries = super::tool_card_summaries(&lines, &names);
    assert_eq!(
        summaries
            .iter()
            .map(|row| row.member_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(
        summaries
            .iter()
            .map(|row| row.output_bytes)
            .collect::<Vec<_>>(),
        [3, 3]
    );
}
