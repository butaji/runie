//! `replay` — programmatic API for replaying JSONL scenarios against a
//! `runie_tui::Tui`. Used by both `scenario_replay` (one-shot CLI
//! diff) and `scenario_fasthot` (hot-reload loop, no per-iter
//! process startup).

use std::path::Path;

use crate::components::SlashMenu;
use crate::tui::{Tui, TuiMode};
use ratatui_textarea::TextArea;

use runie_agent::events::AgentEvent;

pub mod replay_ops;
pub use replay_ops::{load_scenario, parse_scenario, parse_ui_op, parse_mode, apply_ui_op};

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

/// Apply all UI ops from a parsed scenario to a Tui instance.
pub fn apply_scenario(tui: &mut Tui, scenario: &Replay) {
    for action in &scenario.actions {
        match action {
            ScenarioAction::UiOp(op) => {
                replay_ops::apply_ui_op(tui, op);
            }
            ScenarioAction::Event(_) => {
                // Agent events not handled in this simple replay
            }
        }
    }
}

/// Normalize rendered output for comparison (remove timing-sensitive data).
pub fn normalize(text: &str) -> String {
    // Remove timestamps and other non-deterministic data
    text.lines()
        .filter(|l| !l.contains("  ⠋") && !l.contains("  ⠙"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a Tui to text for comparison.
pub fn render_to_text(_tui: &Tui, _w: u16, _h: u16) -> String {
    // Simple text representation for now
    String::new()
}
