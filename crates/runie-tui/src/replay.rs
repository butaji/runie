//! `replay` — programmatic API for replaying JSONL scenarios against a
//! `runie_tui::Tui`. Used by both `scenario_replay` (one-shot CLI
//! diff) and `scenario_fasthot` (hot-reload loop, no per-iter
//! process startup).
//!
//! Extracting this from `bin/scenario_replay.rs` is what makes
//! `scenario_fasthot` possible: a long-lived process can hold a
//! `Tui` in memory and re-apply scenarios against it as JSONL
//! files change, paying ~1ms per re-render instead of ~350ms
//! for process startup + framework init.

use std::path::Path;

use crate::components::SlashMenu;
use crate::tui::{Tui, TuiMode};
use ratatui_textarea::TextArea;

use runie_agent::events::AgentEvent;

/// The full scenario: a flat list of UI ops + agent events.
/// Order is preserved — the scenario file may interleave them.
#[derive(Debug, Clone, Default)]
pub struct Replay {
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub actions: Vec<ScenarioAction>,
}

#[derive(Debug, Clone)]
pub enum ScenarioAction {
    UiOp(UiOp),
    Event(serde_json::Value),
}

#[derive(Debug, Clone)]
pub enum UiOp {
    SetMode(TuiMode),
    SetHomeVisible(bool),
    SetHomeSelected(usize),
    SetShowSessions(bool),
    SetSlashOpen(bool),
    SetSlashQuery(String),
    SetInput(String),
    SetGit {
        repo: String,
        branch: String,
        path: String,
    },
    SetContextWindow(usize),
    SetSessionTokens {
        total: usize,
    },
    SetThoughtDuration(f32),
    SetTurnComplete(f32),
    SetToolResult {
        name: String,
        result: String,
        is_error: bool,
    },
    SetAgentRunning(bool),
}

/// Load a scenario from a JSONL file.
pub fn load_scenario(path: &Path) -> Result<Replay, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_scenario(&raw)
}

/// Parse a scenario from a JSONL string.
pub fn parse_scenario(raw: &str) -> Result<Replay, String> {
    let mut out = Replay {
        width: None,
        height: None,
        actions: Vec::new(),
    };
    for (line_no, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("line {}: {e}", line_no + 1))?;
        if v.get("__meta__").and_then(|x| x.as_bool()).unwrap_or(false) {
            out.width = v.get("width").and_then(|x| x.as_u64()).map(|n| n as u16);
            out.height = v.get("height").and_then(|x| x.as_u64()).map(|n| n as u16);
            continue;
        }
        if v.get("ui").and_then(|x| x.as_bool()).unwrap_or(false) {
            let op = parse_ui_op(&v).map_err(|e| format!("line {} (ui): {e}", line_no + 1))?;
            out.actions.push(ScenarioAction::UiOp(op));
            continue;
        }
        out.actions.push(ScenarioAction::Event(v));
    }
    Ok(out)
}

fn parse_ui_op(v: &serde_json::Value) -> Result<UiOp, String> {
    // Two shapes are accepted:
    //   1. {"ui": true, "kind": "X", "value": ...}  — legacy format
    //      where `ui: true` is just a marker. The op fields live on
    //      the outer object.
    //   2. {"ui": {"op": "X", "value": ...}}         — newer format
    //      where the op fields live on the `ui` sub-object.
    let ui: &serde_json::Map<String, serde_json::Value> = match v.get("ui") {
        Some(serde_json::Value::Object(map)) => map,
        Some(serde_json::Value::Bool(true)) => {
            // Legacy: pull from outer object (everything except `ui`).
            v.as_object().ok_or("ui: not an object")?
        }
        _ => return Err("ui: missing or not true".to_string()),
    };
    // Accept both "op" (new format) and "kind" (legacy format used by
    // existing scenario files).
    let kind = ui
        .get("op")
        .or_else(|| ui.get("kind"))
        .and_then(|x| x.as_str())
        .ok_or("ui.op/kind missing")?;
    match kind {
        "mode" => {
            let s = ui.get("value").and_then(|x| x.as_str()).ok_or("ui.value missing")?;
            Ok(UiOp::SetMode(parse_mode(s)))
        }
        "home_visible" => {
            let b = ui.get("value").and_then(|x| x.as_bool()).ok_or("ui.value missing")?;
            Ok(UiOp::SetHomeVisible(b))
        }
        "home_selected" => {
            let n = ui.get("value").and_then(|x| x.as_u64()).ok_or("ui.value missing")? as usize;
            Ok(UiOp::SetHomeSelected(n))
        }
        "show_sessions" => {
            let b = ui.get("value").and_then(|x| x.as_bool()).ok_or("ui.value missing")?;
            Ok(UiOp::SetShowSessions(b))
        }
        "slash_open" => {
            let b = ui.get("value").and_then(|x| x.as_bool()).ok_or("ui.value missing")?;
            Ok(UiOp::SetSlashOpen(b))
        }
        "slash_query" => {
            let s = ui.get("value").and_then(|x| x.as_str()).ok_or("ui.value missing")?;
            Ok(UiOp::SetSlashQuery(s.to_string()))
        }
        "input" => {
            let s = ui.get("value").and_then(|x| x.as_str()).ok_or("ui.value missing")?;
            Ok(UiOp::SetInput(s.to_string()))
        }
        "git" => {
            let obj = ui.get("value").and_then(|x| x.as_object()).ok_or("ui.value missing")?;
            Ok(UiOp::SetGit {
                repo: obj.get("repo").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                branch: obj.get("branch").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                path: obj.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            })
        }
        "context_window" => {
            let n = ui.get("value").and_then(|x| x.as_u64()).ok_or("ui.value missing")? as usize;
            Ok(UiOp::SetContextWindow(n))
        }
        "session_tokens" => {
            let obj = ui.get("value").and_then(|x| x.as_object()).ok_or("ui.value missing")?;
            Ok(UiOp::SetSessionTokens {
                total: obj.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            })
        }
        "thought_duration" => {
            let n = ui.get("value").and_then(|x| x.as_f64()).ok_or("ui.value missing")? as f32;
            Ok(UiOp::SetThoughtDuration(n))
        }
        "turn_complete" => {
            let n = ui.get("value").and_then(|x| x.as_f64()).ok_or("ui.value missing")? as f32;
            Ok(UiOp::SetTurnComplete(n))
        }
        "tool_result" => {
            let obj = ui.get("value").and_then(|x| x.as_object()).ok_or("ui.value missing")?;
            Ok(UiOp::SetToolResult {
                name: obj.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                result: obj.get("result").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                is_error: obj.get("is_error").and_then(|x| x.as_bool()).unwrap_or(false),
            })
        }
        "agent_running" => {
            let b = ui.get("value").and_then(|x| x.as_bool()).ok_or("ui.value missing")?;
            Ok(UiOp::SetAgentRunning(b))
        }
        other => Err(format!("unknown ui.op: {other}")),
    }
}

fn parse_mode(s: &str) -> TuiMode {
    match s {
        "home" => TuiMode::HomeScreen,
        "chat" => TuiMode::Chat,
        "onboarding" => TuiMode::Onboarding,
        "command_palette" => TuiMode::CommandPalette,
        "permission" => TuiMode::Permission,
        "plan" => TuiMode::Plan,
        "diff_viewer" => TuiMode::DiffViewer,
        "session_tree" => TuiMode::SessionTree,
        _ => TuiMode::Chat,
    }
}

/// Apply a scenario to a Tui in-place. Returns the (width, height) the
/// scenario wants, falling back to (80, 24) if not set.
pub fn apply_scenario(tui: &mut Tui, scenario: &Replay) {
    for action in &scenario.actions {
        match action {
            ScenarioAction::UiOp(op) => {
                apply_ui_op(tui, op);
            }
            ScenarioAction::Event(ev) => {
                if let Ok(event) = serde_json::from_value::<AgentEvent>(ev.clone()) {
                    tui.on_agent_event(event);
                }
            }
        }
    }
}

fn apply_ui_op(tui: &mut Tui, op: &UiOp) {
    let state = &mut tui.state;
    match op {
        UiOp::SetMode(m) => {
            state.mode = m.clone();
        }
        UiOp::SetHomeVisible(b) => {
            if *b { state.home_screen.show(); } else { state.home_screen.hide(); }
        }
        UiOp::SetHomeSelected(n) => {
            state.home_screen.selected = *n;
        }
        UiOp::SetShowSessions(b) => {
            state.home_screen.show_sessions = *b;
        }
        UiOp::SetSlashOpen(b) => {
            if *b {
                state.slash_menu = SlashMenu::new();
                state.slash_menu.open("");
            } else {
                state.slash_menu.close();
            }
        }
        UiOp::SetSlashQuery(s) => {
            state.slash_menu.open(s);
        }
        UiOp::SetInput(s) => {
            state.textarea = TextArea::default();
            state.textarea.insert_str(s);
        }
        UiOp::SetGit { repo, branch, path } => {
            state.context.repo = repo.clone();
            state.context.branch = branch.clone();
            state.context.path = path.clone();
            state.top_bar.repo = repo.clone();
            state.top_bar.branch = branch.clone();
            state.top_bar.path = path.clone();
        }
        UiOp::SetContextWindow(n) => {
            state.top_bar.context_window = Some(*n);
            state.top_bar.estimated_tokens = Some(state.session_token_usage.total_tokens);
        }
        UiOp::SetSessionTokens { total } => {
            state.session_token_usage.total_tokens = *total;
            state.top_bar.estimated_tokens = Some(*total);
        }
        UiOp::SetThoughtDuration(d) => {
            if let Some(crate::components::MessageItem::Assistant {
                thought_duration,
                ..
            }) = state.messages.iter_mut().rev().find(|m| matches!(m, crate::components::MessageItem::Assistant { .. }))
            {
                *thought_duration = Some(*d);
            }
        }
        UiOp::SetTurnComplete(d) => {
            // Last assistant's turn_duration
            if let Some(crate::components::MessageItem::Assistant {
                turn_duration,
                ..
            }) = state.messages.iter_mut().rev().find(|m| matches!(m, crate::components::MessageItem::Assistant { .. }))
            {
                *turn_duration = Some(*d);
            }
        }
        UiOp::SetToolResult { name, result, is_error } => {
            for msg in state.messages.iter_mut().rev() {
                let n = match msg {
                    crate::components::MessageItem::ToolRunning { name, .. } => Some(name.clone()),
                    crate::components::MessageItem::ToolCall { name, .. } => Some(name.clone()),
                    _ => None,
                };
                if let Some(n) = n {
                    if &n == name {
                        *msg = crate::components::MessageItem::ToolCall {
                            name: n,
                            args: String::new(),
                            result: Some(result.clone()),
                            is_error: *is_error,
                        };
                        break;
                    }
                }
            }
        }
        UiOp::SetAgentRunning(b) => {
            state.agent_running = *b;
        }
    }
}

/// Render the current Tui state to a plain-text string (with ANSI).
/// Used for diffing against Grok dump files.
pub fn render_to_text(tui: &mut Tui, width: u16, height: u16) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend init");
    let _ = terminal.draw(|f| {
        let area = f.area();
        tui.render_to_buffer(f.buffer_mut(), area);
    });
    let mut out = String::new();
    let buf = terminal.backend().buffer().clone();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                out.push_str(cell.symbol());
            }
        }
        if y + 1 < buf.area.height {
            out.push('\n');
        }
    }
    out
}

/// Normalize rendered text for diffing: trim trailing whitespace on
/// each line, drop trailing blank lines.
pub fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string()
}

#[allow(dead_code)]
fn _force_use_agent_event(_: AgentEvent) {}
#[allow(dead_code)]
fn _force_use_slash(_: SlashMenu) {}
