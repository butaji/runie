//! Replay apply operations.

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
fn apply_home_visible(tui: &mut Tui, op: &UiOp) { if let UiOp::SetHomeVisible(v) = op { tui.state.home_visible = *v; } }
fn apply_home_selected(tui: &mut Tui, op: &UiOp) { if let UiOp::SetHomeSelected(i) = op { tui.state.home_selected = *i; } }
fn apply_show_sessions(tui: &mut Tui, op: &UiOp) { if let UiOp::SetShowSessions(v) = op { tui.state.show_sessions = *v; } }
fn apply_slash_open(tui: &mut Tui, op: &UiOp) { if let UiOp::SetSlashOpen(v) = op { tui.state.slash_open = *v; } }
fn apply_slash_query(tui: &mut Tui, op: &UiOp) { if let UiOp::SetSlashQuery(q) = op { tui.state.slash_menu.query = q.to_string(); crate::components::slash_menu::rerank(&mut tui.state.slash_menu); } }
fn apply_input(tui: &mut Tui, op: &UiOp) { if let UiOp::SetInput(s) = op { let mut ta = TextArea::default(); ta.move_cursor(ratatui_textarea::TextAreaBookmark::End); ta.insert_str(s); tui.state.textarea = ta; } }
fn apply_git(tui: &mut Tui, op: &UiOp) { if let UiOp::SetGit { repo, branch, path } = op { tui.state.top_bar.git = Some(crate::components::top_bar::GitInfo { repo: repo.to_string(), branch: branch.to_string(), path: path.to_string() }); } }
fn apply_context_window(tui: &mut Tui, op: &UiOp) { if let UiOp::SetContextWindow(n) = op { tui.state.top_bar.context_window = Some(*n); } }
fn apply_session_tokens(tui: &mut Tui, op: &UiOp) { if let UiOp::SetSessionTokens { total } = op { tui.state.session_token_usage.total_tokens = *total; } }
fn apply_thought_duration(tui: &mut Tui, op: &UiOp) { if let UiOp::SetThoughtDuration(d) = op { tui.state.pending_thought_duration = Some(*d); } }
fn apply_turn_complete(tui: &mut Tui, op: &UiOp) { if let UiOp::SetTurnComplete(d) = op { tui.state.replay_turn_duration_secs = Some(*d); } }
fn apply_agent_running(tui: &mut Tui, op: &UiOp) { if let UiOp::SetAgentRunning(v) = op { tui.state.agent_running = *v; } }
fn apply_tool(tui: &mut Tui, op: &UiOp) {
    if let UiOp::SetToolResult { name, result, is_error } = op {
        let pos = tui.state.messages.iter().rposition(|m| matches!(m, MessageItem::ToolRunning { name: n, .. } if n == name));
        if let Some(pos) = pos {
            let msg = &mut tui.state.messages[pos];
            if let MessageItem::ToolRunning { name: n, args: a, .. } = std::mem::take(msg) {
                *msg = MessageItem::ToolCall { name: n, args: a, result: Some(result.to_string()), is_error: *is_error };
            }
        }
    }
}
