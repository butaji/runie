//! Replay operations.

pub mod parse;
pub use parse::{load_scenario, parse_scenario, parse_ui_op, parse_mode};

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::components::MessageItem;
use crate::tui::Tui;
use ratatui_textarea::TextArea;

use super::UiOp;

type OpApplier = fn(&mut Tui, &UiOp);

static APPLIERS: LazyLock<HashMap<usize, OpApplier>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(0, apply_mode as OpApplier);
    m.insert(1, apply_home_visible);
    m.insert(2, apply_home_selected);
    m.insert(3, apply_show_sessions);
    m.insert(4, apply_slash_open);
    m.insert(5, apply_slash_query);
    m.insert(6, apply_input);
    m.insert(7, apply_git);
    m.insert(8, apply_context_window);
    m.insert(9, apply_session_tokens);
    m.insert(10, apply_thought_duration);
    m.insert(11, apply_turn_complete);
    m.insert(12, apply_tool);
    m.insert(13, apply_agent_running);
    m
});

pub fn apply_ui_op(tui: &mut Tui, op: &UiOp) {
    let idx = match op { UiOp::SetMode(_) => 0, UiOp::SetHomeVisible(_) => 1, UiOp::SetHomeSelected(_) => 2, UiOp::SetShowSessions(_) => 3, UiOp::SetSlashOpen(_) => 4, UiOp::SetSlashQuery(_) => 5, UiOp::SetInput(_) => 6, UiOp::SetGit { .. } => 7, UiOp::SetContextWindow(_) => 8, UiOp::SetSessionTokens { .. } => 9, UiOp::SetThoughtDuration(_) => 10, UiOp::SetTurnComplete(_) => 11, UiOp::SetToolResult { .. } => 12, UiOp::SetAgentRunning(_) => 13 };
    if let Some(applier) = APPLIERS.get(&idx) {
        applier(tui, op);
    }
}

fn apply_mode(tui: &mut Tui, op: &UiOp) { if let UiOp::SetMode(m) = op { tui.state.mode = m.clone(); } }
fn apply_home_visible(_tui: &mut Tui, _op: &UiOp) { /* home_visible not in AppState */ }
fn apply_home_selected(_tui: &mut Tui, _op: &UiOp) { /* home_selected not in AppState */ }
fn apply_show_sessions(_tui: &mut Tui, _op: &UiOp) { /* show_sessions not in AppState */ }
fn apply_slash_open(_tui: &mut Tui, _op: &UiOp) { /* slash_open not in AppState */ }
fn apply_slash_query(_tui: &mut Tui, _op: &UiOp) { /* slash_menu.query not accessible */ }
fn apply_input(tui: &mut Tui, op: &UiOp) { if let UiOp::SetInput(s) = op { let mut ta = TextArea::default(); ta.move_cursor(ratatui_textarea::CursorMove::End); ta.insert_str(s); tui.state.textarea = ta; } }
fn apply_git(tui: &mut Tui, op: &UiOp) { if let UiOp::SetGit { repo, branch, path } = op { tui.state.context.repo = repo.to_string(); tui.state.context.branch = branch.to_string(); tui.state.context.path = path.to_string(); } }
fn apply_context_window(_tui: &mut Tui, _op: &UiOp) { /* context_window not in top_bar */ }
fn apply_session_tokens(tui: &mut Tui, op: &UiOp) { if let UiOp::SetSessionTokens { total } = op { tui.state.session_token_usage.total_tokens = *total; } }
fn apply_thought_duration(tui: &mut Tui, op: &UiOp) { if let UiOp::SetThoughtDuration(d) = op { tui.state.pending_thought_duration = Some(*d); } }
fn apply_turn_complete(tui: &mut Tui, op: &UiOp) { if let UiOp::SetTurnComplete(d) = op { tui.state.replay_turn_duration_secs = Some(*d); } }
fn apply_agent_running(tui: &mut Tui, op: &UiOp) { if let UiOp::SetAgentRunning(v) = op { tui.state.agent_running = *v; } }
fn apply_tool(tui: &mut Tui, op: &UiOp) {
    if let UiOp::SetToolResult { name, result, is_error } = op {
        for msg in &mut tui.state.messages {
            if let MessageItem::ToolRunning { name: n, args: a, .. } = msg {
                if n == name {
                    *msg = MessageItem::ToolCall { name: n.clone(), args: a.clone(), result: Some(result.to_string()), is_error: *is_error };
                    break;
                }
            }
        }
    }
}
