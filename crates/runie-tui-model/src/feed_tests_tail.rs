use super::*;
#[test]
fn feed_state_reduces_event_sequence_without_renderer_types() {
    let mut state = super::FeedState::default();
    for message in [
        super::ScrollbackMsg::Append(super::Line::new(super::LineKind::User, "Hey")),
        super::ScrollbackMsg::SetToolName("call-1".into(), "read".into()),
        super::ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        },
        super::ScrollbackMsg::ToolUpdate {
            tool_call_id: "call-1".into(),
            header: None,
            output: vec!["line one".into()],
        },
        super::ScrollbackMsg::ToolEnd {
            tool_call_id: "call-1".into(),
            header: "Read README.md (1 line)".into(),
            activity: None,
            output: vec![(super::LineKind::ToolResult, "done".into())],
        },
    ] {
        state.reduce(message);
    }
    let snapshot = state.snapshot();
    assert_eq!(snapshot.lines[0].kind, super::LineKind::User);
    assert_eq!(snapshot.tool_blocks.len(), 1);
    assert_eq!(snapshot.tool_blocks[0].output, ["line one", "done"]);
    assert_eq!(snapshot.tool_blocks[0].kind, super::ToolCardKind::Read);
}

#[test]
fn terminal_tool_output_replay_is_not_appended_twice() {
    let mut state = super::FeedState::default();
    state.reduce(super::ScrollbackMsg::SetToolName(
        "call-1".into(),
        "read".into(),
    ));
    state.reduce(super::ScrollbackMsg::ToolStart {
        tool_call_id: "call-1".into(),
        header: "Read README.md".into(),
        activity: None,
    });
    state.reduce(super::ScrollbackMsg::ToolUpdate {
        tool_call_id: "call-1".into(),
        header: None,
        output: vec!["first".into(), "second".into()],
    });
    state.reduce(super::ScrollbackMsg::ToolEnd {
        tool_call_id: "call-1".into(),
        header: "Read README.md (2 lines)".into(),
        activity: Some("completed".into()),
        output: vec![
            (super::LineKind::ToolResult, "first".into()),
            (super::LineKind::ToolResult, "second".into()),
        ],
    });
    assert_eq!(state.snapshot().tool_blocks[0].output, ["first", "second"]);
}

#[test]
fn workflow_phase_glyphs_match_grok_fallback_for_terminal_states() {
    assert_eq!(
        super::workflow_text(
            "Workflow release: ship it",
            &[("upload".into(), "cancelled".into())],
            "cancelled",
            Some(900),
            0,
        ),
        "Workflow release ◌ cancelled after 0.9s: ship it  [upload ○]"
    );
}

#[test]
fn running_generic_fold_cycle_is_preserved_by_model_delegation() {
    let mut state = super::FeedState::default();
    state.reduce(super::ScrollbackMsg::ToolStartRunning {
        tool_call_id: "call-1".into(),
        header: "custom_tool running".into(),
        activity: None,
    });
    state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
    assert_eq!(
        state.snapshot().tool_blocks[0].mode,
        ToolDisplayMode::Truncated
    );
    state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
    assert_eq!(
        state.snapshot().tool_blocks[0].mode,
        ToolDisplayMode::Expanded
    );
}

#[test]
fn read_card_settles_collapsed_after_completion() {
    let mut state = super::FeedState::default();
    state.reduce(super::ScrollbackMsg::SetToolName(
        "read-1".into(),
        "read".into(),
    ));
    state.reduce(super::ScrollbackMsg::ToolStart {
        tool_call_id: "read-1".into(),
        header: "Read README.md".into(),
        activity: None,
    });
    state.reduce(super::ScrollbackMsg::SetToolMode(
        "read-1".into(),
        ToolDisplayMode::Expanded,
    ));
    state.reduce(super::ScrollbackMsg::ToolEnd {
        tool_call_id: "read-1".into(),
        header: "Read README.md (2 lines)".into(),
        activity: None,
        output: vec![],
    });
    assert_eq!(
        state.snapshot().tool_blocks[0].mode,
        ToolDisplayMode::Collapsed
    );
}

#[test]
fn layout_measurement_is_delivered_through_the_feed_event_boundary() {
    let mut state = FeedState::default();
    state.reduce(super::ScrollbackMsg::LayoutMeasured {
        content_rows: 42,
        viewport_rows: 12,
        anchor_row: Some(9),
    });
    let snapshot = state.snapshot();
    assert_eq!(snapshot.measured_content_rows, 42);
    assert_eq!(snapshot.measured_viewport_rows, 12);
    assert_eq!(snapshot.measured_anchor_row, Some(9));
}

#[test]
fn measured_anchor_restores_manual_viewport_after_tool_fold() {
    let mut state = FeedState::default();
    state.reduce(super::ScrollbackMsg::ToolStartRunning {
        tool_call_id: "call-1".into(),
        header: "custom_tool running".into(),
        activity: None,
    });
    state.reduce(super::ScrollbackMsg::LayoutMeasured {
        content_rows: 30,
        viewport_rows: 6,
        anchor_row: Some(17),
    });
    state.reduce(super::ScrollbackMsg::ScrollBy(3));
    state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
    assert_eq!(state.snapshot().scroll_offset, 4);
    state.reduce(super::ScrollbackMsg::LayoutMeasured {
        content_rows: 34,
        viewport_rows: 6,
        anchor_row: Some(21),
    });
    assert_eq!(state.snapshot().scroll_offset, 8);
    assert!(!state.snapshot().autoscroll);
}

#[test]
fn measured_anchor_tracks_multi_row_reflow_in_both_directions() {
    assert_eq!(super::measured_anchor_delta(Some(4), Some(11)), 7);
    assert_eq!(super::measured_anchor_delta(Some(11), Some(4)), -7);
    assert_eq!(super::measured_anchor_delta(None, Some(4)), 0);
    assert_eq!(super::measured_anchor_delta(Some(4), None), 0);
}

#[test]
fn web_search_sources_line_dedups_and_keeps_first_seen_order() {
    assert_eq!(
            super::web_search_sources_line(
                "https://docs.rs/runie https://docs.rs/ratatui https://rust-lang.org/learn https://github.com/runie https://docs.rs/extra"
            ),
            Some("  Sources: docs.rs, rust-lang.org, github.com".to_owned())
        );
}

#[test]
fn web_search_sources_line_returns_none_for_empty_source_line() {
    assert_eq!(super::web_search_sources_line(""), None);
    assert_eq!(super::web_search_sources_line("   \n  "), None);
    assert_eq!(super::web_search_sources_line("no citations"), None);
}

#[test]
fn web_search_sources_line_paginates_with_plus_n_more() {
    assert_eq!(
            super::web_search_sources_line(
                "https://a.example https://b.example https://c.example https://d.example https://e.example"
            ),
            Some("  Sources: a.example, b.example, c.example (+2 more)".to_owned())
        );
}

#[test]
fn web_search_sources_line_trims_url_terminators_and_punctuation() {
    assert_eq!(
            super::web_search_sources_line(
                "see https://docs.rs/runie/page, https://crates.io?q=foo#bar and https://github.com/path) (also https://github.com/path] more"
            ),
            Some("  Sources: docs.rs, crates.io, github.com".to_owned())
        );
}

#[test]
fn web_search_site_count_dedups_case_insensitively() {
    assert_eq!(
        super::web_search_site_count(
            "https://docs.rs/a\nhttps://DOCS.RS/b\nhttps://rust-lang.org/learn"
        ),
        2
    );
}

#[test]
fn web_search_site_count_trims_url_terminators_and_punctuation() {
    assert_eq!(
        super::web_search_site_count(
            "see https://docs.rs/a), https://crates.io?q=foo#bar, https://github.com/b"
        ),
        3
    );
}

#[test]
fn web_search_site_count_falls_back_to_non_empty_lines_when_url_free() {
    assert_eq!(super::web_search_site_count("one\ntwo\n\nthree\n"), 3);
    assert_eq!(super::web_search_site_count("plain prose only"), 1);
}

#[test]
fn completed_tool_header_with_args_pins_search_tools_aliases_and_cardinality() {
    let empty_args = serde_json::json!({});
    assert_eq!(
        super::completed_tool_header_with_args(
            "Search tools",
            "search_tools",
            &empty_args,
            &serde_json::json!("tool_alpha"),
        ),
        "Search tools (1 result)"
    );
    assert_eq!(
        super::completed_tool_header_with_args(
            "Search tools",
            "search-tools",
            &empty_args,
            &serde_json::json!("tool_alpha\ntool_beta\ntool_gamma"),
        ),
        "Search tools (3 results)"
    );
    assert_eq!(
        super::completed_tool_header_with_args(
            "Search tools",
            "search_tool",
            &empty_args,
            &serde_json::json!("tool_alpha\n\ntool_beta"),
        ),
        "Search tools (2 results)"
    );
}

#[test]
fn tool_header_pins_search_tools_aliases_and_workspace_anchor() {
    let workspace = "/repo/root";
    for alias in ["search_tools", "search-tools", "search_tool"] {
        assert_eq!(
            super::tool_header(alias, &serde_json::json!({"query": "alpha"}), workspace),
            "Search Tools alpha",
            "alias: {alias}"
        );
    }
    assert_eq!(
        super::tool_header(
            "search_tools",
            &serde_json::json!({"pattern": "alpha"}),
            workspace,
        ),
        "Search Tools alpha"
    );
    assert_eq!(
        super::tool_header("search_tools", &serde_json::json!({}), workspace),
        "Search Tools "
    );
    assert_eq!(
        super::tool_header(
            "search_tools",
            &serde_json::json!({"query": "first", "pattern": "second"}),
            workspace,
        ),
        "Search Tools first"
    );
    assert_eq!(
        super::tool_header(
            "search_tools",
            &serde_json::json!({"query": "alpha"}),
            "/different/anchor",
        ),
        "Search Tools alpha"
    );
}

#[test]
fn completed_tool_header_with_args_routes_read_file_image_content() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Read src/diagram.png",
            "read_file",
            &serde_json::json!({"path": "src/diagram.png"}),
            &serde_json::json!({
                "content": [
                    {"type": "image", "data": "ZmFrZQ=="}
                ]
            })
        ),
        "Read src/diagram.png (image)"
    );
}

#[test]
fn completed_tool_header_with_args_renders_read_file_offset_range_with_total() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Read src/lib.rs",
            "read_file",
            &serde_json::json!({"offset": 40, "limit": 20}),
            &serde_json::json!({
                "content": [{"text": "line 41\nline 42\n[18 more lines in file. Use offset=61 to continue.]"}],
                "details": {"truncation": {"totalLines": 100}}
            })
        ),
        "Read src/lib.rs (41-42 of 100)"
    );
}

#[test]
fn completed_tool_header_with_args_projects_list_dir_cardinality() {
    let args = serde_json::json!({});
    assert_eq!(
        super::completed_tool_header_with_args(
            "List .",
            "list_dir",
            &args,
            &serde_json::json!("Cargo.toml"),
        ),
        "List . (1 entry)"
    );
    assert_eq!(
        super::completed_tool_header_with_args(
            "List .",
            "list_files",
            &args,
            &serde_json::json!("Cargo.toml\nsrc\ncrates"),
        ),
        "List . (3 entries)"
    );
}

#[test]
fn completed_tool_header_with_args_projects_read_line_count() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Read README.md",
            "read",
            &serde_json::json!({}),
            &serde_json::json!("a\nb"),
        ),
        "Read README.md (2 lines)"
    );
}

#[test]
fn completed_tool_header_with_args_projects_search_match_cardinality() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Search \"TODO\"",
            "search",
            &serde_json::json!({}),
            &serde_json::json!("a\nb"),
        ),
        "Search \"TODO\" (2 matches)"
    );
}

#[test]
fn completed_tool_header_with_args_projects_edit_count() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Edit src/main.rs",
            "edit",
            &serde_json::json!({}),
            &serde_json::json!("hunk"),
        ),
        "Edit src/main.rs (1 edit)"
    );
}

#[test]
fn completed_tool_header_with_args_routes_workflow_to_completed_label() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Workflow release",
            "workflow",
            &serde_json::json!({}),
            &serde_json::json!("done"),
        ),
        "Workflow completed: release"
    );
}

#[test]
fn completed_tool_header_with_args_routes_use_to_used_label() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Use git_status",
            "use",
            &serde_json::json!({}),
            &serde_json::json!("{}"),
        ),
        "Used git_status"
    );
}

#[test]
fn completed_tool_header_with_args_routes_subagent_to_completed_label() {
    assert_eq!(
        super::completed_tool_header_with_args(
            "Subagent started: research",
            "subagent",
            &serde_json::json!({}),
            &serde_json::json!("done"),
        ),
        "Subagent completed: research"
    );
}
