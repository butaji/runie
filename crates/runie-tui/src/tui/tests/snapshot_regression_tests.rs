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

const SIDEBAR_WIDTH: u16 = 28;

fn make_test_colors() -> ThemeColors {
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

fn buffer_to_string(buf: &Buffer) -> String {
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

fn user_message(text: &str) -> MessageItem {
    MessageItem::User { text: text.to_string(), model: None, timestamp: None }
}

fn assistant_message(text: &str) -> MessageItem {
    MessageItem::Assistant { text: text.to_string(), model: Some("gpt-4o".to_string()), timestamp: None, stable_text: text.to_string(), tail_text: String::new(), is_streaming: false }
}

fn system_message(text: &str) -> MessageItem {
    MessageItem::System { text: text.to_string() }
}

fn error_message(msg: &str, recoverable: bool) -> MessageItem {
    MessageItem::Error { message: msg.to_string(), recoverable }
}

fn tool_call(name: &str, args: &str, result: Option<&str>, is_error: bool) -> MessageItem {
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

// ─── Tests ───────────────────────────────────────────────────────────────

#[test]
fn snapshot_main_ui_empty_state() {
    let vm = MessageListViewModel {
        messages: vec![],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_main_ui_empty_state", buffer_to_string(&buf));
}

#[test]
fn snapshot_main_ui_with_messages() {
    let vm = MessageListViewModel {
        messages: vec![
            user_message("Hello AI"),
            assistant_message("Hello! How can I help you today?"),
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_main_ui_with_messages", buffer_to_string(&buf));
}

#[test]
fn snapshot_permission_modal() {
    let modal = PermissionModal::new(
        "bash",
        r#"{"command": "ls -la"}"#,
        "Lists files in the current directory",
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_modal", buffer_to_string(&buf));
}

#[test]
fn snapshot_permission_dangerous_command() {
    let modal = PermissionModal::new(
        "bash",
        r#"rm -rf /"#,
        "⚠ This command will DELETE ALL FILES on your system!",
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_dangerous_command", buffer_to_string(&buf));
}

#[test]
fn snapshot_model_overlay() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Select,
        current_model: Some("openai/gpt-4o".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_model_overlay", buffer_to_string(&buf));
}

#[test]
fn snapshot_status_bar_with_model() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: Some("openai/gpt-4o".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_status_bar_with_model", buffer_to_string(&buf));
}

#[test]
fn snapshot_status_bar_no_model() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: None,
        session_token_usage: TokenUsage::default(),
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 0,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_status_bar_no_model", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_0pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 0,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_0pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_50pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 64_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_50pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_100pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 128_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_100pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_over_100pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 150_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_over_100pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_error_message() {
    let vm = MessageListViewModel {
        messages: vec![error_message("Network timeout after 30 seconds", true)],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_error_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_system_message() {
    let vm = MessageListViewModel {
        messages: vec![system_message("Using model: gpt-4o-mini")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_system_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_tool_result_message() {
    let vm = MessageListViewModel {
        messages: vec![
            tool_call("bash", r#"{"command": "ls"}"#, None, false),
            MessageItem::ToolComplete { name: "bash".to_string(), result: "README.md\nsrc\ntests".to_string(), lines: Some(3) },
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_tool_result_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_code_block_message() {
    let code = r#"fn main() {
    println!("Hello, world!");
}"#;
    let vm = MessageListViewModel {
        messages: vec![assistant_message(&format!("Here is some code:\n```rust\n{}\n```", code))],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_code_block_message", buffer_to_string(&buf));
}

#[test]
fn snapshot_narrow_terminal_40cols() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi there! How can I help?")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 40, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_narrow_terminal_40cols", buffer_to_string(&buf));
}

#[test]
fn snapshot_wide_terminal_120cols() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi there! How can I help you today?")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 120, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_wide_terminal_120cols", buffer_to_string(&buf));
}

#[test]
fn snapshot_short_terminal_12rows() {
    let vm = MessageListViewModel {
        messages: vec![user_message("Hello"), assistant_message("Hi!")],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 6);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_short_terminal_12rows", buffer_to_string(&buf));
}

#[test]
fn snapshot_sidebar_visible() {
    let vm = AgentListViewModel {
        plan_steps: vec![
            (1, "Step 1".to_string(), PlanStatus::Complete),
            (2, "Step 2".to_string(), PlanStatus::Active),
            (3, "Step 3".to_string(), PlanStatus::Pending),
        ],
        running_jobs: vec![],
        active_count: 0,
        tokens: 1000,
        cost: 0.005,
        agent_running: false,
        braille_frame: 0,
    };
    let colors = make_test_colors();
    let area = Rect::new(80 - SIDEBAR_WIDTH, 2, SIDEBAR_WIDTH, 20);
    let mut buf = Buffer::empty(area);
    render_agent_list(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_sidebar_visible", buffer_to_string(&buf));
}

#[test]
fn snapshot_diff_viewer() {
    let diff = DiffViewer::new(
        "src/main.rs".to_string(),
        "fn main() {\n    println!(\"Hello\");\n}".to_string(),
        "fn main() {\n    println!(\"Hello, World!\");\n}".to_string(),
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    diff.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_diff_viewer", buffer_to_string(&buf));
}

#[test]
fn snapshot_dark_theme_status_bar() {
    let vm = StatusBarViewModel {
        mode: TuiMode::Chat,
        current_model: Some("MiniMax-M2.7-highspeed".to_string()),
        session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
        agent_running: false,
        thinking_duration_secs: None,
        message_count: 5,
        max_messages: 50,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 23, 80, 1);
    let mut buf = Buffer::empty(area);
    render_status_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_dark_theme_status_bar", buffer_to_string(&buf));
}

#[test]
fn snapshot_dark_theme_top_bar() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 64_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_dark_theme_top_bar", buffer_to_string(&buf));
}

#[test]
fn snapshot_plan_steps() {
    let vm = MessageListViewModel {
        messages: vec![
            MessageItem::PlanStep { step: 1, text: "Analyze the codebase".to_string(), status: PlanStatus::Complete },
            MessageItem::PlanStep { step: 2, text: "Write tests".to_string(), status: PlanStatus::Active },
            MessageItem::PlanStep { step: 3, text: "Implement feature".to_string(), status: PlanStatus::Pending },
        ],
        scroll_offset: 0,
        agent_running: true,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_plan_steps", buffer_to_string(&buf));
}

#[test]
fn snapshot_tool_call_error() {
    let vm = MessageListViewModel {
        messages: vec![
            tool_call("bash", r#"{"command": "ls"}"#, None, false),
            tool_call("bash", r#"{"command": "rm file"}"#, Some("rm: cannot remove 'file': Permission denied"), true),
        ],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 80, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_tool_call_error", buffer_to_string(&buf));
}

#[test]
fn snapshot_permission_modal_timeout() {
    let mut modal = PermissionModal::new("bash", r#"{"command": "npm install"}"#, "Installing npm packages");
    modal.timeout_secs = Some(45);
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_modal_timeout", buffer_to_string(&buf));
}

#[test]
fn snapshot_long_message_wrap() {
    let long_text = "A".repeat(200);
    let vm = MessageListViewModel {
        messages: vec![assistant_message(&long_text)],
        scroll_offset: 0,
        agent_running: false,
        animation: crate::tui::state::AnimationState::default(),
        wrap_cache: WrapCache::new(),
    };
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 2, 60, 18);
    let mut buf = Buffer::empty(area);
    MessageList::render_ref(&vm, area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_long_message_wrap", buffer_to_string(&buf));
}
