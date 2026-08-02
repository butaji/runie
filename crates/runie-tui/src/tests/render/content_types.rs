//! Feed content-type coverage.
//!
//! Every `Element` variant must have a render path and must agree with the
//! core layout calculator. This is deliberately a matrix test: it catches a
//! newly-added feed type being silently routed to the empty fallback.

use runie_core::layout;
use runie_core::model::PatternWorkerStatus;
use runie_core::view::elements::{DiffType, Element, ImageProtocol, WebSearchResult};

fn feed_elements() -> Vec<Element> {
    let now = std::time::Instant::now();
    vec![
        Element::Spacer { timestamp: 0.0 },
        Element::UserMessage {
            content: "hello".into(),
            expanded: true,
            timestamp: 0.0,
        },
        Element::AgentMessage {
            content: "answer".into(),
            timestamp: 0.0,
            provider: "echo".into(),
        },
        Element::SystemMessage {
            content: "system event".into(),
            timestamp: 0.0,
        },
        Element::ContextInfo {
            model: "echo".into(),
            used_tokens: 36_700,
            total_tokens: 1_000_000,
            turns: 5,
            tool_calls: 12,
            timestamp: 0.0,
        },
        Element::Thinking { started: now, timestamp: 0.0 },
        Element::ThoughtMarker {
            content: "reasoning".into(),
            timestamp: 0.0,
        },
        Element::ThoughtSummary {
            content: "Thought".into(),
            duration_secs: 1.2,
            expandable: false,
            timestamp: 0.0,
        },
        Element::AnthropicThinking {
            content: "encrypted reasoning".into(),
            signature: Some("sig".into()),
            redacted: false,
            timestamp: 0.0,
        },
        Element::ToolRunning {
            name: "list_files".into(),
            args: "src".into(),
            started: now,
            timestamp: 0.0,
        },
        Element::ToolDone {
            name: "list_files".into(),
            args: "src".into(),
            duration_secs: 1.0,
            output: "main.rs".into(),
            bytes_transferred: Some(7),
            error: false,
            finished_at: None,
            timestamp: 0.0,
        },
        Element::ToolSummary {
            name: "list_files".into(),
            duration_secs: 1.0,
            timestamp: 0.0,
        },
        Element::ToolConfirmation {
            request_id: "req-1".into(),
            name: "write_file".into(),
            args: "file.txt".into(),
            description: "write a file".into(),
            timestamp: 0.0,
        },
        Element::ContextGroup {
            tools: vec![Element::ToolDone {
                name: "read".into(),
                args: "file.txt".into(),
                duration_secs: 0.1,
                output: "content".into(),
                bytes_transferred: None,
                error: false,
                finished_at: None,
                timestamp: 0.0,
            }],
            collapsed: false,
            timestamp: 0.0,
        },
        Element::SubagentRow {
            id: "worker-1".into(),
            description: "find callers".into(),
            model: "echo".into(),
            status: PatternWorkerStatus::Running,
            started: Some(now),
            duration_ms: None,
            activity: "Waiting".into(),
            output: "".into(),
            expanded: false,
            timestamp: 0.0,
        },
        Element::TurnComplete { duration_secs: 2.0, timestamp: 0.0 },
        Element::Image {
            data: "aW1hZ2U=".into(),
            mime_type: "image/png".into(),
            width_cells: Some(8),
            height_cells: Some(4),
            protocol: ImageProtocol::ITerm2,
            timestamp: 0.0,
        },
        Element::DataPart {
            data: "{\"ok\":true}".into(),
            format_string: Some("json".into()),
            timestamp: 0.0,
        },
        Element::MarkdownTable {
            headers: vec!["Name".into()],
            rows: vec![vec!["Runie".into()]],
            alignments: vec![None],
            timestamp: 0.0,
        },
        Element::DiffOutput {
            content: "-old\n+new".into(),
            diff_type: DiffType::Unified,
            timestamp: 0.0,
        },
        Element::WebSearchCall {
            query: "runie".into(),
            results: vec![WebSearchResult {
                title: "Runie".into(),
                url: "https://example.test".into(),
                snippet: "A result".into(),
            }],
            timestamp: 0.0,
        },
        Element::CreditLimit {
            heading: "You've hit your credit limit.".into(),
            action: "purchase_credits".into(),
            url: "https://grok.com/usage".into(),
            timestamp: 0.0,
        },
        Element::Workflow {
            name: "research".into(),
            objective: "compare sources".into(),
            status: "running".into(),
            phases: vec!["done:Plan".into(), "active:Research".into(), "pending:Write".into()],
            active_agents: 2,
            duration_secs: 0.0,
            timestamp: 0.0,
        },
        Element::BackgroundTask {
            command: "cargo test".into(),
            task_id: "task-1".into(),
            status: "completed".into(),
            description: Some("run tests".into()),
            duration_secs: 1.2,
            exit_code: Some(0),
            signal: None,
            timestamp: 0.0,
        },
        Element::Btw {
            question: "What changed?".into(),
            answer: Some("The feed model was updated.".into()),
            status: "answered".into(),
            expanded: false,
            timestamp: 0.0,
        },
        Element::AnsiStyled {
            raw_content: "\x1b[31mred\x1b[0m".into(),
            plain_text: "red".into(),
            timestamp: 0.0,
        },
    ]
}

#[test]
fn every_feed_element_has_rendered_content_and_matching_layout_count() {
    for (index, element) in feed_elements().iter().enumerate() {
        let rendered = crate::ui::to_lines_internal(element, 80);
        assert!(!rendered.is_empty(), "feed element {index} rendered no lines: {element:?}");
        assert_eq!(
            rendered.len(),
            layout::element_line_count(element, 80),
            "feed element {index} disagrees with core line accounting: {element:?}"
        );
    }
}

#[test]
fn every_subagent_lifecycle_state_renders_a_header() {
    let now = std::time::Instant::now();
    for status in [
        PatternWorkerStatus::Running,
        PatternWorkerStatus::Completed,
        PatternWorkerStatus::Failed,
        PatternWorkerStatus::Cancelled,
    ] {
        let element = Element::SubagentRow {
            id: "worker".into(),
            description: "inspect".into(),
            model: "echo".into(),
            status,
            started: Some(now),
            duration_ms: Some(1000),
            activity: "working".into(),
            output: "body".into(),
            expanded: true,
            timestamp: 0.0,
        };
        let text = crate::ui::to_lines_internal(&element, 80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Subagent"), "missing subagent header for {status:?}: {text}");
    }
}

#[test]
fn system_feed_message_is_muted_and_has_no_assistant_glyph() {
    let lines = crate::message::render_system_message("Context compacted", 80);
    let text = lines.into_iter().map(|line| line.to_string()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("Context compacted"));
    assert!(!text.contains('◆'), "system feed content must not use assistant glyph: {text}");
    assert!(!text.contains('❯'), "system feed content must not use user glyph: {text}");
}

#[test]
fn turn_completion_uses_grok_duration_boundaries() {
    let short = crate::message::render_turn_complete(2.5)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>().join("\n");
    let long = crate::message::render_turn_complete(24.5)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>().join("\n");
    assert_eq!(short, "Worked for 2.5s.");
    assert_eq!(long, "Worked for 24s.");
}

#[test]
fn credit_limit_feed_card_has_warning_copy_and_link() {
    let element = Element::credit_limit(
        "You've hit your credit limit.",
        "purchase_credits",
        "https://grok.com/usage",
    )
    .at(1.0);
    let text = crate::ui::to_lines_internal(&element, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("credit limit"), "missing warning heading: {text}");
    assert!(text.contains("purchasing more credits"), "missing action copy: {text}");
    assert!(text.contains("grok.com/usage"), "missing usage link: {text}");
}

#[test]
fn context_info_feed_snapshot_shows_usage_and_counts() {
    let element = Element::context_info("echo", 36_700, 1_000_000, 5, 12).at(1.0);
    let rendered = crate::ui::to_lines_internal(&element, 100);
    let text = rendered
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Context"), "missing context heading: {text}");
    assert!(text.contains("36.7k / 1.0m tokens"), "missing usage: {text}");
    assert!(text.contains("Turns: 5 · Tool calls: 12"), "missing counts: {text}");
    assert!(text.contains("Auto-compact at 85%"), "missing compaction line: {text}");
    let bar = crate::ui::to_lines_internal(&element, 100)[5..10]
        .iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(bar.matches('◆').count() + bar.matches('◇').count(), 100, "context bar must have 100 cells: {text}");
    assert_eq!(text.lines().count(), 17, "wide context snapshot must reserve five bar rows and Grok separators: {text}");

    let narrow = crate::ui::to_lines_internal(&element, 40)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    assert_eq!(narrow.len(), 22, "narrow context snapshot must reserve ten bar rows and Grok separators");
    assert_eq!(narrow[5..15].iter().map(|line| line.matches('◆').count() + line.matches('◇').count()).sum::<usize>(), 100);
}

#[test]
fn context_info_uses_grok_spacing_and_two_decimal_usage() {
    let element = Element::context_info("echo", 36_700, 1_000_000, 5, 12).at(0.0);
    let lines = crate::ui::to_lines_internal(&element, 100);
    let text = lines.iter().map(|line| line.to_string()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("36.7k / 1.0m tokens (3.67%)"), "{text}");
    assert!(text.contains("◆ Used"), "{text}");
    assert!(text.contains("◇ Free"), "{text}");
    assert!(text.contains("Context\n\n"), "{text}");
}

#[test]
fn workflow_feed_row_shows_phase_trail_and_active_agents() {
    let element = Element::workflow(
        "research",
        "compare sources",
        "running",
        vec!["done:Plan".into(), "active:Research".into()],
        2,
    )
    .at(1.0);
    let text = crate::ui::to_lines_internal(&element, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Workflow research: compare sources"), "missing workflow row: {text}");
    assert!(text.contains("Plan ✓ · Research ●"), "missing phase trail: {text}");
    assert!(text.contains("(2 agents)"), "missing active-agent count: {text}");
}

#[test]
fn workflow_terminal_rows_include_elapsed_time() {
    for (status, marker) in [
        ("done", "research done in 2.5s:"),
        ("failed", "research failed in 2.5s:"),
        ("cancelled", "research ◌ cancelled after 2.5s:"),
        ("paused", "research paused at 2.5s:"),
    ] {
        let element = Element::workflow("research", "compare sources", status, Vec::new(), 0).at(1.0);
        let element = match element {
            Element::Workflow { name, objective, status, phases, active_agents, timestamp, .. } => {
                Element::Workflow { name, objective, status, phases, active_agents, duration_secs: 2.5, timestamp }
            }
            _ => unreachable!(),
        };
        let text = crate::ui::to_lines_internal(&element, 80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains(marker), "wrong workflow terminal wording for {status}: {text}");
    }
}

#[test]
fn background_task_feed_row_covers_grok_lifecycle_variants() {
    let cases = [
        ("started", None, "Task started: run tests"),
        ("completed", Some(0), "Task completed in 1.2s: run tests"),
        ("failed", Some(2), "Task failed in 1.2s: run tests (exit 2)"),
        ("killed", None, "Task killed in 1.2s: run tests"),
        ("cancelled", None, "Task killed in 1.2s: run tests"),
    ];
    for (status, exit_code, expected) in cases {
        let element = Element::background_task(
            "cargo test",
            "task-1",
            status,
            Some("run tests".into()),
            1.2,
            exit_code,
            None,
        )
        .at(1.0);
        let text = crate::ui::to_lines_internal(&element, 80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains(expected), "wrong background-task wording for {status}: {text}");
    }

    for signal in ["killed", "SIGTERM", "SIGKILL", "oom"] {
        let element = Element::background_task(
            "cargo test",
            "task-1",
            "failed",
            Some("run tests".into()),
            1.2,
            Some(137),
            Some(signal.into()),
        )
        .at(1.0);
        let text = crate::ui::to_lines_internal(&element, 80)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Task killed in 1.2s: run tests"), "kill signal {signal} must use killed wording: {text}");
        assert!(!text.contains(&format!(" ({signal})")), "kill signal detail leaked into Grok killed row for {signal}: {text}");
    }

    let long = Element::background_task("cargo test", "task-1", "completed", None, 24.0, None, None).at(1.0);
    let long_text = crate::ui::to_lines_internal(&long, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(long_text.contains("Task completed in 24s: cargo test"), "Grok omits decimals at 10s+: {long_text}");
}

#[test]
fn btw_feed_item_shows_question_and_answer() {
    let element = Element::btw(
        "What changed?",
        Some("The feed model was updated.".into()),
        "answered",
    )
    .at(1.0);
    let text = crate::ui::to_lines_internal(&element, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("/btw What changed?"), "missing BTW question: {text}");
    assert!(!text.contains("The feed model was updated."), "collapsed BTW leaked answer: {text}");

    let expanded = match element {
        Element::Btw { question, answer, status, timestamp, .. } => Element::Btw {
            question,
            answer,
            status,
            expanded: true,
            timestamp,
        },
        _ => unreachable!(),
    };
    let expanded_text = crate::ui::to_lines_internal(&expanded, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(expanded_text.contains("The feed model was updated."), "expanded BTW lost answer: {expanded_text}");
}

#[test]
fn btw_running_and_empty_answer_follow_grok_collapsed_rules() {
    let running = Element::btw("Is this live?", None, "running").at(1.0);
    let running_text = crate::ui::to_lines_internal(&running, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(running_text.contains("/btw… Is this live?"), "missing running BTW marker: {running_text}");

    let empty_answer = Element::btw("No answer yet", Some(String::new()), "answered").at(1.0);
    let expanded = match empty_answer {
        Element::Btw { question, answer, status, timestamp, .. } => Element::Btw {
            question,
            answer,
            status,
            expanded: true,
            timestamp,
        },
        _ => unreachable!(),
    };
    let expanded_text = crate::ui::to_lines_internal(&expanded, 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(expanded_text.lines().count(), 1, "empty BTW answers must not add a blank body: {expanded_text:?}");
}

#[test]
fn expanded_btw_preserves_multiline_answer_rows() {
    let element = Element::btw(
        "Explain the change",
        Some("First point\nSecond point\n\nFinal note".into()),
        "answered",
    )
    .at(1.0);
    let expanded = match element {
        Element::Btw { question, answer, status, timestamp, .. } => Element::Btw {
            question,
            answer,
            status,
            expanded: true,
            timestamp,
        },
        _ => unreachable!(),
    };

    let lines = crate::ui::to_lines_internal(&expanded, 80);
    let text = lines.iter().map(|line| line.to_string()).collect::<Vec<_>>();
    let header_index = text.iter().position(|line| line.contains("/btw Explain the change")).unwrap();
    assert_eq!(text.get(header_index + 1).map(String::as_str), Some(""), "BTW body needs Grok's separator row: {text:?}");
    assert!(text.iter().any(|line| line == "  First point"), "missing first answer row: {text:?}");
    assert!(text.iter().any(|line| line == "  Second point"), "missing second answer row: {text:?}");
    assert!(text.iter().any(|line| line == "  Final note"), "missing final answer row: {text:?}");
}

#[test]
fn expanded_btw_layout_count_matches_separator_and_body_rows() {
    let element = Element::Btw {
        question: "Explain".into(),
        answer: Some("one\ntwo\nthree".into()),
        status: "answered".into(),
        expanded: true,
        timestamp: 1.0,
    };
    let rendered = crate::ui::to_lines_internal(&element, 80).len();
    assert_eq!(runie_core::layout::element_line_count(&element, 80), rendered);
}

#[test]
fn recap_system_event_uses_grok_feed_header_and_preview() {
    let lines = crate::message::render_system_message("Recap — Investigated the feed parity gaps.\nDetails follow.", 80);
    let text = lines.iter().map(|line| line.to_string()).collect::<Vec<_>>();
    assert_eq!(text, vec!["Recap  Investigated the feed parity gaps."], "wrong Grok recap row: {text:?}");
}

#[test]
fn expanded_recap_keeps_body_for_global_feed_toggle() {
    let lines = crate::message::render_system_message(
        "Recap +— Investigated the feed parity gaps.\nDetails follow in the recap body.",
        80,
    );
    let text = lines.iter().map(|line| line.to_string()).collect::<Vec<_>>().join("\n");
    assert!(text.starts_with("Recap  Investigated the feed parity gaps."), "{text}");
    assert!(text.contains("Details follow in the recap body."), "{text}");
}

#[test]
fn expanded_btw_renders_markdown_without_source_markers() {
    let element = Element::Btw {
        question: "Explain".into(),
        answer: Some("A **bold** answer with *emphasis*.".into()),
        status: "answered".into(),
        expanded: true,
        timestamp: 1.0,
    };
    let lines = crate::ui::to_lines_internal(&element, 80);
    let body = lines.last().expect("BTW body row");
    let text = body.to_string();
    assert_eq!(text, "  A bold answer with emphasis.", "BTW markdown markers leaked: {text:?}");
    assert!(body.spans.iter().any(|span| span.content == "bold"), "bold span missing: {body:?}");
}
