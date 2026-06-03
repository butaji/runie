//! Snapshot regression tests for Runie TUI components.

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use crate::components::{
    top_bar::{TopBarViewModel, render_top_bar},
    message_list::{MessageListViewModel, MessageItem, MessageList, PlanStatus},
    message_list::render::WrapCache,
    permission_modal::PermissionModal,
    diff_viewer::DiffViewer,
};
use crate::tui::render::{render_status_bar, render_agent_list};
use crate::tui::view_models::{StatusBarViewModel, AgentListViewModel};
use crate::tui::state::TuiMode;
use crate::theme::{ThemeColors, ThemeWrapper};
use runie_ai::TokenUsage;

pub const SIDEBAR_WIDTH: u16 = 28;

pub fn make_test_colors() -> ThemeColors {
    ThemeColors {
        bg_base: Color::Rgb(0x0F, 0x0C, 0x14),
        bg_panel: Color::Rgb(0x20, 0x1F, 0x26),
        accent_primary: Color::Rgb(0xFF, 0x6B, 0x00),
        text_primary: Color::Rgb(0xFF, 0xFA, 0xF1),
        text_secondary: Color::Rgb(0xDF, 0xDB, 0xDD),
        text_dim: Color::Rgb(0x8A, 0x87, 0x94),
        text_muted: Color::Rgb(0xBF, 0xBC, 0xC8),
        border_unfocused: Color::Rgb(0x3A, 0x39, 0x43),
        success: Color::Rgb(0x00, 0xF5, 0xD4),
        warning: Color::Rgb(0xFF, 0x6B, 0x00),
        error: Color::Rgb(0xEB, 0x42, 0x68),
        syntax_phase: Color::Rgb(0x6B, 0x50, 0xFF),
    }
}

pub fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        if y < buf.area.height - 1 {
            result.push('\n');
        }
    }
    result
}

pub fn user_message(text: &str) -> MessageItem {
    MessageItem::User { text: text.to_string(), model: None, timestamp: None }
}

pub fn assistant_message(text: &str) -> MessageItem {
    MessageItem::Assistant { text: text.to_string(), model: Some("gpt-4o".to_string()), timestamp: None, stable_text: text.to_string(), tail_text: String::new(), is_streaming: false }
}

pub fn system_message(text: &str) -> MessageItem {
    MessageItem::System { text: text.to_string() }
}

pub fn error_message(msg: &str, recoverable: bool) -> MessageItem {
    MessageItem::Error { message: msg.to_string(), recoverable }
}

pub fn tool_call(name: &str, args: &str, result: Option<&str>, is_error: bool) -> MessageItem {
    use crate::components::message_list::types::ToolExecutionState;
    let state = if result.is_none() {
        ToolExecutionState::Pending
    } else if is_error {
        ToolExecutionState::Error
    } else {
        ToolExecutionState::Completed { success: true }
    };
    MessageItem::ToolCall { name: name.to_string(), args: args.to_string(), result: result.map(|s| s.to_string()), state }
}

mod top_bar_tests;
mod message_list_tests;
mod status_bar_tests;
mod permission_modal_tests;
mod misc_tests;
mod grok_parity_tests;

pub use top_bar_tests::*;
pub use message_list_tests::*;
pub use status_bar_tests::*;
pub use permission_modal_tests::*;
pub use misc_tests::*;
pub use grok_parity_tests::*;
