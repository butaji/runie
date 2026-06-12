//! Replay parsing operations.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use crate::tui::TuiMode;

use super::super::{UiOp, ScenarioAction, Replay};

pub fn load_scenario(path: &Path) -> Result<Replay, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_scenario(&raw)
}

pub fn parse_scenario(raw: &str) -> Result<Replay, String> {
    let mut out = Replay { width: None, height: None, actions: Vec::new() };
    for (line_no, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let v: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| format!("line {}: {e}", line_no + 1))?;
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

type OpParser = fn(&serde_json::Value) -> Result<UiOp, String>;

fn parse_mode_value(v: &serde_json::Value) -> Result<UiOp, String> {
    v.get("value").or_else(|| v.get("mode"))
        .and_then(|x| x.as_str()).map(parse_mode).map(UiOp::SetMode)
        .ok_or_else(|| String::from("SetMode needs a mode string"))
}

fn parse_bool_op(op: UiOp) -> impl Fn(&serde_json::Value) -> Result<UiOp, String> {
    move |v: &serde_json::Value| {
        v.get("value").and_then(|x| x.as_bool()).map(|_| op.clone())
            .ok_or_else(|| String::from("expected bool value"))
    }
}

static PARSERS: LazyLock<HashMap<&'static str, OpParser>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, OpParser> = HashMap::new();
    m.insert("SetMode", |v| parse_mode_value(v));
    m.insert("SetHomeVisible", |v| v.get("value").and_then(|x| x.as_bool()).map(UiOp::SetHomeVisible)
        .ok_or_else(|| String::from("SetHomeVisible needs bool")));
    m.insert("SetHomeSelected", |v| v.get("value").and_then(|x| x.as_u64()).map(|n| UiOp::SetHomeSelected(n as usize))
        .ok_or_else(|| String::from("SetHomeSelected needs int")));
    m.insert("SetShowSessions", |v| v.get("value").and_then(|x| x.as_bool()).map(UiOp::SetShowSessions)
        .ok_or_else(|| String::from("SetShowSessions needs bool")));
    m.insert("SetSlashOpen", |v| v.get("value").and_then(|x| x.as_bool()).map(UiOp::SetSlashOpen)
        .ok_or_else(|| String::from("SetSlashOpen needs bool")));
    m.insert("SetSlashQuery", |v| v.get("value").and_then(|x| x.as_str()).map(|s| UiOp::SetSlashQuery(s.to_string()))
        .ok_or_else(|| String::from("SetSlashQuery needs string")));
    m.insert("SetInput", |v| v.get("value").and_then(|x| x.as_str()).map(|s| UiOp::SetInput(s.to_string()))
        .ok_or_else(|| String::from("SetInput needs string")));
    m.insert("SetGit", |v| Ok(UiOp::SetGit {
        repo: get_str(v, "repo"), branch: get_str(v, "branch"), path: get_str(v, "path"),
    }));
    m.insert("SetContextWindow", |v| v.get("value").and_then(|x| x.as_u64()).map(|n| UiOp::SetContextWindow(n as usize))
        .ok_or_else(|| String::from("SetContextWindow needs int")));
    m.insert("SetSessionTokens", |v| v.get("value").and_then(|x| x.get("total")).and_then(|x| x.as_u64())
        .map(|n| UiOp::SetSessionTokens { total: n as usize })
        .ok_or_else(|| String::from("SetSessionTokens needs {total: int}")));
    m.insert("SetThoughtDuration", |v| v.get("value").and_then(|x| x.as_f64()).map(|n| UiOp::SetThoughtDuration(n as f32))
        .ok_or_else(|| String::from("SetThoughtDuration needs float")));
    m.insert("SetTurnComplete", |v| v.get("value").and_then(|x| x.as_f64()).map(|n| UiOp::SetTurnComplete(n as f32))
        .ok_or_else(|| String::from("SetTurnComplete needs float")));
    m.insert("SetToolResult", |v| Ok(UiOp::SetToolResult {
        name: get_str(v, "name"), result: get_str(v, "result"),
        is_error: v.get("is_error").and_then(|x| x.as_bool()).unwrap_or(false),
    }));
    m.insert("SetAgentRunning", |v| v.get("value").and_then(|x| x.as_bool()).map(UiOp::SetAgentRunning)
        .ok_or_else(|| String::from("SetAgentRunning needs bool")));
    m
});

pub fn parse_ui_op(v: &serde_json::Value) -> Result<UiOp, String> {
    let kind = v.get("op").or_else(|| v.get("kind"))
        .and_then(|x| x.as_str()).unwrap_or("unknown");
    PARSERS.get(kind).map(|f| f(v))
        .unwrap_or_else(|| Err(format!("unknown ui op: {kind}")))
}

fn get_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

pub fn parse_mode(s: &str) -> TuiMode {
    match s {
        "Chat" | "chat" => TuiMode::Chat,
        "Permission" | "permission" => TuiMode::Permission,
        "Home" | "home" | "HomeScreen" | "homescreen" => TuiMode::HomeScreen,
        "Onboarding" | "onboarding" => TuiMode::Onboarding,
        "Diff" | "diff" => TuiMode::DiffViewer,
        _ => TuiMode::Chat,
    }
}
