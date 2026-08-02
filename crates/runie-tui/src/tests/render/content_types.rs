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
