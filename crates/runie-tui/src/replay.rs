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
