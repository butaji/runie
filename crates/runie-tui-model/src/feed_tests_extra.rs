#[test]
fn session_start_messages_emits_three_bracket_rows() {
    // Pin the smoke path: the projection emits exactly three messages
    // so the actor-owned session-start projection and the renderer
    // agree on the wrapping shape.
    let messages = super::session_start_messages();
    assert_eq!(messages.len(), 3);
    assert!(matches!(&messages[0], super::ScrollbackMsg::Append(_)));
    assert!(matches!(&messages[1], super::ScrollbackMsg::Append(_)));
    assert!(matches!(&messages[2], super::ScrollbackMsg::Append(_)));
}

#[test]
fn session_start_messages_pins_separator_and_hooks_content() {
    // Pin the wrapping shape: the outer rows are blank `Separator`
    // lines and the middle row is the `SessionStart` marker with the
    // `[hooks: 1]` count.
    let messages = super::session_start_messages();
    let first = match &messages[0] {
        super::ScrollbackMsg::Append(line) => line,
        other => panic!("expected separator append, got {other:?}"),
    };
    assert_eq!(first.kind, super::LineKind::Separator);
    assert!(first.text.is_empty());
    let middle = match &messages[1] {
        super::ScrollbackMsg::Append(line) => line,
        other => panic!("expected session start append, got {other:?}"),
    };
    assert_eq!(middle.kind, super::LineKind::SessionStart);
    assert_eq!(middle.text, "◆ session_start  [hooks: 1]");
    let last = match &messages[2] {
        super::ScrollbackMsg::Append(line) => line,
        other => panic!("expected separator append, got {other:?}"),
    };
    assert_eq!(last.kind, super::LineKind::Separator);
    assert!(last.text.is_empty());
}

#[test]
fn append_user_with_timestamp_right_aligns_timestamp_into_first_row() {
    // Pin the gutter path: the timestamp appears at the right edge of
    // the first row, with the prompt text filling the leading
    // columns and the trailing ` TIMESTAMP_EDGE_OFFSET` slack.
    let mut rows = Vec::new();
    super::append_user_with_timestamp(&mut rows, "hello world".into(), "3:07 PM", 40);
    let first = &rows[0];
    assert_eq!(first.0, super::LineKind::User);
    assert!(first.1.starts_with("hello world"), "{}", first.1);
    assert!(first.1.ends_with("3:07 PM"), "{}", first.1);
    assert!(!first.2);
}

#[test]
fn append_user_with_timestamp_wraps_remaining_text_with_indent() {
    // Pin the wrap path: a long prompt that exceeds the gutter width
    // emits a continuation row indent matching the `LineKind::User`
    // prefix so the projected widget keeps its indentation.
    let mut rows = Vec::new();
    super::append_user_with_timestamp(
        &mut rows,
        "the quick brown fox jumps over the lazy dog".into(),
        "3:07 PM",
        10,
    );
    assert!(rows.len() >= 2);
    // Pin the smoke path: the first row holds the timestamp and the
    // remaining rows wrap the rest of the prompt text.
    assert!(rows[0].1.contains("3:07 PM"));
    for row in &rows[1..] {
        assert_eq!(row.0, super::LineKind::User);
    }
}

#[test]
fn make_relative_path_strips_workspace_and_collapses_to_dot() {
    // Pin the smoke path: the workspace-only path collapses to `.`
    // so the rendered header is a clean directory anchor.
    assert_eq!(super::make_relative_path("/work", "/work"), ".");
    // Pin the workspace-relative path: a leading separator is
    // stripped so the rendered header never shows `<workspace>/`.
    assert_eq!(super::make_relative_path("/work", "/work/file"), "file");
    // Pin the nested path: a deeper workspace-relative path keeps
    // its directory structure intact.
    assert_eq!(
        super::make_relative_path("/work", "/work/dir/sub/file"),
        "dir/sub/file"
    );
    // Pin the negative path: a path outside the workspace is
    // returned verbatim so the renderer can decide how to label it.
    assert_eq!(
        super::make_relative_path("/work", "/tmp/other/file"),
        "/tmp/other/file"
    );
}

#[test]
fn grok_effective_compact_pins_user_and_terminal_signal() {
    // Pin the user signal: an explicit user compact override always
    // wins, regardless of measured terminal height.
    assert!(super::grok_effective_compact(true, 0));
    assert!(super::grok_effective_compact(true, 80));
    // Pin the terminal signal: an unmeasured height (zero rows)
    // does not force compact mode so the renderer can wait for a
    // real measurement.
    assert!(!super::grok_effective_compact(false, 0));
    // Pin the auto-compact band: heights at or below
    // `GROK_AUTO_COMPACT_MAX_ROWS` force compact mode.
    assert!(super::grok_effective_compact(
        false,
        super::GROK_AUTO_COMPACT_MAX_ROWS
    ));
    // Pin the full-mode range: heights above the auto-compact band
    // do not force compact mode.
    assert!(!super::grok_effective_compact(
        false,
        super::GROK_AUTO_COMPACT_MAX_ROWS + 1
    ));
}

#[test]
fn grok_small_screen_tip_visible_targets_the_pre_compact_band() {
    // Pin the boundary: the tip is hidden at and below the
    // auto-compact threshold.
    assert!(!super::grok_small_screen_tip_visible(
        super::GROK_AUTO_COMPACT_MAX_ROWS
    ));
    // Pin the smoke path: heights strictly above the auto-compact
    // threshold and at or below the tip max are visible.
    assert!(super::grok_small_screen_tip_visible(
        super::GROK_AUTO_COMPACT_MAX_ROWS + 1
    ));
    assert!(super::grok_small_screen_tip_visible(
        super::GROK_SMALL_SCREEN_TIP_MAX_ROWS
    ));
    // Pin the upper bound: the tip is hidden above the max.
    assert!(!super::grok_small_screen_tip_visible(
        super::GROK_SMALL_SCREEN_TIP_MAX_ROWS + 1
    ));
}

#[test]
fn model_selector_rows_renders_provider_slash_model_pairs() {
    use runie_core::model_catalog::ModelCatalogSnapshot;
    use runie_core::types::Model;
    let snapshot = ModelCatalogSnapshot {
        catalog: runie_core::model_catalog::ModelCatalog::new(Vec::new(), Vec::new()),
        query: String::new(),
        scoped_only: false,
        results: vec![
            Model {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                api: "openai".into(),
                provider: "openai".into(),
                ..Default::default()
            },
            Model {
                id: "claude-3-5-sonnet".into(),
                name: "Claude".into(),
                api: "anthropic".into(),
                provider: "anthropic".into(),
                ..Default::default()
            },
        ],
        selected: None,
        last_event: None,
    };
    let rows = super::model_selector_rows(&snapshot);
    assert_eq!(rows, vec!["openai/gpt-4o", "anthropic/claude-3-5-sonnet"]);
}

#[test]
fn model_selector_rows_returns_empty_for_empty_snapshot() {
    use runie_core::model_catalog::ModelCatalogSnapshot;
    let snapshot = ModelCatalogSnapshot::default();
    assert!(super::model_selector_rows(&snapshot).is_empty());
}

#[test]
fn repository_label_renders_home_relative_path() {
    // Pin the home-relative path: a path under the home is rendered
    // with the `~/` prefix so the header stays compact.
    let home = std::path::Path::new("/home/user");
    let path = std::path::Path::new("/home/user/proj/runie");
    assert_eq!(super::repository_label(path, Some(home)), "~/proj/runie");
}

#[test]
fn repository_label_renders_full_path_outside_home() {
    // Pin the negative path: a path outside the home is rendered
    // verbatim so the user can navigate from the absolute path.
    let home = std::path::Path::new("/home/user");
    let path = std::path::Path::new("/tmp/other/runie");
    assert_eq!(
        super::repository_label(path, Some(home)),
        "/tmp/other/runie"
    );
}

#[test]
fn repository_label_returns_full_path_when_home_is_missing() {
    // Pin the missing-home path: a `None` home means the absolute
    // path is rendered as-is so the renderer never has to guess.
    let path = std::path::Path::new("/var/runie");
    assert_eq!(super::repository_label(path, None), "/var/runie");
}

#[test]
fn structured_update_messages_emits_tool_update_for_active_tool() {
    // Pin the smoke path: a `ToolExecutionUpdate` for an active
    // tool emits one `ScrollbackMsg::ToolUpdate` with the projected
    // output lines. The projection is actor-owned and free of any
    // renderer dependency.
    use runie_core::types::AgentEvent;
    let mut active_tools = std::collections::HashSet::new();
    active_tools.insert("bash-1".to_owned());
    let event = AgentEvent::ToolExecutionUpdate {
        tool_call_id: "bash-1".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({}),
        partial_result: serde_json::json!({"output": "line one\nline two"}),
    };
    let messages = super::structured_update_messages(&active_tools, &event);
    assert_eq!(messages.len(), 1);
    match &messages[0] {
        super::ScrollbackMsg::ToolUpdate {
            tool_call_id,
            header,
            output,
        } => {
            assert_eq!(tool_call_id, "bash-1");
            assert!(header.is_none());
            assert_eq!(output, &["line one", "line two"]);
        }
        other => panic!("expected ToolUpdate, got {other:?}"),
    }
}

#[test]
fn structured_update_messages_skips_inactive_or_empty_events() {
    // Pin the negative paths: an inactive tool, an empty partial
    // result, and a non-`ToolExecutionUpdate` event all return
    // empty message vectors so the caller stays a pure projection.
    use runie_core::types::AgentEvent;
    let active_tools = std::collections::HashSet::new();
    let event = AgentEvent::ToolExecutionUpdate {
        tool_call_id: "absent".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({}),
        partial_result: serde_json::json!({"output": "ignored"}),
    };
    assert!(super::structured_update_messages(&active_tools, &event).is_empty());
    let mut active_tools = std::collections::HashSet::new();
    active_tools.insert("bash-1".to_owned());
    let event = AgentEvent::ToolExecutionUpdate {
        tool_call_id: "bash-1".into(),
        tool_name: "bash".into(),
        args: serde_json::json!({}),
        partial_result: serde_json::json!({}),
    };
    assert!(super::structured_update_messages(&active_tools, &event).is_empty());
    let event = AgentEvent::AgentStart;
    assert!(super::structured_update_messages(&active_tools, &event).is_empty());
}

#[test]
fn activity_group_exists_since_latest_user_detects_activity_after_user() {
    // Pin the smoke path: an activity line after the latest user
    // message is detected so the group can be appended.
    let snapshot = super::FeedSnapshot {
        lines: vec![
            super::Line::new(super::LineKind::User, "hello"),
            super::Line::new(super::LineKind::Activity, "running"),
        ],
        ..Default::default()
    };
    assert!(super::activity_group_exists_since_latest_user(&snapshot));
}

#[test]
fn activity_group_exists_since_latest_user_returns_false_without_activity() {
    // Pin the negative path: a transcript with no activity line
    // after the latest user keeps the group-creation flag false.
    let snapshot = super::FeedSnapshot {
        lines: vec![
            super::Line::new(super::LineKind::User, "hello"),
            super::Line::new(super::LineKind::Assistant, "hi"),
        ],
        ..Default::default()
    };
    assert!(!super::activity_group_exists_since_latest_user(&snapshot));
}

#[test]
fn activity_counts_with_start_increments_classified_tool() {
    // Pin the smoke path: a classified tool increments the relevant
    // counter and preserves the rest of the tuple.
    let snapshot = super::FeedSnapshot::default();
    let (dirs, files, commands, subagents, failures) =
        super::activity_counts_with_start(&snapshot, "list_dir", true);
    assert_eq!(dirs, 1);
    assert_eq!(files, 0);
    assert_eq!(commands, 0);
    assert_eq!(subagents, 0);
    assert_eq!(failures, 0);
    // Pin the no-reset path: the existing counters are added to
    // when the new tool fits the same classification.
    let (commands, _, _, _, _) = super::activity_counts_with_start(&snapshot, "bash", false);
    assert_eq!(commands, 0);
}

#[test]
fn activity_counts_projects_snapshot_counters() {
    // Pin the smoke path: the snapshot's activity fields are
    // projected into the canonical tuple shape.
    let snapshot = super::FeedSnapshot {
        facts: super::FeedFacts {
            activity_dirs: 1,
            activity_files: 2,
            activity_commands: 3,
            activity_subagents: 4,
            activity_failures: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(super::activity_counts(&snapshot), (1, 2, 3, 4, 5));
}

#[test]
fn tool_record_events_preserve_independent_name_and_argument_updates() {
    let mut state = super::FeedState::default();
    state.reduce(super::ScrollbackMsg::SetToolName(
        "tool-1".into(),
        "read_file".into(),
    ));
    state.reduce(super::ScrollbackMsg::SetToolArgs(
        "tool-1".into(),
        serde_json::json!({"path": "README.md"}),
    ));

    let snapshot = state.snapshot();
    let record = snapshot
        .facts
        .tools
        .get("tool-1")
        .expect("tool record exists");
    assert_eq!(record.name.as_deref(), Some("read_file"));
    assert_eq!(record.args, Some(serde_json::json!({"path": "README.md"})));

    state.reduce(super::ScrollbackMsg::RemoveToolArgs("tool-1".into()));
    let snapshot = state.snapshot();
    let record = snapshot.facts.tools.get("tool-1").unwrap();
    assert_eq!(record.name.as_deref(), Some("read_file"));
    assert!(record.args.is_none());

    state.reduce(super::ScrollbackMsg::Clear);
    assert!(state.snapshot().facts.tools.is_empty());
}

#[test]
fn tool_record_trace_replays_as_one_memoized_projection() {
    let memo = runie_core::event_trace!(
        super::FeedState::default(),
        |state, event| state.reduce(event.clone()),
        [
            super::ScrollbackMsg::SetToolName("tool-2".into(), "bash".into()),
            super::ScrollbackMsg::SetToolArgs("tool-2".into(), serde_json::json!({"cmd": "pwd"})),
        ]
    );
    let snapshot = memo.state().snapshot();
    let record = snapshot.facts.tools.get("tool-2").expect("trace record");
    assert_eq!(record.name.as_deref(), Some("bash"));
    assert_eq!(record.args, Some(serde_json::json!({"cmd": "pwd"})));
    assert_eq!(memo.events().len(), 2);
}

#[test]
fn active_tool_count_filters_running_blocks() {
    // Pin the smoke path: a snapshot with two running blocks and
    // one completed block reports two active tools.
    let snapshot = super::FeedSnapshot {
        tool_blocks: vec![
            super::ToolBlock {
                tool_call_id: "a".into(),
                header: "a".into(),
                kind: super::ToolCardKind::Execute,
                output: Vec::new(),
                mode: runie_core::types::ToolDisplayMode::Truncated,
                is_running: true,
                is_error: false,
                tool_row_id: None,
            },
            super::ToolBlock {
                tool_call_id: "b".into(),
                header: "b".into(),
                kind: super::ToolCardKind::Read,
                output: Vec::new(),
                mode: runie_core::types::ToolDisplayMode::Truncated,
                is_running: false,
                is_error: false,
                tool_row_id: None,
            },
            super::ToolBlock {
                tool_call_id: "c".into(),
                header: "c".into(),
                kind: super::ToolCardKind::WebSearch,
                output: Vec::new(),
                mode: runie_core::types::ToolDisplayMode::Truncated,
                is_running: true,
                is_error: false,
                tool_row_id: None,
            },
        ],
        ..Default::default()
    };
    assert_eq!(super::active_tool_count(&snapshot), 2);
}

#[test]
fn current_tool_args_returns_null_for_absent_tool() {
    // Pin the negative path: an absent tool id returns a `Null`
    // JSON value so the caller's optional-argument contract is
    // preserved.
    let snapshot = super::FeedSnapshot::default();
    assert_eq!(
        super::current_tool_args(&snapshot, "absent"),
        serde_json::Value::Null
    );
}

#[test]
fn background_messages_for_event_emits_subagent_setup() {
    // Pin the smoke path: a `BackgroundWorkStarted` event emits a
    // `SetToolName` + `SetToolMode` + `ToolStart` triple so the
    // actor-owned background projection agrees with the renderer.
    use runie_core::types::AgentEvent;
    let event = AgentEvent::BackgroundWorkStarted {
        work_id: "subagent-1".into(),
        description: "research".into(),
        background: false,
    };
    let messages = super::background_messages_for_event(&event);
    assert_eq!(messages.len(), 3);
    assert!(matches!(
        &messages[0],
        super::ScrollbackMsg::SetToolName(id, name)
            if id == "subagent-1" && name == "subagent"
    ));
    assert!(matches!(
        &messages[1],
        super::ScrollbackMsg::SetToolMode(id, _)
            if id == "subagent-1"
    ));
}

#[test]
fn background_messages_for_event_emits_subagent_tool_start() {
    // Pin the third message of the subagent lifecycle: the
    // `ToolStart` row carries the `Subagent running: ...` header
    // and a `None` activity.
    use runie_core::types::AgentEvent;
    let event = AgentEvent::BackgroundWorkStarted {
        work_id: "subagent-1".into(),
        description: "research".into(),
        background: false,
    };
    let messages = super::background_messages_for_event(&event);
    assert!(matches!(
        &messages[2],
        super::ScrollbackMsg::ToolStart {
            tool_call_id,
            header,
            activity
        } if tool_call_id == "subagent-1"
            && header == "Subagent running: \"research\""
            && activity.is_none()
    ));
}

#[test]
fn background_messages_for_event_returns_empty_for_non_background() {
    // Pin the negative path: a non-background event returns an
    // empty vector so the model's caller can keep the
    // pass-through behavior.
    use runie_core::types::AgentEvent;
    let event = AgentEvent::AgentStart;
    assert!(super::background_messages_for_event(&event).is_empty());
}

#[test]
fn bus_messages_for_event_emits_clear_for_reset() {
    // Pin the smoke path: a `Reset` event emits a single
    // `ScrollbackMsg::Clear` so the actor-owned bus projection
    // and the renderer agree on the reset semantics.
    use runie_core::types::AgentEvent;
    let messages = super::bus_messages_for_event(&AgentEvent::Reset);
    assert_eq!(messages, vec![super::ScrollbackMsg::Clear]);
}
