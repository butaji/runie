//! Message rendering module
//!
//! Split into focused submodules:
//! - `assistant` - Assistant message rendering (with think blocks)
//! - `markdown` - Markdown/text rendering and syntax highlighting
//! - `messages` - Various message type renderers
//! - `tool` - Tool call/message rendering
//! - `user` - User message rendering
//! - `wrap` - Text wrapping and WrapCache

mod assistant;
mod markdown;
mod messages;
mod tool;
mod user;
mod wrap;

pub use assistant::{render_assistant_msg, extract_think_blocks, strip_think_tags};
pub use messages::{
    render_thought_msg,
    render_separator,
    render_system_msg,
    render_error_msg,
    render_edit_msg,
    render_tool_running_msg,
    render_tool_complete_msg,
    render_plan_step_msg,
    render_interrupt_msg,
    render_rewind_msg,
    render_empty_state,
};
pub use markdown::{highlight_code_block_ratatui, render_text_content};
pub use tool::{render_tool_call_msg, format_tool_args_compact};
pub use user::render_user_msg;
pub use wrap::{WrapCache, wrap_text, wrap_text_preserving_newlines};

use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use super::types::{MessageItem, PlanStatus};

/// Determine if cursor should be shown for a message
pub fn should_show_cursor(
    animation: &AnimationState,
    agent_running: bool,
    absolute_idx: usize,
    total_messages: usize,
    msg: &MessageItem,
) -> bool {
    animation.streaming_cursor_visible
        && agent_running
        && absolute_idx == total_messages.saturating_sub(1)
        && matches!(msg, MessageItem::Assistant { .. })
}

/// Find the index of the most recent message that needs a spinner.
pub fn find_most_recent_spinner_index(messages: &[MessageItem]) -> Option<usize> {
    messages.iter().enumerate().rev().find(|(_, msg)| {
        matches!(msg,
            MessageItem::ToolRunning { .. }
            | MessageItem::PlanStep { status: PlanStatus::Active, .. }
            | MessageItem::Rewind { .. }
        )
    }).map(|(i, _)| i)
}

/// Get the type identifier for a message
pub fn get_msg_type(msg: &MessageItem) -> &'static str {
    match msg {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        MessageItem::Thought { .. } => "thought",
        MessageItem::Separator { .. } => "separator",
        MessageItem::ToolCall { .. } => "tool",
        MessageItem::Edit { .. } => "edit",
        MessageItem::System { .. } => "system",
        MessageItem::Error { .. } => "error",
        MessageItem::ToolRunning { .. } => "tool_running",
        MessageItem::ToolComplete { .. } => "tool_complete",
        MessageItem::PlanStep { .. } => "plan_step",
        MessageItem::Interrupt { .. } => "interrupt",
        MessageItem::Rewind { .. } => "rewind",
    }
}

/// Render a single message based on its type
pub fn render_single_msg(
    msg: &MessageItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
    spinner: char,
    cursor_visible: bool,
    show_spinner: bool,
    rewind_spinner: char,
    animation: &AnimationState,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
) -> u16 {
    match msg {
        MessageItem::User { text, .. } => {
            render_user_msg(text, area, row, margin_x, text_x, max_rows, buf, theme, accent_primary, wrap_cache)
        }
        MessageItem::Assistant { text, .. } => {
            render_assistant_msg(text, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, cursor_visible, wrap_cache, agent_running, spinner)
        }
        MessageItem::Thought { duration_secs } => {
            render_thought_msg(*duration_secs, area, row, margin_x, text_x, buf, text_muted, spinner, show_spinner)
        }
        MessageItem::Separator { elapsed_secs, tool_calls, tokens_used } => {
            render_separator(*elapsed_secs, *tool_calls, *tokens_used, area, row, margin_x, buf, text_dim)
        }
        MessageItem::ToolCall { name, args, result, is_error } => {
            render_tool_call_msg(name, args, result.as_deref(), *is_error, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, success, error)
        }
        MessageItem::Edit { filename, diff } => {
            render_edit_msg(filename, diff.as_deref().unwrap_or(""), area, row, margin_x, text_x, buf, text_secondary, code_path)
        }
        MessageItem::System { text } => {
            render_system_msg(text, area, row, margin_x, text_x, buf, text_muted, error)
        }
        MessageItem::Error { message, recoverable } => {
            render_error_msg(message, *recoverable, area, row, margin_x, text_x, buf, error, text_muted)
        }
        MessageItem::ToolRunning { name, args, duration_ms } => {
            render_tool_running_msg(name, args, *duration_ms, area, row, margin_x, text_x, buf, text_secondary, spinner, show_spinner)
        }
        MessageItem::ToolComplete { name, result, lines } => {
            render_tool_complete_msg(name, result, lines.as_ref(), area, row, margin_x, text_x, buf, success, text_muted)
        }
        MessageItem::PlanStep { step, text, status } => {
            render_plan_step_msg(*step, text, status, area, row, margin_x, text_x, buf, text_dim, text_secondary, spinner, show_spinner)
        }
        MessageItem::Interrupt { .. } => {
            render_interrupt_msg(area, row, margin_x, text_x, buf, error, text_dim, animation)
        }
        MessageItem::Rewind { steps } => {
            render_rewind_msg(*steps, area, row, margin_x, text_x, buf, text_muted, rewind_spinner, show_spinner)
        }
    }
}
