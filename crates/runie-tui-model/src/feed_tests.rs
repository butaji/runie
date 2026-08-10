use super::{
    classify_activity_tool, default_tool_display_mode, is_transport_only_update,
    structured_update_text, ActivityKind, FeedState,
};
use runie_core::types::ToolDisplayMode;

#[test]
fn is_output_tool_pins_every_alias_and_excludes_others() {
    for alias in [
        "list_dir",
        "list_files",
        "read",
        "read_file",
        "web_fetch",
        "web-fetch",
        "fetch",
        "memory_search",
        "memory-search",
    ] {
        assert!(super::is_output_tool(alias), "alias: {alias}");
    }
    for name in ["bash", "subagent"] {
        assert!(!super::is_output_tool(name), "name: {name}");
    }
    assert!(!super::is_output_tool("unknown"));
}

#[test]
fn activity_classifier_pins_every_alias() {
    let cases = [
        (
            ActivityKind::Dir,
            ["list_dir", "list_files", "ls"].as_slice(),
        ),
        (ActivityKind::File, ["read", "read_file"].as_slice()),
        (
            ActivityKind::Command,
            [
                "bash",
                "shell",
                "exec",
                "run",
                "execute",
                "run_terminal_command",
                "run_terminal_cmd",
            ]
            .as_slice(),
        ),
        (
            ActivityKind::Subagent,
            ["subagent", "agent", "task"].as_slice(),
        ),
    ];
    for (kind, aliases) in cases {
        for alias in aliases {
            assert_eq!(classify_activity_tool(alias), Some(kind), "alias: {alias}");
        }
    }
    assert_eq!(classify_activity_tool("unknown"), None);
}

#[test]
fn structured_update_prefers_output_over_content() {
    let value = serde_json::json!({
        "output": "from-output",
        "content": "from-content",
    });
    assert_eq!(
        structured_update_text(&value).as_deref(),
        Some("from-output")
    );
}

#[test]
fn structured_update_falls_back_to_content_when_output_missing() {
    let value = serde_json::json!({"content": "from-content"});
    assert_eq!(
        structured_update_text(&value).as_deref(),
        Some("from-content")
    );
}

#[test]
fn structured_update_returns_none_for_non_string_envelope() {
    assert!(structured_update_text(&serde_json::json!({"status": "running"})).is_none());
    assert!(structured_update_text(&serde_json::json!({"output": 7})).is_none());
    assert!(structured_update_text(&serde_json::json!({"content": ["line"]})).is_none());
    assert!(structured_update_text(&serde_json::Value::Null).is_none());
}

#[test]
fn is_transport_only_update_flags_status_only_envelopes() {
    assert!(is_transport_only_update(
        &serde_json::json!({"status": "running"})
    ));
    assert!(!is_transport_only_update(
        &serde_json::json!({"status": "running", "output": "hi"})
    ));
    assert!(!is_transport_only_update(&serde_json::json!({"step": 2})));
}

#[test]
fn clear_event_resets_turn_lifecycle_state() {
    let mut state = FeedState::default();
    state.reduce(super::ScrollbackMsg::TurnStart);
    assert!(state.snapshot().facts.turn_started);
    state.reduce(super::ScrollbackMsg::AssistantStreamStart);
    assert!(state.snapshot().facts.assistant_stream_open);
    state.reduce(super::ScrollbackMsg::Clear);
    assert!(!state.snapshot().facts.turn_started);
    assert!(!state.snapshot().facts.assistant_stream_open);
}

#[test]
fn assistant_stream_lifecycle_is_reducer_owned() {
    let mut state = FeedState::default();
    state.reduce(super::ScrollbackMsg::AssistantStreamStart);
    assert!(state.snapshot().facts.assistant_stream_open);
    state.reduce(super::ScrollbackMsg::AssistantStreamEnd);
    assert!(!state.snapshot().facts.assistant_stream_open);
}

#[test]
fn mouse_selection_normalizes_reversed_cells_and_commits_through_events() {
    let mut state = FeedState::default();
    state.reduce(super::ScrollbackMsg::MouseSelectionStart(
        super::CellPosition {
            row: 10,
            column: 18,
        },
    ));
    state.reduce(super::ScrollbackMsg::MouseSelectionExtend(
        super::CellPosition { row: 8, column: 4 },
    ));
    let selection = state.snapshot().cell_selection.expect("selection");
    assert_eq!(
        selection.normalized(),
        (
            super::CellPosition { row: 8, column: 4 },
            super::CellPosition {
                row: 10,
                column: 18
            }
        )
    );
    state.reduce(super::ScrollbackMsg::MouseSelectionCommit);
    assert!(state.snapshot().cell_selection.is_some());
    state.reduce(super::ScrollbackMsg::RequestCopySelection);
    assert!(state.snapshot().copy_selection.is_some());
    state.reduce(super::ScrollbackMsg::ClearCopyRequest);
    assert!(state.snapshot().copy_selection.is_none());
    state.reduce(super::ScrollbackMsg::ClearCellSelection);
    assert!(state.snapshot().cell_selection.is_none());
}

#[test]
fn selected_cell_text_projects_normalized_rows_without_clipboard_side_effects() {
    let lines = vec![
        super::Line::new(super::LineKind::User, "hello"),
        super::Line::new(super::LineKind::Assistant, "世界"),
    ];
    let selection = super::CellSelection {
        anchor: super::CellPosition { row: 1, column: 1 },
        head: super::CellPosition { row: 0, column: 3 },
    };

    assert_eq!(super::selected_cell_text(&lines, selection), "lo\n世");
    assert_eq!(
        super::selected_cell_text(
            &lines,
            super::CellSelection {
                anchor: super::CellPosition { row: 1, column: 0 },
                head: super::CellPosition { row: 1, column: 4 },
            },
        ),
        "世界"
    );
    assert_eq!(
        super::selected_cell_text(
            &lines,
            super::CellSelection {
                anchor: super::CellPosition { row: 9, column: 0 },
                head: super::CellPosition { row: 9, column: 1 },
            },
        ),
        ""
    );
}

#[test]
fn last_assistant_text_only_projects_the_latest_assistant_block() {
    let lines = vec![
        super::Line::new(super::LineKind::Assistant, "older"),
        super::Line::new(super::LineKind::User, "next"),
        super::Line::new(super::LineKind::Assistant, "latest"),
        super::Line::new(super::LineKind::Assistant, "answer"),
    ];
    assert_eq!(super::last_assistant_text(&lines), "latest\nanswer");
    assert_eq!(super::last_assistant_text(&[]), "");
}

#[test]
fn default_tool_modes_match_grok_families() {
    assert_eq!(
        default_tool_display_mode("bash"),
        ToolDisplayMode::Truncated
    );
    assert_eq!(
        default_tool_display_mode("read"),
        ToolDisplayMode::Collapsed
    );
    assert_eq!(
        default_tool_display_mode("memory_search"),
        ToolDisplayMode::Collapsed
    );
}

#[test]
fn format_elapsed_emits_empty_when_missing() {
    assert_eq!(super::format_elapsed(None), String::new());
    assert!(super::format_elapsed(None).is_empty());
}

#[test]
fn format_elapsed_renders_seconds_for_some_value() {
    assert_eq!(super::format_elapsed(Some(1_500)), " in 1.5s");
    assert_eq!(super::format_elapsed(Some(0)), " in 0.0s");
}

#[test]
fn format_error_with_error_flag_and_no_message_yields_empty() {
    assert_eq!(super::format_error(true, None), String::new());
    assert!(super::format_error(true, None).is_empty());
}

#[test]
fn format_error_with_error_flag_and_message_renders_parenthesised_text() {
    assert_eq!(super::format_error(true, Some("boom")), " (boom)");
}

#[test]
fn format_error_suppresses_suffix_when_not_error() {
    assert_eq!(super::format_error(false, None), String::new());
    assert_eq!(super::format_error(false, Some("ignored")), String::new());
}

#[test]
fn thinking_summary_pins_default_and_observed_elapsed() {
    // Pin the fallback path: when no reasoning elapsed is observed, the
    // summary still renders the pinned default rather than an empty or
    // missing label, so replay and live paths share one identity.
    assert_eq!(
        super::thinking_summary(None),
        format!(
            "◆ Thought for {:.1}s",
            super::DEFAULT_THINKING_ELAPSED_MS as f64 / 1_000.0
        )
    );
    // Pin the observed path: an explicit elapsed value overrides the
    // default and renders the same "◆ Thought for …" shape.
    assert_eq!(super::thinking_summary(Some(2_500)), "◆ Thought for 2.5s");
}

#[test]
fn running_bullet_pins_grok_frame_vocabulary_and_wraps() {
    // Pin the four source-backed Grok frames in order; the renderer
    // depends on the exact glyphs and trailing space.
    assert_eq!(super::RUNNING_BULLETS, ["⋅ ", ": ", "⸬ ", "⁙ "]);
    // Pin the frame projection: index 0..4 yields the vocabulary in
    // order, and index 4 wraps back to the first frame.
    assert_eq!(super::running_bullet(0), "⋅ ");
    assert_eq!(super::running_bullet(1), ": ");
    assert_eq!(super::running_bullet(2), "⸬ ");
    assert_eq!(super::running_bullet(3), "⁙ ");
    assert_eq!(super::running_bullet(4), "⋅ ");
    // Pin the wrap-around for a large frame index so the actor-owned
    // animation frame never panics on overflow.
    assert_eq!(super::running_bullet(usize::MAX), "⁙ ");
}

#[test]
fn is_fence_detects_three_backtick_marker_with_or_without_grok_prefix() {
    // Pin the smoke path: a plain triple-backtick opening fence is
    // detected regardless of the renderer prefix.
    assert!(super::is_fence("```rust"));
    assert!(super::is_fence("```"));
    // Pin the Grok-prefix path: the renderer prefix must not hide the
    // fence marker so the actor-owned markdown classifier agrees.
    assert!(super::is_fence("┃ ```rust"));
    // Pin the negative paths: blank lines, single backticks, and prose
    // must not be misclassified as a code fence.
    assert!(!super::is_fence(""));
    assert!(!super::is_fence("`inline`"));
    assert!(!super::is_fence("hello world"));
}

#[test]
fn is_table_row_requires_leading_trailing_pipe_and_two_separators() {
    // Pin the smoke path: a header row with three pipes is detected.
    assert!(super::is_table_row("| a | b | c |"));
    // Pin the body path: a row with surrounding whitespace still counts.
    assert!(super::is_table_row("  | x | y |  "));
    // Pin the single-cell row: a row with two pipes (start/end) is also
    // a table row, matching the existing renderer predicate.
    assert!(super::is_table_row("| single cell |"));
    // Pin the negative paths: an opening pipe only, a trailing pipe
    // only, and prose must not be misclassified as a table row.
    assert!(!super::is_table_row("| only opening"));
    assert!(!super::is_table_row("only trailing |"));
    assert!(!super::is_table_row("no pipes here"));
}

#[test]
fn is_table_separator_accepts_only_dash_colon_and_whitespace_cells() {
    // Pin the smoke path: a Markdown table separator is detected.
    assert!(super::is_table_separator("| --- | :---: | ---: |"));
    assert!(super::is_table_separator("|---|---|"));
    // Pin the negative paths: cells with prose or non-alignment glyphs
    // must not be misclassified as a separator.
    assert!(!super::is_table_separator("| a | b | c |"));
    assert!(!super::is_table_separator("| — | — |")); // em-dash not allowed
    assert!(!super::is_table_separator(""));
}

#[test]
fn atx_heading_returns_title_only_within_commonmark_levels() {
    // Pin the smoke path: a level-1 heading returns the title body.
    assert_eq!(super::atx_heading("# Title"), Some("Title"));
    // Pin the level range: levels 1..=6 are accepted, 0 and 7+ are not.
    assert_eq!(super::atx_heading("###### Title"), Some("Title"));
    assert_eq!(super::atx_heading("####### Title"), None);
    assert_eq!(super::atx_heading("Title"), None);
    // Pin the missing-space edge case: a hash run without a space is not
    // a heading under the CommonMark spec.
    assert_eq!(super::atx_heading("#Title"), None);
    // Pin the empty-title edge case: a heading mark with no body still
    // returns an empty title rather than `None`.
    assert_eq!(super::atx_heading("# "), Some(""));
}

#[test]
fn table_bottom_border_aligns_with_separator_widths() {
    // Pin the smoke path: a three single-char header drives three
    // 3-char border segments (cell width + 2 padding) joined with `┴`.
    assert_eq!(super::table_bottom_border("| a | b | c |"), "└───┴───┴───┘");
    // Pin the wide-cell path: a four-cell header produces four border
    // segments sized to `cell_width + 2` so each column aligns with
    // the header text.
    assert_eq!(
        super::table_bottom_border("| a | bb | ccc | dddd |"),
        "└───┴────┴─────┴──────┘"
    );
    // Pin the noise-tolerance path: surrounding whitespace is trimmed
    // and does not change the border shape.
    assert_eq!(super::table_bottom_border("  | x | y |  "), "└───┴───┘");
}

#[test]
fn append_wrapped_splits_long_lines_at_width_boundary() {
    let mut rows = Vec::new();
    super::append_wrapped(
        &mut rows,
        super::LineKind::Assistant,
        "hello".into(),
        true,
        10,
    );
    assert_eq!(
        rows,
        vec![(super::LineKind::Assistant, "hello".to_owned(), true)]
    );
    rows.clear();
    super::append_wrapped(
        &mut rows,
        super::LineKind::Assistant,
        "abcdefghij".into(),
        false,
        3,
    );
    assert_eq!(
        rows,
        vec![
            (super::LineKind::Assistant, "abc".to_owned(), false),
            (super::LineKind::Assistant, "def".to_owned(), false),
            (super::LineKind::Assistant, "ghi".to_owned(), false),
            (super::LineKind::Assistant, "j".to_owned(), false),
        ]
    );
    rows.clear();
    super::append_wrapped(&mut rows, super::LineKind::User, "x".into(), false, 0);
    assert_eq!(rows, vec![(super::LineKind::User, "x".to_owned(), false)]);
}

#[test]
fn append_wrapped_words_breaks_on_whitespace_and_preserves_indent() {
    // Pin the word-break path: a long phrase wraps at the most recent
    // whitespace without breaking a word in half.
    let mut rows = Vec::new();
    super::append_wrapped_words(
        &mut rows,
        super::LineKind::Assistant,
        "the quick brown fox jumps over the lazy dog".into(),
        10,
    );
    let projected: Vec<&str> = rows.iter().map(|(_, text, _)| text.as_str()).collect();
    assert_eq!(
        projected,
        vec!["the quick", "brown fox", "jumps over", "the lazy", "dog"]
    );
    // Pin the leading-indent path: a leading whitespace run is
    // preserved across the wrap so the projected widget keeps its
    // original indentation.
    rows.clear();
    super::append_wrapped_words(
        &mut rows,
        super::LineKind::User,
        "    indented prompt".into(),
        8,
    );
    let projected: Vec<&str> = rows.iter().map(|(_, text, _)| text.as_str()).collect();
    assert_eq!(projected, vec!["    indented", "    prompt"]);
}

#[test]
fn version_badge_pins_three_grok_welcome_variants() {
    // Pin the full variant: the long `v{version} · Beta` label that
    // the wide hero footer renders right-aligned.
    let full = super::version_badge(super::VersionBadgeVariant::Full);
    assert!(full.starts_with("runie v"), "{full}");
    assert!(full.ends_with(" · Beta"), "{full}");
    // Pin the hero-footer variant: the same version appears in the
    // `Beta · v{version}` order for the right-aligned wide hero.
    let footer = super::version_badge(super::VersionBadgeVariant::HeroFooter);
    assert!(footer.starts_with("runie Beta · v"), "{footer}");
    // Pin the inline variant: the compact `v{version}` form used in
    // compact widgets.
    let inline = super::version_badge(super::VersionBadgeVariant::HeroInline);
    assert!(inline.starts_with("runie v"), "{inline}");
    assert!(!inline.contains("Beta"), "{inline}");
}

#[test]
fn is_quit_command_pins_grok_vocab_with_trim_and_lowercase() {
    // Pin the smoke path: the three Grok quit commands are detected.
    assert!(super::is_quit_command("exit"));
    assert!(super::is_quit_command("quit"));
    assert!(super::is_quit_command(":q"));
    // Pin the normalization path: leading/trailing whitespace and
    // mixed-case input are accepted as quit commands.
    assert!(super::is_quit_command("  QUIT  "));
    assert!(super::is_quit_command("Exit"));
    assert!(super::is_quit_command(":Q"));
}

#[test]
fn is_quit_command_rejects_non_quit_inputs() {
    // Pin the negative paths: prose, partial matches, and empty input
    // are not quit commands so the router treats them as regular text.
    assert!(!super::is_quit_command(""));
    assert!(!super::is_quit_command("hello"));
    assert!(!super::is_quit_command("exiting"));
    assert!(!super::is_quit_command("quitting"));
    assert!(!super::is_quit_command(":quit"));
}

#[test]
fn welcome_modal_lines_pins_idle_chrome_shape() {
    // Pin the smoke path: the modal emits exactly six `LineKind::System`
    // rows so the actor-owned welcome payload and the renderer agree
    // on the chrome line count.
    let lines = super::welcome_modal_lines();
    assert_eq!(lines.len(), 6);
    for line in &lines {
        assert_eq!(line.kind, super::LineKind::System);
    }
    // Pin the chrome shape: the surrounding `╭─` and `╰─` glyphs mark
    // the modal borders, `◆ session_start` closes the modal, and the
    // middle rows carry the model/help breadcrumb.
    let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    assert!(texts[0].starts_with("╭─ Runie  v"), "{}", texts[0]);
    assert_eq!(texts[1], "│ main runie");
    assert_eq!(texts[2], "│ Model · runie-core");
    assert_eq!(texts[3], "│ /help for commands");
    assert_eq!(texts[4], "╰─");
    assert_eq!(texts[5], "◆ session_start");
}
