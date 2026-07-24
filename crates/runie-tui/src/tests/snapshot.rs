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
    use ratatui::style::Color;
    let height = buffer.area.height;
    let mut rows = Vec::new();
    for y in 0..height {
        let mut row = String::new();
        for x in 0..width {
            let cell = &buffer[(x, y)];
            // Include ANSI foreground color if set (not default/reset).
            let has_color = !matches!(cell.fg, Color::Reset);
            if has_color {
                match cell.fg {
                    Color::Rgb(r, g, b) => {
                        row.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                    }
                    _ => {}
                }
            }
            // Include bold modifier.
            let has_bold = cell.modifier.intersects(ratatui::style::Modifier::BOLD);
            if has_bold {
                row.push_str("\x1b[1m");
            }
            row.push_str(cell.symbol());
            // Reset if we set any style.
            if has_color || has_bold {
                row.push_str("\x1b[0m");
            }
        }
        rows.push(row);
    }
    rows.join("\n")
}

#[test]
fn snapshot_chat_message_renders_correctly() {
    let lines = render_user_message("Hello, world!", 1.0, true, 40);
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
    let lines = render_tool_done(
        "list_files",
        "",
        1.2,
        "src/main.rs\nlib.rs",
        None,
        false,
        &None,
        0,
    );
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_diff_renders_correctly() {
    let diff_text = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@
-old
+new
";
    let lines = render_diff_text(diff_text);
    let output = render_lines(lines, 40, 5);
    insta::assert_snapshot!(output);
}

#[test]
fn test_render_agent_heading_h1_has_teal_color() {
    // H1 heading should have teal foreground color (115, 218, 202).
    let lines = render_agent_message("# Heading One\n\nSome body text.", 1.0, 60);
    let rendered = render_lines(lines, 60, 5);
    eprintln!("=== H1 Rendered ===\n{}", rendered);

    // Extract all unique ANSI color sequences from the rendered output.
    let mut color_seqs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, _) in rendered.match_indices("\x1b[") {
        let s = &rendered[i..];
        if s.starts_with("\x1b[38;") || s.starts_with("\x1b[48;") {
            if let Some(m_pos) = s.find('m') {
                let seq = format!("\x1b[{}", &s[3..m_pos + 1]);
                color_seqs.insert(seq);
            }
        }
    }
    let mut color_seqs: Vec<_> = color_seqs.into_iter().collect();
    color_seqs.sort();
    eprintln!("=== Unique color sequences ===");
    for seq in &color_seqs {
        eprintln!("  {}", seq.escape_debug());
    }

    // Strip ANSI codes to check for plain text content.
    fn strip_ansi(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip until we hit a letter (end of ANSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if c != '\n' {
                result.push(c);
            }
        }
        result
    }
    let plain = strip_ansi(&rendered);
    eprintln!("=== H1 Plain Text ===\n{}", plain);

    // Check for teal color in the rendered output.
    assert!(
        rendered.contains("\x1b[38;2;115;218;202m"),
        "H1 should have teal foreground color, got:\n{}",
        rendered
    );
    // Check that heading marker is NOT in the output (should be stripped).
    assert!(
        !plain.contains("#"),
        "H1 marker should be hidden, got:\n{}",
        plain
    );
    // Check that content is visible (after stripping ANSI codes).
    assert!(
        plain.contains("Heading One"),
        "H1 content should appear, got:\n{}",
        plain
    );
}

#[test]
fn test_render_agent_shows_full_content() {
    use crate::message::render_agent_message;
    let content = "xyzabc123 what is 2+2";
    let lines = render_agent_message(content, 1000.0, 80);
    let full_text: String = lines.iter().map(|l| l.to_string()).collect();
    assert!(
        full_text.contains("xyzabc123"),
        "Full text should appear, got: {}",
        full_text
    );
}

#[test]
fn test_render_user_message_long_content() {
    // This is the exact content from the tmux session that was truncated
    let content = "what is rust async";
    let lines = render_user_message(content, 1000.0, true, 80);
    let full_text: String = lines.iter().map(|l| l.to_string()).collect();
    let rendered = render_lines(lines, 80, 5);
    // Print for debugging
    eprintln!("=== render_user_message('{}') ===", content);
    eprintln!("Full text: {}", full_text);
    eprintln!("Rendered:\n{}", rendered);
    assert!(
        full_text.contains("what is rust async"),
        "Full text should appear, got: {}",
        full_text
    );
}

#[test]
fn test_render_user_message_with_timestamp() {
    let content = "what is rust async";
    let lines = render_user_message(content, 1720761720.0, true, 80); // realistic timestamp
    let full_text: String = lines.iter().map(|l| l.to_string()).collect();
    eprintln!("=== With timestamp ===");
    eprintln!("Full text: {}", full_text);
    let rendered = render_lines(lines, 80, 5);
    eprintln!("Rendered:\n{}", rendered);
    assert!(
        full_text.contains("what is rust async"),
        "Full text should appear, got: {}",
        full_text
    );
}
