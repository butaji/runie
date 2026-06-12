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
mod thinking;
mod tool;
mod tool_call;
mod user;
mod wrap;

pub use assistant::{render_assistant_msg, extract_think_blocks, strip_think_tags};
pub use messages::{
    render_thought_msg,
    render_separator,
    item_separator_height,
    render_system_msg,
    render_error_msg,
    render_edit_msg,
    render_tool_running_msg,
    render_tool_complete_msg,
    render_plan_step_msg,
    render_interrupt_msg,
    render_rewind_msg,
    render_empty_state,
    render_session_starting,
};
pub use markdown::{highlight_code_block_ratatui, render_text_content};
pub use tool::{render_tool_call_msg, format_tool_args_compact};
pub use user::render_user_msg;
pub use wrap::{WrapCache, wrap_text, wrap_text_preserving_newlines};
// Re-export thinking block types for external testing
pub use thinking::{ThinkingBlock, render_thinking_block, render_thought_indicator};
// Re-export tool call block types for external testing
pub use tool_call::{ToolCallBlock, ToolStatus, render_tool_call_block, render_tool_call_inline_compact};

use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use super::types::{MessageItem, PlanStatus};
use super::feed::FeedItem;
use super::MessageColors;

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

/// Determine if cursor should be shown for a FeedItem (Feed-based rendering)
pub fn should_show_cursor_feed(
    animation: &AnimationState,
    agent_running: bool,
    absolute_idx: usize,
    total_items: usize,
    item: &FeedItem,
) -> bool {
    animation.streaming_cursor_visible
        && agent_running
        && absolute_idx == total_items.saturating_sub(1)
        && matches!(item, FeedItem::AssistantMessage { .. })
}

/// Find the index of the most recent message that needs a spinner.
fn find_most_recent_spinner_index(messages: &[MessageItem]) -> Option<usize> {
    messages.iter().enumerate().rev().find(|(_, msg)| {
        matches!(msg,
            MessageItem::ToolRunning { .. }
            | MessageItem::PlanStep { status: PlanStatus::Active, .. }
            | MessageItem::Rewind { .. }
        )
    }).map(|(i, _)| i)
}

/// Get the type identifier for a message
fn get_msg_type(msg: &MessageItem) -> &'static str {
    // User/Assistant core messages
    match msg {
        MessageItem::User { .. } => "user",
        MessageItem::Assistant { .. } => "assistant",
        _ => msg_type_else(msg),
    }
}

fn msg_type_else(msg: &MessageItem) -> &'static str {
    // Thought/separator
    match msg {
        MessageItem::Thought { .. } => "thought",
        MessageItem::Separator { .. } => "separator",
        _ => msg_type_tool(msg),
    }
}

fn msg_type_tool(msg: &MessageItem) -> &'static str {
    // Tool-related
    match msg {
        MessageItem::ToolCall { .. } => "tool",
        MessageItem::ToolRunning { .. } => "tool_running",
        MessageItem::ToolComplete { .. } => "tool_complete",
        _ => msg_type_edit_plan(msg),
    }
}

fn msg_type_edit_plan(msg: &MessageItem) -> &'static str {
    // Edit/plan
    match msg {
        MessageItem::Edit { .. } => "edit",
        MessageItem::PlanStep { .. } => "plan_step",
        _ => msg_type_sys_err(msg),
    }
}

fn msg_type_sys_err(msg: &MessageItem) -> &'static str {
    // System/error
    match msg {
        MessageItem::System { .. } => "system",
        MessageItem::Error { .. } => "error",
        _ => msg_type_misc(msg),
    }
}

fn msg_type_misc(msg: &MessageItem) -> &'static str {
    // Interrupt/rewind
    match msg {
        MessageItem::Interrupt { .. } => "interrupt",
        MessageItem::Rewind { .. } => "rewind",
        _ => "unknown",
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
    _accent_primary: ratatui::style::Color,
    accent_secondary: ratatui::style::Color,
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
    thought_duration: Option<f32>,
    turn_complete: Option<f32>,
    feed_tool_bar: ratatui::style::Color,
    streaming_thinking_elapsed_ms: Option<u64>,
    streaming_total_elapsed_ms: Option<u64>,
    streaming_download_bytes: Option<u64>,
) -> u16 {
    match msg {
        MessageItem::User { text, timestamp, .. } => {
            render_user_msg(text, timestamp.as_deref(), area, row, margin_x, text_x, max_rows, buf, theme, wrap_cache)
        }
        MessageItem::Assistant { text, timestamp, .. } => {
            render_assistant_msg(text, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, accent_secondary, cursor_visible, wrap_cache, agent_running, spinner, timestamp.as_deref(), thought_duration, turn_complete, true, &[], accent_secondary, false, streaming_thinking_elapsed_ms, streaming_total_elapsed_ms, streaming_download_bytes, None)
        }
        MessageItem::Thought { duration_secs, .. } => {
            render_thought_msg(*duration_secs, area, row, margin_x, text_x, buf, text_muted, spinner, show_spinner)
        }
        MessageItem::Separator { elapsed_secs, tool_calls, tokens_used } => {
            render_separator(*elapsed_secs, *tool_calls, *tokens_used, true, area, row, margin_x, buf, text_dim)
        }
        MessageItem::ToolCall { name, args, result, is_error: _ } => {
            let colors = MessageColors {
                accent_primary: _accent_primary,
                accent_secondary,
                accent_tertiary: ratatui::style::Color::Reset,
                text_secondary,
                text_muted,
                text_dim,
                success,
                error,
                code_path,
            };
            render_tool_call_msg(name, args, result.as_deref(), area, buf, &colors, feed_tool_bar, theme)
        }
        MessageItem::Edit { filename, diff } => {
            render_edit_msg(filename, diff.as_deref().unwrap_or(""), area, row, margin_x, text_x, buf, text_secondary, code_path)
        }
        MessageItem::System { text } => {
            render_system_msg(text, area, row, margin_x, text_x, buf, text_muted, error, wrap_cache)
        }
        MessageItem::Error { message, recoverable } => {
            render_error_msg(message, *recoverable, area, row, margin_x, text_x, buf, error, text_muted, wrap_cache)
        }
        MessageItem::ToolRunning { name, args, duration_ms, total_elapsed_ms, download_bytes } => {
            render_tool_running_msg(name, args, *duration_ms, *total_elapsed_ms, *download_bytes, area, row, margin_x, text_x, buf, text_secondary, spinner, show_spinner)
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

/// Render a single FeedItem (Feed-based rendering pipeline)
/// Note: Thoughts and ToolCalls are now inline in AssistantMessage, not separate items.
pub fn render_single_msg_feed(
    item: &FeedItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    _accent_primary: ratatui::style::Color,
    accent_secondary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    _success: ratatui::style::Color,
    error: ratatui::style::Color,
    _code_path: ratatui::style::Color,
    spinner: char,
    cursor_visible: bool,
    _show_spinner: bool,
    _rewind_spinner: char,
    _animation: &AnimationState,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
    thought_duration: Option<f32>,
    turn_complete: Option<f32>,
    is_last_item: bool,
    thoughts_collapsed: bool,
    streaming_think_content: Option<&str>,
) -> u16 {
    match item {
        FeedItem::UserMessage { text, timestamp, .. } => {
            render_user_msg(text, timestamp.as_deref(), area, row, margin_x, text_x, max_rows, buf, theme, wrap_cache)
        }
        FeedItem::AssistantMessage { text, thoughts, tool_calls, timestamp, turn_duration, streaming_thinking_elapsed_ms, streaming_total_elapsed_ms, streaming_download_bytes, .. } => {
            // Use first thought's duration if provided, otherwise use the passed thought_duration
            let effective_thought_duration = thoughts.first().map(|t| t.duration).or(thought_duration);
            // Use turn_duration from FeedItem, or passed turn_complete. Pass
            // as f32 (seconds with 0.1s precision) — `turn_completed`
            // formats it as "{:.1}s" so 3.6s round-trips correctly.
            // The u64 form is only used in the (legacy) non-f32 path.
            #[allow(unused_variables)]
            let _ignored_legacy_u64_path = turn_duration.map(|d| (d * 10.0) as u64);
            let effective_turn_complete = turn_duration.or(turn_complete);
            // Tool bar color is purple #6B50FF (same as feed.tool.bar)
            let tool_bar_color = ratatui::style::Color::Rgb(0x6B, 0x50, 0xFF);
            render_assistant_msg(text, area, row, margin_x, text_x, max_rows, buf, text_secondary, text_muted, accent_secondary, cursor_visible, wrap_cache, agent_running, spinner, timestamp.as_deref(), effective_thought_duration, effective_turn_complete, is_last_item, tool_calls, tool_bar_color, thoughts_collapsed, *streaming_thinking_elapsed_ms, *streaming_total_elapsed_ms, *streaming_download_bytes, streaming_think_content)
        }
        FeedItem::SystemNotice { text } => {
            render_system_msg(text, area, row, margin_x, text_x, buf, text_muted, error, wrap_cache)
        }
        FeedItem::Separator { elapsed_secs, tool_calls, tokens_used } => {
            render_separator(*elapsed_secs, *tool_calls, *tokens_used, true, area, row, margin_x, buf, text_dim)
        }
        FeedItem::ToolRunning { name, args, duration_ms, total_elapsed_ms, download_bytes } => {
            render_tool_running_msg(name, args, *duration_ms, *total_elapsed_ms, *download_bytes, area, row, margin_x, text_x, buf, text_secondary, spinner, true)
        }
        FeedItem::ToolComplete { name, result, lines } => {
            let lines_ref = lines.as_ref();
            render_tool_complete_msg(name, result, lines_ref, area, row, margin_x, text_x, buf, _success, text_muted)
        }
    }
}
