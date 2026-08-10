//! Actor-owned state for interactive Runie/Grok commands.
//!
//! Command parsing lives in `commands`; this actor owns the mutable effects
//! of commands which are not part of the Pi session journal itself.

use crate::task_owner::{spawn_actor_worker, TaskOwner};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ApprovalMode {
    #[default]
    Ask,
    Deny,
    Auto,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    pub fix_requested: bool,
    pub checks: Vec<String>,
}

impl DiagnosticReport {
    pub fn summary(&self) -> String {
        format!(
            "{} ({} checks)",
            if self.fix_requested {
                "fix requested"
            } else {
                "diagnostic requested"
            },
            self.checks.len()
        )
    }

    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![self.summary()];
        lines.extend(self.checks.iter().map(|check| format!("check: {check}")));
        lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommandState {
    pub last_command: Option<String>,
    pub invocation_count: u64,
    pub settings: BTreeMap<String, String>,
    pub approval: ApprovalMode,
    pub effort: Option<String>,
    pub plan: Option<String>,
    pub memory_enabled: bool,
    pub remembered: Vec<String>,
    pub authenticated: Vec<String>,
    pub active_goal: Option<String>,
    pub workflows: Vec<String>,
    pub scheduled_loops: Vec<String>,
    pub research_queries: Vec<String>,
    pub trusted_projects: Vec<String>,
    pub enabled_skills: bool,
    pub hooks_loaded: bool,
    pub plugins_loaded: bool,
    pub mcps_loaded: bool,
    pub shared_sessions: u64,
    pub feedback: Vec<String>,
    pub history_queries: Vec<String>,
    pub transcript_queries: Vec<String>,
    pub rewind_requests: Vec<String>,
    pub usage_requests: u64,
    pub reload_count: u64,
    pub last_diagnostic: Option<String>,
    #[serde(default)]
    pub diagnostic_report: Option<DiagnosticReport>,
}

enum CommandMessage {
    Invoke {
        name: String,
        args: String,
        reply: oneshot::Sender<CommandState>,
    },
}

#[derive(Clone)]
pub struct CommandActor {
    tx: mpsc::Sender<CommandMessage>,
    snapshot: watch::Receiver<CommandState>,
    _owner: Arc<TaskOwner>,
}

impl CommandActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(CommandState::default());
        let (tx, owner) = spawn_actor_worker!(128, move |mut rx: mpsc::Receiver<
            CommandMessage,
        >| async move {
            let mut state = CommandState::default();
            while let Some(CommandMessage::Invoke { name, args, reply }) = rx.recv().await {
                reduce(&mut state, &name, &args);
                let _ = snapshot_tx.send(state.clone());
                let _ = reply.send(state.clone());
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub async fn invoke(&self, name: impl Into<String>, args: impl Into<String>) -> CommandState {
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(CommandMessage::Invoke {
                name: name.into(),
                args: args.into(),
                reply,
            })
            .await
            .is_err()
        {
            return self.snapshot();
        }
        result.await.unwrap_or_else(|_| self.snapshot())
    }

    pub fn snapshot(&self) -> CommandState {
        self.snapshot.borrow().clone()
    }
}

impl Default for CommandActor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn reduce(state: &mut CommandState, name: &str, raw_args: &str) {
    let args = raw_args.trim();
    state.last_command = Some(name.to_owned());
    state.invocation_count = state.invocation_count.saturating_add(1);
    reduce_command(state, name, args);
}

fn reduce_command(state: &mut CommandState, name: &str, args: &str) {
    if reduce_command_collection(state, name, args) {
        return;
    }
    match name {
        "deny" => state.approval = approval(args, ApprovalMode::Deny),
        "always-approve" => state.approval = approval(args, ApprovalMode::Always),
        "auto" => state.approval = approval(args, ApprovalMode::Auto),
        "plan" => state.plan = (!args.is_empty()).then(|| args.into()),
        "memory" => update_memory(state, args),
        "remember" if !args.is_empty() => state.remembered.push(args.into()),
        "login" => add_unique(&mut state.authenticated, args),
        "logout" => state.authenticated.retain(|p| p != args),
        "goal" => update_goal(state, args),
        "workflow" if !args.is_empty() => state.workflows.push(args.into()),
        "loop" if !args.is_empty() => state.scheduled_loops.push(args.into()),
        "deep-research" if !args.is_empty() => state.research_queries.push(args.into()),
        "trust" => add_unique(
            &mut state.trusted_projects,
            if args.is_empty() { "." } else { args },
        ),
        "reload" => state.reload_count = state.reload_count.saturating_add(1),
        "skills" => state.enabled_skills = true,
        "hooks" => state.hooks_loaded = true,
        "plugins" => state.plugins_loaded = true,
        "mcps" => state.mcps_loaded = true,
        "doctor" => {
            let report = DiagnosticReport {
                fix_requested: args == "fix",
                checks: vec!["workspace".into(), "provider".into(), "session".into()],
            };
            state.last_diagnostic = Some(report.summary());
            state.diagnostic_report = Some(report);
        }
        _ => {}
    }
}

fn reduce_command_collection(state: &mut CommandState, name: &str, args: &str) -> bool {
    match name {
        "settings" => update_setting(state, args),
        "share" => state.shared_sessions = state.shared_sessions.saturating_add(1),
        "feedback" if !args.is_empty() => state.feedback.push(args.into()),
        "history" if !args.is_empty() => state.history_queries.push(args.into()),
        "find" | "jump" if !args.is_empty() => state.transcript_queries.push(args.into()),
        "rewind" if !args.is_empty() => state.rewind_requests.push(args.into()),
        "usage" => state.usage_requests = state.usage_requests.saturating_add(1),
        "effort" => update_effort(state, args),
        _ => return false,
    }
    true
}

fn update_setting(state: &mut CommandState, args: &str) {
    if let Some((key, value)) = args.split_once('=') {
        state
            .settings
            .insert(key.trim().into(), value.trim().into());
    }
}

fn update_effort(state: &mut CommandState, args: &str) {
    if matches!(args, "low" | "medium" | "high" | "xhigh") {
        state.effort = Some(args.into());
    }
}

fn approval(args: &str, enabled: ApprovalMode) -> ApprovalMode {
    if is_off(args) {
        ApprovalMode::Ask
    } else {
        enabled
    }
}

fn update_memory(state: &mut CommandState, args: &str) {
    if matches!(args, "on" | "enable") {
        state.memory_enabled = true;
    }
    if matches!(args, "off" | "disable") {
        state.memory_enabled = false;
    }
}

fn add_unique(values: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !values.iter().any(|item| item == value) {
        values.push(value.into());
    }
}

fn update_goal(state: &mut CommandState, args: &str) {
    if args == "clear" {
        state.active_goal = None;
    } else if !args.is_empty() && !matches!(args, "status" | "pause" | "resume") {
        state.active_goal = Some(args.into());
    }
}

fn is_off(value: &str) -> bool {
    matches!(value, "off" | "false" | "0" | "no" | "disable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn command_sequence_reduces_through_one_actor() {
        let actor = CommandActor::new();
        actor.invoke("settings", "theme=dark").await;
        actor.invoke("effort", "high").await;
        actor.invoke("remember", "keep auth details").await;
        let state = actor.snapshot();
        assert_eq!(state.settings.get("theme"), Some(&"dark".into()));
        assert_eq!(state.effort.as_deref(), Some("high"));
        assert_eq!(state.remembered, vec!["keep auth details"]);
    }

    #[tokio::test]
    async fn integration_commands_have_observable_state_transitions() {
        let actor = CommandActor::new();
        actor.invoke("reload", "").await;
        actor.invoke("trust", "/workspace").await;
        actor.invoke("skills", "").await;
        actor.invoke("loop", "30m check status").await;
        actor.invoke("deep-research", "compare providers").await;
        actor.invoke("share", "").await;
        actor.invoke("feedback", "needs clearer output").await;
        actor.invoke("find", "provider").await;
        let state = actor.snapshot();
        assert_eq!(state.reload_count, 1);
        assert_eq!(state.trusted_projects, vec!["/workspace"]);
        assert!(state.enabled_skills);
        assert_eq!(state.scheduled_loops, vec!["30m check status"]);
        assert_eq!(state.research_queries, vec!["compare providers"]);
        assert_eq!(state.shared_sessions, 1);
        assert_eq!(state.feedback, vec!["needs clearer output"]);
        assert_eq!(state.transcript_queries, vec!["provider"]);
    }

    #[tokio::test]
    async fn doctor_projects_a_replayable_report() {
        let actor = CommandActor::new();
        actor.invoke("doctor", "fix").await;
        let state = actor.snapshot();
        let report = state.diagnostic_report.expect("diagnostic report");
        assert!(report.fix_requested);
        assert_eq!(report.checks, ["workspace", "provider", "session"]);
        assert_eq!(
            state.last_diagnostic.as_deref(),
            Some("fix requested (3 checks)")
        );
        assert_eq!(
            report.terminal_lines(),
            vec![
                "fix requested (3 checks)",
                "check: workspace",
                "check: provider",
                "check: session"
            ]
        );
    }
}
