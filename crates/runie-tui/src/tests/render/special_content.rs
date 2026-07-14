//! Layer 3 rendering tests for special content types:
//! images, tables, diffs, thinking blocks, web search, ANSI styling,
//! data parts, and tool confirmations.

use ratatui::{backend::TestBackend, widgets::Paragraph, Terminal};

use crate::message::{
    render_ansi_styled, render_anthropic_thinking, render_data_part, render_diff_output,
    render_image, render_list_item_from_spans, render_markdown_table, render_tool_confirmation,
    render_web_search_call,
};
use runie_core::view::elements::{DiffType, ImageProtocol, WebSearchResult};

// ─── Test helpers ──────────────────────────────────────────────────────────────

fn render_to_string(lines: Vec<ratatui::text::Line<'static>>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    let buf = terminal.backend().buffer();
    (0..height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buf.area().width)
        .map(|x| buf[(x, y)].symbol())
        .collect()
}

// ─── 1. Images ─────────────────────────────────────────────────────────────────

mod images {
    use super::*;

    #[test]
    fn image_shows_mime_type() {
        let lines = render_image(
            "base64data",
            "image/png",
            None,
            None,
            ImageProtocol::ITerm2,
            0.0,
        );
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("image/png"),
            "Should show mime type: {}",
            output
        );
    }

    #[test]
    fn image_shows_protocol() {
        let lines = render_image(
            "base64data",
            "image/png",
            None,
            None,
            ImageProtocol::Kitty,
            0.0,
        );
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("Kitty"),
            "Should show protocol: {}",
            output
        );
    }

    #[test]
    fn image_shows_dimensions() {
        let lines = render_image(
            "base64data",
            "image/jpeg",
            Some(40),
            Some(20),
            ImageProtocol::Sixel,
            0.0,
        );
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("40x20"),
            "Should show dimensions: {}",
            output
        );
    }

    #[test]
    fn image_shows_auto_when_no_dimensions() {
        let lines = render_image(
            "base64data",
            "image/gif",
            None,
            None,
            ImageProtocol::ITerm2,
            0.0,
        );
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("auto"),
            "Should show 'auto' when no dimensions: {}",
            output
        );
    }

    #[test]
    fn image_shows_placeholder() {
        let lines = render_image(
            "base64data",
            "image/webp",
            Some(80),
            None,
            ImageProtocol::Kitty,
            0.0,
        );
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("[Image:"),
            "Should contain image header: {}",
            output
        );
        assert!(
            output.contains("80 cells wide"),
            "Should show width: {}",
            output
        );
    }
}

// ─── 2. Redacted/Encrypted Thinking ───────────────────────────────────────────

mod anthropic_thinking {
    use super::*;

    #[test]
    fn thinking_shows_header() {
        let lines = render_anthropic_thinking("Let me think about this...", None, false, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("[Thinking]"),
            "Should show thinking header: {}",
            output
        );
    }

    #[test]
    fn thinking_shows_content() {
        let lines = render_anthropic_thinking("Step 1: analyze\nStep 2: solve", None, false, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("Step 1"),
            "Should show thinking content: {}",
            output
        );
        assert!(
            output.contains("Step 2"),
            "Should show thinking content: {}",
            output
        );
    }

    #[test]
    fn thinking_shows_signature() {
        let sig = "abc123def456xyz789".to_string();
        let lines = render_anthropic_thinking("content", Some(sig), false, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("sig:"),
            "Should show signature: {}",
            output
        );
        assert!(
            output.contains("xyz789"),
            "Should show signature tail: {}",
            output
        );
    }

    #[test]
    fn redacted_hides_content() {
        let lines = render_anthropic_thinking(
            "This should not appear",
            None,
            true,
            0.0,
        );
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("[Redacted Thinking]"),
            "Should show redacted header: {}",
            output
        );
        assert!(
            output.contains("[encrypted content]"),
            "Should show encrypted placeholder: {}",
            output
        );
        assert!(
            !output.contains("should not appear"),
            "Should NOT show actual content: {}",
            output
        );
    }

    #[test]
    fn redacted_with_signature() {
        let sig = "secret123".to_string();
        let lines = render_anthropic_thinking("hidden", Some(sig), true, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("[Redacted Thinking]"),
            "Should show redacted header: {}",
            output
        );
        assert!(
            output.contains("sig:"),
            "Should still show signature: {}",
            output
        );
    }
}

// ─── 3. Structured Data Parts ─────────────────────────────────────────────────

mod data_parts {
    use super::*;

    #[test]
    fn data_part_shows_label() {
        let lines = render_data_part(r#"{"key": "value"}"#, Some("json"), 0.0);
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("[json]"),
            "Should show format label: {}",
            output
        );
    }

    #[test]
    fn data_part_shows_data() {
        let lines = render_data_part(r#"{"name": "test"}"#, None, 0.0);
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("name"),
            "Should show data content: {}",
            output
        );
        assert!(
            output.contains("test"),
            "Should show data content: {}",
            output
        );
    }

    #[test]
    fn data_part_truncates_long_data() {
        let long_data = "x".repeat(300);
        let lines = render_data_part(&long_data, Some("text"), 0.0);
        let output = render_to_string(lines, 80, 5);
        // Data content may be shown with data label and truncated
        assert!(
            output.contains("[text]") || output.contains("[data]"),
            "Should show format label: {}",
            output
        );
    }

    #[test]
    fn data_part_defaults_to_data_label() {
        let lines = render_data_part("raw content", None, 0.0);
        let output = render_to_string(lines, 80, 5);
        assert!(
            output.contains("[data]"),
            "Should default to 'data' label: {}",
            output
        );
    }
}

// ─── 4. Markdown Tables ───────────────────────────────────────────────────────

mod markdown_tables {
    use super::*;

    #[test]
    fn table_shows_headers() {
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![vec!["Alice".to_string(), "30".to_string()]];
        let lines = render_markdown_table(&headers, &rows, &[None, None], 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("Name"),
            "Should show header: {}",
            output
        );
        assert!(
            output.contains("Age"),
            "Should show header: {}",
            output
        );
    }

    #[test]
    fn table_shows_rows() {
        let headers = vec!["Fruit".to_string()];
        let rows = vec![
            vec!["Apple".to_string()],
            vec!["Banana".to_string()],
        ];
        let lines = render_markdown_table(&headers, &rows, &[None], 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("Apple"),
            "Should show Apple: {}",
            output
        );
        assert!(
            output.contains("Banana"),
            "Should show Banana: {}",
            output
        );
    }

    #[test]
    fn table_shows_separator() {
        let headers = vec!["A".to_string()];
        let rows = vec![vec!["B".to_string()]];
        let lines = render_markdown_table(&headers, &rows, &[None], 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("─"),
            "Should show column separator: {}",
            output
        );
    }

    #[test]
    fn table_empty_returns_nothing() {
        let headers: Vec<String> = vec![];
        let rows: Vec<Vec<String>> = vec![];
        let lines = render_markdown_table(&headers, &rows, &[], 0.0);
        assert!(
            lines.is_empty(),
            "Empty table should return no lines"
        );
    }

    #[test]
    fn table_aligns_right() {
        let headers = vec!["Num".to_string()];
        let rows = vec![vec!["42".to_string()]];
        // true = right align
        let lines = render_markdown_table(&headers, &rows, &[Some(true)], 0.0);
        let output = render_to_string(lines, 80, 10);
        // Right-aligned content should have trailing space padding
        assert!(
            output.contains("42"),
            "Should show number: {}",
            output
        );
    }
}

// ─── 5. Diff/Changelist Output ─────────────────────────────────────────────────

mod diff_output {
    use super::*;

    #[test]
    fn diff_shows_header() {
        let content = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n old line\n+new line\n";
        let lines = render_diff_output(content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("[Diff:"),
            "Should show diff header: {}",
            output
        );
        assert!(
            output.contains("unified"),
            "Should show diff type: {}",
            output
        );
    }

    #[test]
    fn diff_shows_additions_in_green() {
        let content = "+ This line was added";
        let lines = render_diff_output(content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("+"),
            "Should show addition marker: {}",
            output
        );
    }

    #[test]
    fn diff_shows_removals() {
        let content = "- This line was removed";
        let lines = render_diff_output(content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("-"),
            "Should show removal marker: {}",
            output
        );
    }

    #[test]
    fn diff_shows_hunks() {
        let content = "@@ -1,3 +1,4 @@ context";
        let lines = render_diff_output(content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("@@"),
            "Should show hunk header: {}",
            output
        );
    }

    #[test]
    fn diff_shows_file_headers() {
        let content = "--- a/src/main.rs\n+++ b/src/main.rs\n";
        let lines = render_diff_output(content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("---"),
            "Should show file header: {}",
            output
        );
        assert!(
            output.contains("+++"),
            "Should show file header: {}",
            output
        );
    }

    #[test]
    fn diff_side_by_side_type() {
        let content = "unchanged";
        let lines = render_diff_output(content, DiffType::SideBySide, 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("side-by-side"),
            "Should show side-by-side type: {}",
            output
        );
    }

    #[test]
    fn diff_truncates_long_output() {
        let lines_vec: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let content = lines_vec.join("\n");
        let lines = render_diff_output(&content, DiffType::Unified, 0.0);
        let output = render_to_string(lines, 80, 60);
        assert!(
            output.contains("...") || output.contains("truncated"),
            "Should truncate long diff: {}",
            output
        );
    }
}

// ─── 6. Web Search Calls ───────────────────────────────────────────────────────

mod web_search {
    use super::*;

    #[test]
    fn search_shows_query() {
        let results = vec![WebSearchResult {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            snippet: "Test snippet".to_string(),
        }];
        let lines = render_web_search_call("rust programming", &results, 0.0);
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("rust programming"),
            "Should show query: {}",
            output
        );
    }

    #[test]
    fn search_shows_results() {
        let results = vec![WebSearchResult {
            title: "Rust Docs".to_string(),
            url: "https://rust-lang.org".to_string(),
            snippet: "The Rust programming language".to_string(),
        }];
        let lines = render_web_search_call("rust", &results, 0.0);
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("Rust Docs"),
            "Should show title: {}",
            output
        );
        assert!(
            output.contains("rust-lang.org"),
            "Should show URL: {}",
            output
        );
        assert!(
            output.contains("The Rust programming language"),
            "Should show snippet: {}",
            output
        );
    }

    #[test]
    fn search_shows_result_number() {
        let results = vec![WebSearchResult {
            title: "Result 1".to_string(),
            url: "https://ex.com".to_string(),
            snippet: "".to_string(),
        }];
        let lines = render_web_search_call("query", &results, 0.0);
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("1."),
            "Should number results: {}",
            output
        );
    }

    #[test]
    fn search_empty_shows_searching() {
        let results: Vec<WebSearchResult> = vec![];
        let lines = render_web_search_call("query", &results, 0.0);
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("Searching..."),
            "Should show searching placeholder: {}",
            output
        );
    }

    #[test]
    fn search_limits_to_five_results() {
        let results: Vec<WebSearchResult> = (0..10)
            .map(|i| WebSearchResult {
                title: format!("Result {}", i),
                url: format!("https://example{}.com", i),
                snippet: format!("Snippet {}", i),
            })
            .collect();
        let lines = render_web_search_call("query", &results, 0.0);
        let output = render_to_string(lines, 80, 80);
        assert!(
            output.contains("Result 0"),
            "Should show first result: {}",
            output
        );
        assert!(
            !output.contains("Result 9"),
            "Should not show 10th result (max 5): {}",
            output
        );
    }
}

// ─── 7. ANSI Styled Content ────────────────────────────────────────────────────

mod ansi_styled {
    use super::*;

    #[test]
    fn ansi_shows_header() {
        let lines = render_ansi_styled("\x1B[31mred text\x1B[0m", "red text", 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("[ANSI Styled]"),
            "Should show ANSI header: {}",
            output
        );
    }

    #[test]
    fn ansi_parses_bold() {
        let lines = render_ansi_styled("\x1B[1mbold text\x1B[0m", "bold text", 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("bold text"),
            "Should show content: {}",
            output
        );
    }

    #[test]
    fn ansi_parses_color() {
        let lines = render_ansi_styled("\x1B[32mgreen\x1B[0m", "green", 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("green"),
            "Should show colored text: {}",
            output
        );
    }

    #[test]
    fn ansi_parses_underline() {
        let lines = render_ansi_styled("\x1B[4munderlined\x1B[0m", "underlined", 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("underlined"),
            "Should show underlined text: {}",
            output
        );
    }

    #[test]
    fn ansi_multiline() {
        let content = "\x1B[31mline1\x1B[0m\n\x1B[32mline2\x1B[0m";
        let lines = render_ansi_styled(content, "line1\nline2", 0.0);
        let output = render_to_string(lines, 80, 10);
        assert!(
            output.contains("line1"),
            "Should show first line: {}",
            output
        );
        assert!(
            output.contains("line2"),
            "Should show second line: {}",
            output
        );
    }

    #[test]
    fn ansi_truncates_long_output() {
        let long_content: String = (0..30)
            .map(|i| format!("\x1B[{}mline{}\x1B[0m", i % 7 + 30, i))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = render_ansi_styled(&long_content, "", 0.0);
        let output = render_to_string(lines, 80, 40);
        assert!(
            output.contains("...") || output.contains("truncated"),
            "Should truncate: {}",
            output
        );
    }
}

// ─── 8. Tool Confirmation Requests ────────────────────────────────────────────

mod tool_confirmation {
    use super::*;

    #[test]
    fn confirmation_shows_header() {
        let lines = render_tool_confirmation(
            "req-123",
            "bash",
            "rm -rf /",
            "Delete everything",
            0.0,
        );
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("[CONFIRM]"),
            "Should show confirm header: {}",
            output
        );
        assert!(
            output.contains("Tool call requires approval"),
            "Should explain need for approval: {}",
            output
        );
    }

    #[test]
    fn confirmation_shows_tool_name() {
        let lines = render_tool_confirmation(
            "req-456",
            "read_file",
            "/path/to/file",
            "Read a file",
            0.0,
        );
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("read_file"),
            "Should show tool name: {}",
            output
        );
    }

    #[test]
    fn confirmation_shows_description() {
        let lines = render_tool_confirmation(
            "req-789",
            "bash",
            "ls -la",
            "List directory contents",
            0.0,
        );
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("List directory contents"),
            "Should show description: {}",
            output
        );
    }

    #[test]
    fn confirmation_shows_request_id() {
        let lines = render_tool_confirmation(
            "abc-123-xyz",
            "test",
            "",
            "",
            0.0,
        );
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("Request ID:"),
            "Should show request ID label: {}",
            output
        );
        assert!(
            output.contains("abc-123-xyz"),
            "Should show request ID: {}",
            output
        );
    }

    #[test]
    fn confirmation_shows_hint() {
        let lines = render_tool_confirmation(
            "req-1",
            "test",
            "args",
            "desc",
            0.0,
        );
        let output = render_to_string(lines, 80, 20);
        assert!(
            output.contains("y"),
            "Should show 'y' hint: {}",
            output
        );
        assert!(
            output.contains("n"),
            "Should show 'n' hint: {}",
            output
        );
        assert!(
            output.contains("confirm"),
            "Should show confirm hint: {}",
            output
        );
        assert!(
            output.contains("deny"),
            "Should show deny hint: {}",
            output
        );
    }

    #[test]
    fn confirmation_handles_empty_description() {
        let lines = render_tool_confirmation("req-1", "tool", "args", "", 0.0);
        let output = render_to_string(lines, 80, 20);
        // Should not crash and should still show tool name
        assert!(
            output.contains("tool"),
            "Should show tool name even without description: {}",
            output
        );
    }

    #[test]
    fn confirmation_handles_multiline_args() {
        let args = "line1\nline2\nline3\nline4\nline5\nline6";
        let lines = render_tool_confirmation("req-1", "tool", args, "", 0.0);
        let output = render_to_string(lines, 80, 30);
        assert!(
            output.contains("line1"),
            "Should show first arg line: {}",
            output
        );
        // Lines after 5 should be truncated
        assert!(
            !output.contains("line6"),
            "Should truncate args after 5 lines: {}",
            output
        );
    }
}

// ─── 9. List Item Helper ──────────────────────────────────────────────────────

mod list_item {
    use super::*;
    use crate::markdown_render::MdSpan;
    use ratatui::style::Style;

    #[test]
    fn list_item_bullet() {
        let row = vec![MdSpan {
            content: "First item".to_string(),
            style: Style::default(),
        }];
        let line = render_list_item_from_spans(&row, false, 0, true, "", "12:00", 5, 70);
        let text = line.to_string();
        assert!(
            text.contains("First item"),
            "Should contain item text: {}",
            text
        );
    }

    #[test]
    fn list_item_ordered_number() {
        let row = vec![MdSpan {
            content: "Third".to_string(),
            style: Style::default(),
        }];
        let line = render_list_item_from_spans(&row, true, 2, true, "", "12:00", 5, 70);
        let text = line.to_string();
        assert!(
            text.contains("3."),
            "Should show ordered number: {}",
            text
        );
    }

    #[test]
    fn list_item_with_prefix() {
        let row = vec![MdSpan {
            content: "Item".to_string(),
            style: Style::default(),
        }];
        let line = render_list_item_from_spans(&row, false, 0, true, "A", "12:00", 5, 70);
        let text = line.to_string();
        assert!(
            text.contains("A"),
            "Should show prefix: {}",
            text
        );
    }
}
