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
