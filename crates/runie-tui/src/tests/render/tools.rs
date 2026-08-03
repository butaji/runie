//! Layer 3 rendering tests for inline tool rendering.

use ratatui::{backend::TestBackend, widgets::Paragraph, Terminal};

use crate::message::{render_tool_done, render_tool_running, render_tool_summary};

/// Helper to render a paragraph widget and return the buffer content as string.
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

// ─── render_tool_running ────────────────────────────────────────────────────

#[test]
fn render_tool_running_uses_stable_header_text() {
    let lines = render_tool_running("ls", ".", 1.5, 0);
    let output = render_to_string(lines, 80, 3);
    // Grok owns the braille spinner in the live turn-status row, not here.
    assert!(
        output.contains("Run ls") && !output.contains("⠋") && !output.contains("⠙"),
        "Tool feed content must not contain a duplicate spinner: {}",
        output
    );
}

#[test]
fn render_tool_running_shows_label() {
    let lines = render_tool_running("ls", ".", 1.5, 0);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("Run ls"),
        "Output should contain 'Run ls': {}",
        output
    );
}

#[test]
fn render_tool_running_shows_args() {
    let lines = render_tool_running("bash", "echo hello", 0.5, 0);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("echo hello"),
        "Output should contain args: {}",
        output
    );
}

#[test]
fn render_tool_running_does_not_append_duration_to_grok_header() {
    let lines = render_tool_running("ls", ".", 12.5, 0);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("Run ls") && !output.contains("12.5s"),
        "Grok live tool headers do not append elapsed duration: {}",
        output
    );
}

#[test]
fn render_builtin_tool_uses_grok_action_label() {
    let lines = render_tool_running("read_file", "src/main.rs", 0.5, 0);
    let output = render_to_string(lines, 80, 2);
    assert!(
        output.contains("Read src/main.rs"),
        "read tool label: {output}"
    );
    assert!(
        !output.contains("Run read_file"),
        "raw protocol name must not replace action label: {output}"
    );
}

#[test]
fn completed_list_tool_shows_entry_count() {
    let lines = render_tool_done(
        "list_dir",
        ".",
        0.5,
        "one.txt\ntwo.txt",
        None,
        false,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 2);
    assert!(
        output.contains("List . (2 entries)"),
        "list summary: {output}"
    );
}

#[test]
fn completed_read_tool_renders_line_number_gutter() {
    let output = render_to_string(
        render_tool_done(
            "read_file",
            "src/main.rs",
            0.5,
            "fn main() {}\nlet x = 1;",
            None,
            false,
            &None,
            0,
        ),
        80,
        5,
    );
    assert!(
        output.contains("   1 │ fn main() {}"),
        "read gutter: {output}"
    );
    assert!(
        output.contains("   2 │ let x = 1;"),
        "read second line gutter: {output}"
    );
}

#[test]
fn completed_empty_read_has_explicit_empty_suffix() {
    let output = render_to_string(
        render_tool_done("read_file", "empty.txt", 0.5, "", None, false, &None, 0),
        80,
        3,
    );
    assert!(
        output.contains("Read empty.txt (empty)"),
        "empty read header: {output}"
    );
}

#[test]
fn completed_search_shows_match_count() {
    let output = render_to_string(
        render_tool_done(
            "grep",
            "TODO",
            0.5,
            "src/a.rs:1\nsrc/b.rs:9",
            None,
            false,
            &None,
            0,
        ),
        80,
        4,
    );
    assert!(
        output.contains("Search TODO (2 matches)"),
        "search summary: {output}"
    );
}

#[test]
fn completed_edit_shows_diffstat_in_header() {
    let diff = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,2 @@\n-old line\n+new line";
    let output = render_to_string(
        render_tool_done("edit", "src/lib.rs", 0.5, diff, None, false, &None, 0),
        80,
        8,
    );
    assert!(
        output.contains("Edit src/lib.rs +1/-1"),
        "edit diffstat: {output}"
    );
}

#[test]
fn write_tool_uses_creating_header() {
    let output = render_to_string(render_tool_running("write", "src/new.rs", 0.0, 0), 80, 2);
    assert!(
        output.contains("Creating src/new.rs"),
        "write header: {output}"
    );
    assert!(
        !output.contains("Run write"),
        "write must not use generic label: {output}"
    );
}

#[test]
fn skill_and_use_tool_have_named_headers() {
    let skill = render_to_string(render_tool_running("skill", "search", 0.0, 0), 80, 2);
    let use_tool = render_to_string(render_tool_running("use_tool", "browser", 0.0, 0), 80, 2);
    assert!(skill.contains("Skill search"), "skill header: {skill}");
    assert!(
        use_tool.contains("Use Tool browser"),
        "use_tool header: {use_tool}"
    );
    assert!(!skill.contains("Run skill"));
    assert!(!use_tool.contains("Run use_tool"));
}

#[test]
fn completed_web_fetch_uses_ten_line_content_box() {
    let body = (1..=12)
        .map(|n| format!("article-line-{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let output = render_to_string(
        render_tool_done(
            "web_fetch",
            "https://example.test",
            0.5,
            &body,
            None,
            false,
            &None,
            0,
        ),
        100,
        16,
    );
    assert!(output.contains("article-line-1"));
    assert!(output.contains("article-line-10"));
    assert!(!output.contains("article-line-11"));
    assert!(
        output.contains("… +2 more lines"),
        "fetch overflow hint: {output}"
    );
}

#[test]
fn failed_tool_uses_error_theme_for_header_and_output() {
    let lines = render_tool_done(
        "bash",
        "exit 1",
        0.5,
        "permission denied",
        None,
        true,
        &None,
        0,
    );
    let header = &lines[0];
    assert_eq!(header.spans[0].style.fg, Some(crate::theme::color_error()));
    let output = lines
        .iter()
        .find(|line| line.to_string().contains("permission denied"))
        .expect("error output row");
    let error_span = output
        .spans
        .iter()
        .find(|span| span.content.contains("permission denied"))
        .expect("error text span");
    assert_eq!(error_span.style.fg, Some(crate::theme::color_error()));
}

#[test]
fn failed_tool_without_output_has_explicit_failure_row() {
    let lines = render_tool_done("bash", "false", 0.5, "", None, true, &None, 0);
    let failure = lines
        .iter()
        .find(|line| line.to_string().contains("Tool failed"))
        .expect("empty failed tool should render a failure row");
    assert_eq!(failure.spans[0].style.fg, Some(crate::theme::color_error()));
}

#[test]
fn completed_list_tool_uses_singular_entry_count() {
    let output = render_to_string(
        render_tool_done("list_dir", ".", 0.5, "one.txt", None, false, &None, 0),
        80,
        2,
    );
    assert!(
        output.contains("List . (1 entry)"),
        "list summary: {output}"
    );
    assert!(
        !output.contains("1 entries"),
        "singular count must not be plural: {output}"
    );
}

#[test]
fn completed_empty_list_tool_has_no_zero_count_suffix() {
    let output = render_to_string(
        render_tool_done("list_dir", ".", 0.5, "", None, false, &None, 0),
        80,
        2,
    );
    assert!(output.contains("List ."), "list summary: {output}");
    assert!(
        !output.contains("(0 entry"),
        "empty listing must omit count: {output}"
    );
}

#[test]
fn render_unknown_tool_keeps_run_label() {
    let lines = render_tool_running("custom_tool", "arg", 0.5, 0);
    let output = render_to_string(lines, 80, 2);
    assert!(
        output.contains("Run custom_tool"),
        "unknown tool label: {output}"
    );
}

#[test]
fn render_search_tool_variants_use_grok_action_labels() {
    let web = render_to_string(render_tool_running("web_search", "rust", 0.5, 0), 80, 2);
    let memory = render_to_string(render_tool_running("memory_search", "rust", 0.5, 0), 80, 2);
    assert!(web.contains("Web Search rust"), "web search label: {web}");
    assert!(
        memory.contains("Memory Search rust"),
        "memory search label: {memory}"
    );
}

// ─── render_tool_done ───────────────────────────────────────────────────────

#[test]
fn render_tool_done_content_excludes_feed_diamond() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false, &None, 0);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("◆"),
        "feed diamond belongs to the compositor: {output}"
    );
    assert!(
        !output.contains("✓"),
        "Output should not contain the old checkmark: {}",
        output
    );
}

#[test]
fn render_tool_done_shows_label() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false, &None, 0);
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("Run ls"),
        "Output should contain 'Run ls': {}",
        output
    );
}

#[test]
fn render_tool_done_formats_markdown_output() {
    let lines = render_tool_done(
        "bash",
        "",
        0.0,
        "## **Files**\n- `Cargo.toml`",
        None,
        false,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("Files"),
        "heading text should remain: {output}"
    );
    assert!(
        output.contains("Cargo.toml"),
        "inline code text should remain: {output}"
    );
    assert!(
        !output.contains("**"),
        "bold markers should be rendered: {output}"
    );
    assert!(
        !output.contains('`'),
        "code markers should be rendered: {output}"
    );
}

#[test]
fn render_directory_listing_normalizes_section_labels() {
    let lines = render_tool_done(
        "list_dir",
        ".",
        0.0,
        "Config & Project Files:**\n.cargo/",
        None,
        false,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("Config & Project Files:"),
        "section label: {output}"
    );
    assert!(!output.contains("**"), "pseudo-markdown markers: {output}");
    assert!(output.contains("• .cargo/"), "directory bullet: {output}");
}

#[test]
fn render_tool_output_does_not_leak_malformed_markdown_markers() {
    let lines = render_tool_done(
        "list_dir",
        "",
        0.0,
        "Config:**\n• `Cargo.toml`\n**broken",
        None,
        false,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 8);
    assert!(!output.contains("**"), "emphasis markers leaked: {output}");
    assert!(!output.contains('`'), "code markers leaked: {output}");
    assert!(output.contains("Config:"));
    assert!(output.contains("Cargo.toml"));
}

#[test]
fn render_tool_done_shows_bytes() {
    let lines = render_tool_done(
        "bash",
        "echo hello",
        1.0,
        "hello",
        Some(5_000_000),
        false,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("⇣") && output.contains("5.0M"),
        "Output should contain bytes indicator: {}",
        output
    );
}

#[test]
fn render_tool_done_content_excludes_feed_error_icon() {
    let lines = render_tool_done("bash", "exit 1", 0.5, "error", None, true, &None, 0);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("✗"),
        "feed error icon belongs to the compositor: {output}"
    );
}

#[test]
fn render_tool_done_hides_duration() {
    let lines = render_tool_done("ls", ".", 5.7, "file1\nfile2", None, false, &None, 0);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("5.7s"),
        "Done tool post should not render a duration (grok parity): {}",
        output
    );
}

#[test]
fn render_tool_done_no_bytes_when_none() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false, &None, 0);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("⇣"),
        "Output should not contain bytes when None: {}",
        output
    );
}

#[test]
fn render_tool_done_content_preserves_error_text() {
    let lines = render_tool_done(
        "bash",
        "exit 1",
        0.5,
        "command not found",
        None,
        true,
        &None,
        0,
    );
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("command not found"),
        "Output should contain error text: {}",
        output
    );
}

// ─── render_tool_summary ────────────────────────────────────────────────────

#[test]
fn render_tool_summary_is_one_line() {
    let lines = render_tool_summary("ls", ".", 2.5);
    // Tool summary should be a single logical content line (with margins added by caller)
    // The key is that there is only ONE content line, not multiple output lines
    assert!(!lines.is_empty(), "Should return at least one line");
    // Check that the content contains the expected text (first line)
    let output = lines[0].to_string();
    assert!(
        output.contains("◆") && output.contains("Run ls"),
        "Should show summary: {}",
        output
    );
}

#[test]
fn render_tool_summary_shows_diamond() {
    let lines = render_tool_summary("ls", ".", 2.5);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("◆"),
        "Output should contain the tool diamond: {}",
        output
    );
    assert!(
        !output.contains("✓"),
        "Output should not contain the old checkmark: {}",
        output
    );
}

#[test]
fn render_tool_summary_shows_label() {
    let lines = render_tool_summary("ls", ".", 2.5);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("Run ls"),
        "Output should contain 'Run ls': {}",
        output
    );
}

#[test]
fn render_tool_summary_hides_duration() {
    let lines = render_tool_summary("ls", ".", 65.0); // > 60s
    let output = render_to_string(lines, 80, 3);
    assert!(
        !output.contains("1m") && !output.contains("65"),
        "Tool summary should not render a duration (grok parity): {}",
        output
    );
}
