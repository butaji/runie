use super::*;
#[test]
fn snapshot_facts_are_the_complete_navigation_facts_projection() {
    let mut state = super::FeedState::default();
    state.reduce(super::ScrollbackMsg::TurnStart);
    state.reduce(super::ScrollbackMsg::SetToolName(
        "tool-1".into(),
        "read".into(),
    ));
    let snapshot = state.snapshot();
    assert_eq!(snapshot.facts, state.navigation.facts);
}
#[test]
fn lifecycle_yaml_trace_replays_through_the_feed_reducer() {
    let state = runie_core::replay_yaml_state(
        include_str!("../fixtures/scrollback-lifecycle.yaml"),
        super::FeedState::default(),
        |state, event: &super::ScrollbackLifecycleEvent| {
            for message in super::ScrollbackEvent::Lifecycle(event.clone()).into_messages() {
                state.reduce(message);
            }
        },
    )
    .expect("valid lifecycle fixture");
    assert!(!state.snapshot().facts.turn_started);
    assert!(!state.snapshot().facts.assistant_stream_open);
    assert!(state.snapshot().lines.is_empty());
}
#[test]
fn tool_yaml_trace_replays_through_the_normalized_tool_projection() {
    let state = runie_core::replay_yaml_state(
        include_str!("../fixtures/scrollback-tool.yaml"),
        super::FeedState::default(),
        |state, event: &super::ScrollbackToolEvent| {
            for message in super::ScrollbackEvent::Tool(event.clone()).into_messages() {
                state.reduce(message);
            }
        },
    )
    .expect("valid tool fixture");
    let snapshot = state.snapshot();
    assert_eq!(snapshot.tool_blocks.len(), 1);
    assert_eq!(snapshot.tool_blocks[0].tool_call_id, "tool-1");
    assert!(!snapshot.tool_blocks[0].is_running);
}
#[test]
fn workflow_yaml_trace_replays_through_the_workflow_projection() {
    let state = runie_core::replay_yaml_state(
        include_str!("../fixtures/scrollback-workflow.yaml"),
        super::FeedState::default(),
        |state, event: &super::ScrollbackWorkflowEvent| {
            for message in super::ScrollbackEvent::Workflow(event.clone()).into_messages() {
                state.reduce(message);
            }
        },
    )
    .expect("valid workflow fixture");
    let snapshot = state.snapshot();
    assert_eq!(
        snapshot.facts.workflow_headers["run-1"],
        "Workflow review: inspect changes"
    );
    assert_eq!(snapshot.facts.workflow_phases["run-1"][0].0, "analysis");
    assert_eq!(snapshot.facts.workflow_phases["run-1"][0].1, "active");
}
#[test]
fn line_kind_prefix_pins_user_and_assistant_rails() {
    // Pin the user/assistant prefix shapes: the user gutter is
    // three columns wide, the assistant/reasoning rail is two
    // columns, and the thinking status matches the assistant rail.
    assert_eq!(super::LineKind::User.prefix(), "   ❯ ");
    assert_eq!(super::LineKind::Assistant.prefix(), "┃  ");
    assert_eq!(super::LineKind::Reasoning.prefix(), "┃  ");
    assert_eq!(super::LineKind::ThinkingStatus.prefix(), "┃  ");
}
#[test]
fn line_kind_prefix_pins_tool_card_glyphs() {
    // Pin the tool-card prefix shapes: the running/completed/error
    // tool glyphs share the `◆ ` marker, the result arrow is
    // `  ↳ `, and the structured output is indented by two spaces.
    assert_eq!(super::LineKind::Tool.prefix(), "◆ ");
    assert_eq!(super::LineKind::ToolRunning.prefix(), "◆ ");
    assert_eq!(super::LineKind::ToolError.prefix(), "◆ ");
    assert_eq!(super::LineKind::ToolResult.prefix(), "  ↳ ");
    assert_eq!(super::LineKind::ToolOutput.prefix(), "  ");
}
#[test]
fn line_kind_prefix_pins_session_and_metadata_rows() {
    // Pin the session/system/separator prefix shapes: the
    // session-start/system rows are blank or prefixed, the
    // separator is empty, and the assistant-completion rows
    // are blank.
    assert_eq!(super::LineKind::SessionStart.prefix(), "   ");
    assert_eq!(super::LineKind::System.prefix(), "   * ");
    assert_eq!(super::LineKind::Separator.prefix(), "");
    assert_eq!(super::LineKind::TurnSummary.prefix(), "   ");
    assert_eq!(super::LineKind::CompletedAssistant.prefix(), "   ");
}
#[test]
fn line_kind_prefix_pins_activity_rail() {
    // Pin the activity rail: the activity row carries the `❙  `
    // marker so the fold-summary rail stays aligned with the
    // assistant gutter.
    assert_eq!(super::LineKind::Activity.prefix(), "❙  ");
}
#[test]
fn line_kind_tool_predicates_share_the_declared_vocabulary() {
    for kind in [LineKind::Tool, LineKind::ToolRunning, LineKind::ToolError] {
        assert!(kind.is_tool_header());
        assert!(kind.is_tool_line());
    }
    for kind in [LineKind::ToolOutput, LineKind::ToolResult] {
        assert!(!kind.is_tool_header());
        assert!(kind.is_tool_line());
    }
    assert!(LineKind::Tool.is_live_tool_header());
    assert!(LineKind::ToolRunning.is_live_tool_header());
    assert!(!LineKind::ToolError.is_live_tool_header());
    assert!(LineKind::User.is_selectable_transcript());
    assert!(LineKind::Assistant.is_selectable_transcript());
    assert!(!LineKind::System.is_selectable_transcript());
    assert!(!LineKind::Assistant.is_tool_line());
}
#[test]
fn format_clock_timestamp_pins_short_clock_shape() {
    // Pin the fallback path: when libc cannot resolve the local clock,
    // the UTC-derived 12-hour shape with a zero-padded minute is still
    // emitted so the label stays well-formed for replay and live paths.
    for timestamp in [0, 13 * 3_600 + 7 * 60, 12 * 3_600] {
        let formatted = super::format_clock_timestamp(timestamp);
        assert!(formatted.contains(':'), "{formatted}");
        assert!(
            formatted.ends_with(" AM") || formatted.ends_with(" PM"),
            "{formatted}"
        );
        // Pin the zero-padded minute shape: the colon-aligned header
        // must keep minutes as exactly two digits regardless of
        // meridiem or 12-hour rollover.
        let minute_segment = formatted
            .split(':')
            .nth(1)
            .unwrap_or_else(|| panic!("missing minute segment: {formatted}"));
        let minute_part = minute_segment
            .split(' ')
            .next()
            .unwrap_or_else(|| panic!("missing minute digits: {formatted}"));
        assert_eq!(
            minute_part.len(),
            2,
            "minute must be zero-padded to two digits: {formatted}"
        );
    }
}
#[test]
fn tool_update_header_text_appends_serialized_json_fragment() {
    assert_eq!(
        super::tool_update_header_text(
            "Run ls",
            &serde_json::json!({"status": "running", "step": 2})
        ),
        "Run ls | update: {\"status\":\"running\",\"step\":2}"
    );
    assert_eq!(
        super::tool_update_header_text("Read src/lib.rs", &serde_json::Value::Null),
        "Read src/lib.rs | update: null"
    );
}
#[test]
fn tool_update_header_text_keeps_separator_for_empty_serialization() {
    // `serde_json::Value` always serializes, so the `unwrap_or_default()`
    // fallback degrades to an empty fragment rather than a panic. Pin the
    // header shape around a minimal payload and around the empty default.
    let fragment = serde_json::to_string(&serde_json::json!({})).unwrap_or_default();
    assert_eq!(fragment, "{}");
    assert_eq!(
        super::tool_update_header_text("Run ls", &serde_json::json!({})),
        format!("Run ls | update: {fragment}")
    );
    assert_eq!(
        super::tool_update_header_text("", &serde_json::json!({})),
        " | update: {}"
    );
}
#[test]
fn edit_aliases_match_groks_edit_card_family() {
    for header in [
        "apply_patch",
        "apply_patch src/lib.rs",
        "strreplace",
        "edit",
    ] {
        assert_eq!(ToolCardKind::from_header(header), ToolCardKind::Edit);
    }
}
#[test]
fn ls_alias_matches_groks_list_dir_card_family() {
    assert_eq!(ToolCardKind::from_header("ls"), ToolCardKind::ListDir);
    assert_eq!(ToolCardKind::from_header("ls src"), ToolCardKind::ListDir);
}
#[test]
fn terminal_command_aliases_match_groks_execute_family() {
    for header in ["execute", "run_terminal_command", "run_terminal_cmd"] {
        assert_eq!(ToolCardKind::from_header(header), ToolCardKind::Execute);
        assert_eq!(
            default_tool_display_mode(header),
            ToolDisplayMode::Truncated
        );
    }
}
#[test]
fn grok_search_aliases_keep_their_specialized_card_families() {
    assert_eq!(ToolCardKind::from_header("glob"), ToolCardKind::Search);
    assert_eq!(
        ToolCardKind::from_header("search_tool"),
        ToolCardKind::SearchTools
    );
}
#[test]
fn tool_projection_is_ordered_and_renderer_independent() {
    let lines = vec![
        Line::new(LineKind::Tool, "read src/lib.rs").for_tool("second"),
        Line::new(LineKind::ToolOutput, "line").for_tool("second"),
        Line::new(LineKind::ToolRunning, "bash cargo test").for_tool("first"),
    ];
    let names = HashMap::from([
        ("second".to_owned(), "read".to_owned()),
        ("first".to_owned(), "bash".to_owned()),
    ]);
    let blocks = project_tool_blocks(&lines, &names, &HashMap::new());
    assert_eq!(
        blocks
            .iter()
            .map(|block| block.tool_call_id.as_str())
            .collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert_eq!(blocks[0].output, ["line"]);
    assert_eq!(blocks[1].kind, ToolCardKind::Execute);
}
#[test]
fn typed_card_rows_expose_semantic_paint_intents() {
    let header = ToolCardRow {
        tool_call_id: "read-1".into(),
        tool_row_id: Some(7),
        member_index: 0,
        card_kind: ToolCardKind::Read,
        row_kind: ToolCardRowKind::Header,
        text: "Read file".into(),
        mode: ToolDisplayMode::Collapsed,
        is_running: true,
        is_error: false,
    };
    let output = ToolCardRow {
        row_kind: ToolCardRowKind::Content,
        ..header.clone()
    };
    let error = ToolCardRow {
        row_kind: ToolCardRowKind::Status,
        is_running: false,
        is_error: true,
        ..header.clone()
    };
    let memory = ToolCardRow {
        card_kind: ToolCardKind::MemorySearch,
        row_kind: ToolCardRowKind::Content,
        ..header.clone()
    };
    assert_eq!(header.paint_intent(), ToolCardPaintIntent::Running);
    let mut settled_header = header.clone();
    settled_header.is_running = false;
    assert_eq!(settled_header.paint_intent(), ToolCardPaintIntent::Header);
    assert_eq!(output.paint_intent(), ToolCardPaintIntent::Content);
    assert_eq!(error.paint_intent(), ToolCardPaintIntent::Error);
    assert_eq!(memory.paint_intent(), ToolCardPaintIntent::Muted);
}
#[test]
fn card_rows_preserve_specialized_identity_and_semantic_role() {
    let lines = vec![
        Line::new(LineKind::Tool, "Read README.md")
            .for_tool("call-1")
            .for_tool_row(41),
        Line::new(LineKind::ToolOutput, "first line")
            .for_tool("call-1")
            .for_tool_row(41),
        Line::new(LineKind::ToolError, "failed")
            .for_tool("call-1")
            .for_tool_row(41),
    ];
    let names = HashMap::from([(String::from("call-1"), String::from("read"))]);
    let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
    assert_eq!(rows[0].card_kind, ToolCardKind::Read);
    assert_eq!(rows[0].row_kind, ToolCardRowKind::Header);
    assert_eq!(rows[0].tool_row_id, Some(41));
    assert_eq!(rows[1].tool_row_id, Some(41));
    assert_eq!(rows[1].row_kind, ToolCardRowKind::Content);
    assert!(rows[2].is_error);
    assert_eq!(rows[2].row_kind, ToolCardRowKind::Status);
}
#[test]
fn duplicate_call_ids_keep_live_row_member_ordinals_distinct() {
    let lines = vec![
        Line::new(LineKind::Tool, "Read one")
            .for_tool("duplicate")
            .for_tool_row(1),
        Line::new(LineKind::ToolOutput, "one")
            .for_tool("duplicate")
            .for_tool_row(1),
        Line::new(LineKind::Tool, "Read two")
            .for_tool("duplicate")
            .for_tool_row(2),
        Line::new(LineKind::ToolOutput, "two")
            .for_tool("duplicate")
            .for_tool_row(2),
    ];
    let names = HashMap::from([(String::from("duplicate"), String::from("read"))]);
    let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
    assert_eq!(
        rows.iter().map(|row| row.member_index).collect::<Vec<_>>(),
        [0, 0, 1, 1]
    );
    assert_eq!(
        super::logical_tool_member_indices(&lines),
        [Some(0), Some(0), Some(1), Some(1)]
    );
    assert_eq!(super::logical_tool_member_index_at(&lines, 2), Some(1));
}
#[test]
fn selected_snapshot_uses_exact_live_row_member_identity() {
    let lines = vec![
        Line::new(LineKind::Tool, "first")
            .for_tool("duplicate")
            .for_tool_row(1),
        Line::new(LineKind::Tool, "second")
            .for_tool("duplicate")
            .for_tool_row(2),
    ];
    let mut state = FeedState {
        lines,
        ..FeedState::default()
    };
    state.navigation.selected_entry = Some(1);
    assert_eq!(state.snapshot().selected_member_index, Some(1));
}
#[test]
fn keyboard_selection_keeps_duplicate_live_cards_selectable() {
    let mut state = FeedState {
        lines: vec![
            Line::new(LineKind::Tool, "first")
                .for_tool("duplicate")
                .for_tool_row(1),
            Line::new(LineKind::Tool, "second")
                .for_tool("duplicate")
                .for_tool_row(2),
        ],
        ..FeedState::default()
    };
    assert_eq!(state.selectable_entries(), vec![0, 1]);
    state.select_entry(1);
    state.select_entry(1);
    assert_eq!(state.navigation.selected_entry, Some(1));
    state.navigation.selected_entry = None;
    state.navigation.selected_tool_id = None;
    state.reduce(super::ScrollbackMsg::SelectNextTool);
    assert_eq!(state.navigation.selected_entry, Some(0));
    state.reduce(super::ScrollbackMsg::SelectNextTool);
    assert_eq!(state.navigation.selected_entry, Some(1));
    assert_eq!(state.snapshot().selected_tool_row_id, Some(2));
}
#[test]
fn tool_block_output_stays_with_duplicate_live_card_identity() {
    let lines = vec![
        Line::new(LineKind::Tool, "first")
            .for_tool("duplicate")
            .for_tool_row(1),
        Line::new(LineKind::ToolOutput, "first output")
            .for_tool("duplicate")
            .for_tool_row(1),
        Line::new(LineKind::Tool, "second")
            .for_tool("duplicate")
            .for_tool_row(2),
        Line::new(LineKind::ToolOutput, "second output")
            .for_tool("duplicate")
            .for_tool_row(2),
    ];
    let blocks = project_tool_blocks(
        &lines,
        &HashMap::<String, String>::new(),
        &HashMap::<String, ToolDisplayMode>::new(),
    );
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].output, ["first output"]);
    assert_eq!(blocks[1].output, ["second output"]);
}
#[test]
fn tool_card_summary_reduces_output_metadata_as_data() {
    let lines = vec![
        Line::new(LineKind::Tool, "Read").for_tool("read-1"),
        Line::new(LineKind::ToolOutput, "alpha").for_tool("read-1"),
        Line::new(LineKind::ToolOutput, "beta").for_tool("read-1"),
        Line::new(LineKind::ToolResult, "[output truncated]").for_tool("read-1"),
    ];
    let names = HashMap::from([("read-1".to_owned(), "read".to_owned())]);
    let summaries = super::tool_card_summaries(&lines, &names);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].output_lines, 3);
    assert_eq!(
        summaries[0].output_bytes,
        "alpha".len() + "beta".len() + "[output truncated]".len()
    );
    assert!(summaries[0].truncated);
    assert_eq!(
        summaries[0].output_preview.as_deref(),
        Some("alpha\nbeta\n[output truncated]")
    );
    assert_eq!(summaries[0].card_kind, ToolCardKind::Read);
}
#[test]
fn memory_card_rows_separate_metadata_from_snippet_content() {
    let lines = vec![
        Line::new(LineKind::Tool, "Memory Search actors").for_tool("memory-1"),
        Line::new(
            LineKind::ToolOutput,
            "  1. notes.md:4-8  (score: 0.72, global)",
        )
        .for_tool("memory-1"),
        Line::new(LineKind::ToolOutput, "    actor state").for_tool("memory-1"),
    ];
    let names = HashMap::from([(String::from("memory-1"), String::from("memory_search"))]);
    let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
    assert_eq!(rows[1].row_kind, ToolCardRowKind::Metadata);
    assert_eq!(rows[1].paint_intent(), ToolCardPaintIntent::Muted);
    assert_eq!(rows[2].row_kind, ToolCardRowKind::Content);
}
#[test]
fn web_search_sources_are_metadata_rows() {
    let lines = vec![
        Line::new(LineKind::Tool, "Web Search rust").for_tool("web-1"),
        Line::new(LineKind::ToolOutput, "  Sources: docs.rs, rust-lang.org").for_tool("web-1"),
    ];
    let names = HashMap::from([(String::from("web-1"), String::from("web_search"))]);
    let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
    assert_eq!(rows[1].row_kind, ToolCardRowKind::Metadata);
    assert_eq!(rows[1].paint_intent(), ToolCardPaintIntent::Muted);
}
#[test]
fn web_fetch_response_fields_are_metadata_rows() {
    let lines = vec![
        Line::new(LineKind::Tool, "Fetch https://example.com").for_tool("fetch-1"),
        Line::new(LineKind::ToolOutput, "status: 200").for_tool("fetch-1"),
        Line::new(LineKind::ToolOutput, "content_type: text/html").for_tool("fetch-1"),
        Line::new(LineKind::ToolOutput, "title: Release notes").for_tool("fetch-1"),
        Line::new(LineKind::ToolOutput, "body").for_tool("fetch-1"),
    ];
    let names = HashMap::from([(String::from("fetch-1"), String::from("web_fetch"))]);
    let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
    assert_eq!(rows[1].row_kind, ToolCardRowKind::Metadata);
    assert_eq!(rows[2].row_kind, ToolCardRowKind::Metadata);
    assert_eq!(rows[3].row_kind, ToolCardRowKind::Metadata);
    assert_eq!(rows[4].row_kind, ToolCardRowKind::Content);
}
#[test]
fn navigation_transitions_are_pure_and_resettable() {
    let mut navigation = super::FeedNavigation::default();
    navigation.advance_animation();
    navigation.detach_from_tail();
    navigation.reveal_latest(12);
    assert_eq!(navigation.animation_frame, 1);
    assert_eq!(navigation.scroll_offset, 12);
    assert!(navigation.autoscroll);
    assert!(!navigation.follow_latest_user);
    navigation.reset();
    assert_eq!(navigation, super::FeedNavigation::default());
}
