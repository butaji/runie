use runie_core::view::elements::{Element, DiffType, ImageProtocol, WebSearchResult};

#[test]
fn user_message_builder() {
    let e = Element::user("hello").at(1.0);
    assert!(
        matches!(e, Element::UserMessage { content, timestamp } if content == "hello" && timestamp == 1.0)
    );
}

#[test]
fn agent_message_builder() {
    let e = Element::agent("world").at(2.0);
    assert!(
        matches!(e, Element::AgentMessage { content, timestamp, .. } if content == "world" && timestamp == 2.0)
    );
}

#[test]
fn thought_marker_builder() {
    let e = Element::thought("thinking...").at(3.0);
    assert!(
        matches!(e, Element::ThoughtMarker { content, timestamp } if content == "thinking..." && timestamp == 3.0)
    );
}

#[test]
fn thought_summary_builder() {
    let e = Element::thought_summary("sum", 1.5).at(4.0);
    assert!(
        matches!(e, Element::ThoughtSummary { content, duration_secs, timestamp, expandable }
        if content == "sum" && duration_secs == 1.5 && timestamp == 4.0 && expandable)
    );
}

#[test]
fn tool_running_builder() {
    let started = std::time::Instant::now();
    let e = Element::tool_running("ls", ".", started).at(5.0);
    assert!(
        matches!(e, Element::ToolRunning { name, args, timestamp, .. }
        if name == "ls" && args == "." && timestamp == 5.0)
    );
}

#[test]
fn tool_done_builder() {
    let e = Element::tool_done("ls", ".", 0.5, "file1", None, false).at(6.0);
    assert!(
        matches!(e, Element::ToolDone { name, args, duration_secs, output, bytes_transferred, error, timestamp }
        if name == "ls" && args == "." && duration_secs == 0.5 && output == "file1" && timestamp == 6.0 && bytes_transferred.is_none() && !error)
    );
}

#[test]
fn tool_summary_builder() {
    let e = Element::tool_summary("ls", 0.5).at(7.0);
    assert!(
        matches!(e, Element::ToolSummary { name, duration_secs, timestamp }
        if name == "ls" && duration_secs == 0.5 && timestamp == 7.0)
    );
}

#[test]
fn turn_complete_builder() {
    let e = Element::turn_complete(1.0).at(8.0);
    assert!(
        matches!(e, Element::TurnComplete { duration_secs, timestamp }
        if duration_secs == 1.0 && timestamp == 8.0)
    );
}

#[test]
fn spacer_builder() {
    let e = Element::spacer().at(9.0);
    assert!(matches!(e, Element::Spacer { timestamp } if timestamp == 9.0));
}

#[test]
fn thinking_builder() {
    let started = std::time::Instant::now();
    let e = Element::thinking(started).at(10.0);
    assert!(matches!(e, Element::Thinking { timestamp, .. } if timestamp == 10.0));
}

// ─── New Special Content Type Builders ───────────────────────────────────

#[test]
fn anthropic_thinking_builder() {
    let sig = "abc123".to_string();
    let e = Element::anthropic_thinking("thinking content", Some(sig.clone())).at(11.0);
    assert!(
        matches!(e, Element::AnthropicThinking { content, signature, redacted, timestamp }
        if content == "thinking content" && signature == Some(sig) && !redacted && timestamp == 11.0)
    );
}

#[test]
fn redacted_thinking_builder() {
    let e = Element::redacted_thinking("encrypted...").at(12.0);
    assert!(
        matches!(e, Element::AnthropicThinking { content, signature, redacted, timestamp }
        if content == "encrypted..." && signature.is_none() && redacted && timestamp == 12.0)
    );
}

#[test]
fn tool_confirmation_builder() {
    let e = Element::tool_confirmation("req.1", "delete", "{}", "Delete file").at(13.0);
    assert!(
        matches!(e, Element::ToolConfirmation { request_id, name, args, description, timestamp }
        if request_id == "req.1" && name == "delete" && args == "{}" && description == "Delete file" && timestamp == 13.0)
    );
}

#[test]
fn image_builder() {
    let e = Element::image("base64data...", "image/png").at(14.0);
    assert!(
        matches!(e, Element::Image { data, mime_type, width_cells, height_cells, protocol, timestamp }
        if data == "base64data..." && mime_type == "image/png" && width_cells.is_none() && height_cells.is_none() && protocol == ImageProtocol::ITerm2 && timestamp == 14.0)
    );
}

#[test]
fn data_part_builder() {
    let e = Element::data_part(r#"{"key": "value"}"#, Some("JSON object".to_string())).at(15.0);
    assert!(
        matches!(e, Element::DataPart { data, format_string, timestamp }
        if data == r#"{"key": "value"}"# && format_string == Some("JSON object".to_string()) && timestamp == 15.0)
    );
}

#[test]
fn markdown_table_builder() {
    let headers = vec!["Name".to_string(), "Age".to_string()];
    let rows = vec![
        vec!["Alice".to_string(), "30".to_string()],
        vec!["Bob".to_string(), "25".to_string()],
    ];
    let alignments = vec![None, Some(true)];
    let e = Element::markdown_table(headers.clone(), rows.clone(), alignments.clone()).at(16.0);
    assert!(
        matches!(e, Element::MarkdownTable { headers: h, rows: r, alignments: a, timestamp }
        if h == headers && r == rows && a == alignments && timestamp == 16.0)
    );
}

#[test]
fn diff_output_builder() {
    let e = Element::diff_output("@@ -1,3 +1,4 @@", DiffType::Unified).at(17.0);
    assert!(
        matches!(e, Element::DiffOutput { content, diff_type, timestamp }
        if content == "@@ -1,3 +1,4 @@" && diff_type == DiffType::Unified && timestamp == 17.0)
    );
}

#[test]
fn web_search_call_builder() {
    let results = vec![
        WebSearchResult {
            title: "Result 1".to_string(),
            url: "https://example.com/1".to_string(),
            snippet: "Snippet 1".to_string(),
        },
    ];
    let e = Element::web_search_call("rust programming", results.clone()).at(18.0);
    assert!(
        matches!(e, Element::WebSearchCall { query, results: r, timestamp }
        if query == "rust programming" && r == results && timestamp == 18.0)
    );
}

#[test]
fn ansi_styled_builder() {
    let e = Element::ansi_styled("\x1b[31mred\x1b[0m", "red").at(19.0);
    assert!(
        matches!(e, Element::AnsiStyled { raw_content, plain_text, timestamp }
        if raw_content == "\x1b[31mred\x1b[0m" && plain_text == "red" && timestamp == 19.0)
    );
}
