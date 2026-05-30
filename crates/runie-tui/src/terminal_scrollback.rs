//! Terminal infinite scrollback using VT100 sequences.
//!
//! When a turn completes, messages are pushed to the terminal's scrollback buffer
//! using VT100 scroll region manipulation, keeping the viewport state lean.

use crossterm::{
    ExecutableCommand,
    Command,
    style::{Print, ResetColor},
};
use std::io::{self, Write};

use crate::components::MessageItem;

/// VT100 scroll region command to set the scrollable area.
/// Messages pushed above the viewport end up in scrollback.
pub struct SetScrollRegion(pub std::ops::Range<u16>);

impl Command for SetScrollRegion {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "\x1b[{};{}r", self.0.start, self.0.end)
    }
}

/// Reset scroll region to full terminal height.
pub struct ResetScrollRegion;

impl Command for ResetScrollRegion {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "\x1b[r")
    }
}

/// Save cursor position.
pub struct SaveCursor;

impl Command for SaveCursor {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "\x1b[s")
    }
}

/// Restore cursor position.
pub struct RestoreCursor;

impl Command for RestoreCursor {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "\x1b[u")
    }
}

/// Move cursor to absolute position.
pub struct MoveCursor(pub u16, pub u16);

impl Command for MoveCursor {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "\x1b[{};{}H", self.1 + 1, self.0 + 1)
    }
}

/// Format a MessageItem as plain text for scrollback.
pub fn format_message_item(item: &MessageItem) -> String {
    match item {
        MessageItem::User { text, model, .. } => format_user_msg(model, text),
        MessageItem::Assistant { text, model, .. } => format_assistant_msg(model, text),
        MessageItem::Thought { duration_secs } => format_thought_msg(*duration_secs),
        MessageItem::Separator { elapsed_secs, tool_calls, tokens_used } => 
            format_separator_msg(*elapsed_secs, *tool_calls, *tokens_used),
        MessageItem::ToolCall { name, args, result, is_error } => 
            format_toolcall_msg(name, args, result.as_deref(), *is_error),
        MessageItem::Edit { filename, diff, .. } => format_edit_msg(filename, diff.as_deref()),
        MessageItem::System { text } => format_system_msg(text),
        MessageItem::Error { message, .. } => format_error_msg(message),
        MessageItem::ToolRunning { name, args, duration_ms } => 
            format_tool_running_msg(name, args, *duration_ms),
        MessageItem::ToolComplete { name, result, lines } => 
            format_tool_complete_msg(name, result, *lines),
        MessageItem::PlanStep { step, text, status } => 
            format_plan_step_msg(*step, text, status),
        MessageItem::Interrupt => format_interrupt_msg(),
        MessageItem::Rewind { steps } => format_rewind_msg(*steps),
    }
}

fn format_user_msg(model: &Option<String>, text: &str) -> String {
    let model_str = model.as_deref().unwrap_or("user");
    format!("[{}] {}\n", model_str, text)
}

fn format_assistant_msg(model: &Option<String>, text: &str) -> String {
    let model_str = model.as_deref().unwrap_or("assistant");
    format!("[{}] {}\n", model_str, text)
}

fn format_thought_msg(duration_secs: f32) -> String {
    format!("◆ Thought for {:.1}s\n", duration_secs)
}

fn format_separator_msg(elapsed_secs: u64, tool_calls: usize, tokens_used: Option<usize>) -> String {
    let elapsed_str = if elapsed_secs < 60 {
        format!("{}s", elapsed_secs)
    } else if elapsed_secs < 3600 {
        format!("{}m {:02}s", elapsed_secs / 60, elapsed_secs % 60)
    } else {
        format!("{}h {:02}m", elapsed_secs / 3600, (elapsed_secs % 3600) / 60)
    };
    let mut tag = format!("[turn: {}", elapsed_str);
    if tool_calls > 0 {
        tag.push_str(&format!(", {}tc", tool_calls));
    }
    if let Some(tokens) = tokens_used {
        tag.push_str(&format!(", ⇣{}", format_token_count(tokens)));
    }
    tag.push_str("]\n");
    tag
}

fn format_toolcall_msg(name: &str, args: &str, result: Option<&str>, is_error: bool) -> String {
    let result_str = result.unwrap_or("");
    let status = if is_error { "ERROR" } else { "OK" };
    if result_str.is_empty() {
        format!("→ {} {} [{}]\n", name, args, status)
    } else {
        format!("→ {} {} [{}]: {}\n", name, args, status, result_str)
    }
}

fn format_edit_msg(filename: &str, diff: Option<&str>) -> String {
    let diff_str = diff.unwrap_or("");
    format!("◆ Edit {}\n{}\n", filename, diff_str)
}

fn format_system_msg(text: &str) -> String {
    format!("· {}\n", text)
}

fn format_error_msg(message: &str) -> String {
    format!("! {}\n", message)
}

fn format_tool_running_msg(name: &str, args: &str, duration_ms: u64) -> String {
    format!("● {} {} ({}ms)\n", name, args, duration_ms)
}

fn format_tool_complete_msg(name: &str, result: &str, lines: Option<usize>) -> String {
    let lines_str = lines.map(|l| format!(" ({} lines)", l)).unwrap_or_default();
    format!("✓ {} {}{}\n", name, result, lines_str)
}

fn format_plan_step_msg(step: usize, text: &str, status: &crate::components::message_list::PlanStatus) -> String {
    let status_str = match status {
        crate::components::message_list::PlanStatus::Pending => "pending",
        crate::components::message_list::PlanStatus::Active => "active",
        crate::components::message_list::PlanStatus::Complete => "complete",
    };
    format!("▸ {}. {} ({})\n", step, text, status_str)
}

fn format_interrupt_msg() -> String {
    "✗ Interrupted\n".to_string()
}

fn format_rewind_msg(steps: usize) -> String {
    format!("↺ Rewinding ({} steps)\n", steps)
}

fn format_token_count(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Push finalized messages to terminal scrollback using VT100 sequences.
pub fn push_to_scrollback(messages: &[MessageItem], terminal_height: u16) -> io::Result<()> {
    if messages.is_empty() {
        return Ok(());
    }

    let formatted: String = messages.iter().map(format_message_item).collect();
    
    push_scrollback_lines(&formatted, terminal_height)
}

fn push_scrollback_lines(formatted: &str, terminal_height: u16) -> io::Result<()> {
    let mut stdout = io::stdout();

    stdout.execute(SaveCursor)?;
    stdout.execute(MoveCursor(0, 0))?;
    stdout.execute(SetScrollRegion(0..terminal_height))?;
    
    print_fill_lines(&mut stdout, terminal_height)?;
    
    for line in formatted.lines() {
        stdout.execute(Print(line))?;
        stdout.execute(Print("\r\n"))?;
    }

    stdout.execute(ResetScrollRegion)?;
    stdout.execute(RestoreCursor)?;
    stdout.execute(ResetColor)?;
    stdout.flush()?;

    Ok(())
}

fn print_fill_lines(stdout: &mut dyn io::Write, terminal_height: u16) -> io::Result<()> {
    for _ in 0..terminal_height {
        stdout.execute(Print("\n"))?;
    }
    Ok(())
}

/// Push messages to scrollback using current terminal size.
pub fn push_to_scrollback_auto(messages: &[MessageItem]) -> io::Result<bool> {
    let Ok((_cols, rows)) = crossterm::terminal::size() else {
        return Ok(false);
    };

    if rows == 0 {
        return Ok(false);
    }

    push_to_scrollback(messages, rows)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_user_message() {
        let item = MessageItem::User {
            text: "Hello".to_string(),
            model: Some("You".to_string()),
            timestamp: None,
        };
        let formatted = format_message_item(&item);
        assert!(formatted.contains("Hello"));
    }

    #[test]
    fn test_format_assistant_message() {
        let item = MessageItem::Assistant {
            text: "Hi there!".to_string(),
            model: Some("gpt-4".to_string()),
            timestamp: None,
        };
        let formatted = format_message_item(&item);
        assert!(formatted.contains("Hi there!"));
    }

    #[test]
    fn test_format_separator() {
        let item = MessageItem::Separator {
            elapsed_secs: 125,
            tool_calls: 3,
            tokens_used: Some(1500),
        };
        let formatted = format_message_item(&item);
        assert!(formatted.contains("2m 05s"));
        assert!(formatted.contains("3tc"));
    }

    #[test]
    fn test_format_tool_call() {
        let item = MessageItem::ToolCall {
            name: "bash".to_string(),
            args: "ls -la".to_string(),
            result: Some("total 0".to_string()),
            is_error: false,
        };
        let formatted = format_message_item(&item);
        assert!(formatted.contains("bash"));
        assert!(formatted.contains("ls -la"));
    }

    #[test]
    fn test_format_error() {
        let item = MessageItem::Error {
            message: "Something went wrong".to_string(),
            recoverable: true,
        };
        let formatted = format_message_item(&item);
        assert!(formatted.contains("Something went wrong"));
    }
}
