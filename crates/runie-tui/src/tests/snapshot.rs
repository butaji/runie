//! Snapshot tests for key TUI rendering paths.

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Terminal;

use crate::diff::render_diff_text;
use crate::message::{render_agent_message, render_tool_done, render_user_message};

fn render_lines(lines: Vec<ratatui::text::Line<'static>>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let paragraph = Paragraph::new(lines);
            paragraph.render(Rect::new(0, 0, width, height), f.buffer_mut());
        })
        .unwrap();
    buffer_to_string(terminal.backend().buffer(), width)
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer, width: u16) -> String {
    let height = buffer.area.height;
    let mut rows = Vec::new();
    for y in 0..height {
        let mut row = String::new();
        for x in 0..width {
            row.push_str(buffer[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows.join("\n")
}

#[test]
fn snapshot_chat_message_renders_correctly() {
    let lines = render_user_message("Hello, world!", 1.0, 40);
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_agent_message_renders_correctly() {
    let lines = render_agent_message("Hello from the agent.", 2.0, 40);
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_tool_output_renders_correctly() {
    let lines = render_tool_done("list_files", "", 1.2, "src/main.rs\nlib.rs", None, false);
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_diff_renders_correctly() {
    let diff_text = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
    let lines = render_diff_text(diff_text);
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}
