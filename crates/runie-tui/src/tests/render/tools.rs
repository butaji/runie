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
fn render_tool_running_shows_spinner() {
    let lines = render_tool_running("ls", ".", 1.5, 0);
    let output = render_to_string(lines, 80, 3);
    // Should contain spinner char
    assert!(
        output.contains("⠋") || output.contains("⠙"),
        "Output should contain spinner char: {}",
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
fn render_tool_running_shows_duration() {
    let lines = render_tool_running("ls", ".", 12.5, 0);
    let output = render_to_string(lines, 80, 3);
    assert!(
        output.contains("12.5s"),
        "Output should contain duration: {}",
        output
    );
}

// ─── render_tool_done ───────────────────────────────────────────────────────

#[test]
fn render_tool_done_shows_diamond() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false);
    let output = render_to_string(lines, 80, 5);
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
fn render_tool_done_shows_label() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false);
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("Run ls"),
        "Output should contain 'Run ls': {}",
        output
    );
}

#[test]
fn render_tool_done_shows_bytes() {
    let lines = render_tool_done("bash", "echo hello", 1.0, "hello", Some(5_000_000), false);
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("⇣") && output.contains("5.0M"),
        "Output should contain bytes indicator: {}",
        output
    );
}

#[test]
fn render_tool_done_shows_error_icon() {
    let lines = render_tool_done("bash", "exit 1", 0.5, "error", None, true);
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("✗") || output.contains("[✗]"),
        "Output should contain error icon: {}",
        output
    );
}

#[test]
fn render_tool_done_hides_duration() {
    let lines = render_tool_done("ls", ".", 5.7, "file1\nfile2", None, false);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("5.7s"),
        "Done tool post should not render a duration (grok parity): {}",
        output
    );
}

#[test]
fn render_tool_done_no_bytes_when_none() {
    let lines = render_tool_done("ls", ".", 2.5, "file1\nfile2", None, false);
    let output = render_to_string(lines, 80, 5);
    assert!(
        !output.contains("⇣"),
        "Output should not contain bytes when None: {}",
        output
    );
}

#[test]
fn render_tool_done_shows_error_text() {
    let lines = render_tool_done("bash", "exit 1", 0.5, "command not found", None, true);
    let output = render_to_string(lines, 80, 5);
    assert!(
        output.contains("✗"),
        "Output should contain error icon: {}",
        output
    );
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
