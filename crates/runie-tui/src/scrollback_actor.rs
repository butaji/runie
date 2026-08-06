//! Actor-owned transcript projection.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{mpsc, oneshot, watch};

use runie_core::types::AgentEvent;
use runie_core::{mailbox_ack, spawn_actor_worker, spawn_owned_worker, task_owner::TaskOwner};

use crate::widgets::{Scrollback, ScrollbackMsg};

enum Command {
    ApplyBatch(Vec<ScrollbackMsg>, oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct ScrollbackActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<Scrollback>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl ScrollbackActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(Scrollback::new());
        let (tx, owner) = spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Command>| async move {
            let mut state = Scrollback::new();
            while let Some(command) = rx.recv().await {
                let (messages, reply) = match command {
                    Command::ApplyBatch(messages, reply) => (messages, reply),
                };
                for message in messages {
                    state.apply(message);
                }
                let _ = snapshot_tx.send(state.clone());
                let _ = reply.send(());
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

    /// Construct a live transcript projection that owns its lifecycle-event
    /// subscription. Complex feed rendering remains an explicit event reducer;
    /// reset ownership is handled here by the transcript actor itself.
    pub fn new_with_bus(bus: &runie_core::events::EventBus) -> Self {
        let mut actor = Self::new();
        let events = bus.subscribe();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(run_bus_projection(events, tx)));
        actor
    }

    pub async fn apply(&self, message: ScrollbackMsg) {
        self.apply_batch(vec![message]).await;
    }

    pub async fn apply_batch(&self, messages: Vec<ScrollbackMsg>) {
        let _ = mailbox_ack!(self.tx, |reply| Command::ApplyBatch(messages, reply));
    }

    pub fn snapshot(&self) -> Scrollback {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Scrollback> {
        self.snapshot.clone()
    }
}

async fn run_bus_projection(
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    tx: mpsc::Sender<Command>,
) {
    let mut active_tools = HashSet::new();
    let mut tool_headers = HashMap::new();
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        };
        if let AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } = &event
        {
            active_tools.insert(tool_call_id.clone());
            tool_headers.insert(
                tool_call_id.clone(),
                crate::event_renderer::tool_header(tool_name, args),
            );
        }
        if let AgentEvent::ToolExecutionEnd { tool_call_id, .. } = &event {
            active_tools.remove(tool_call_id);
            tool_headers.remove(tool_call_id);
        }
        let messages = bus_messages_for_event(event.clone());
        let messages = if messages.is_empty() {
            tool_update_messages(&active_tools, &mut tool_headers, &event)
        } else {
            messages
        };
        if !messages.is_empty() && !mailbox_ack!(tx, |reply| Command::ApplyBatch(messages, reply)) {
            break;
        }
    }
}

fn default_tool_display_mode(tool_name: &str) -> runie_core::types::ToolDisplayMode {
    if matches!(tool_name, "bash" | "shell" | "exec" | "run") {
        runie_core::types::ToolDisplayMode::Truncated
    } else {
        runie_core::types::ToolDisplayMode::Collapsed
    }
}

fn format_elapsed(elapsed_ms: Option<u64>) -> String {
    elapsed_ms
        .map(|millis| format!(" in {:.1}s", millis as f64 / 1_000.0))
        .unwrap_or_default()
}

fn format_error(is_error: bool, error: Option<&str>) -> String {
    if is_error {
        error.map(|value| format!(" ({value})")).unwrap_or_default()
    } else {
        String::new()
    }
}

fn bus_messages_for_event(event: AgentEvent) -> Vec<ScrollbackMsg> {
    match event {
        AgentEvent::Reset => vec![ScrollbackMsg::Clear],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(theme)],
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id, mode)]
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            ..
        } => vec![ScrollbackMsg::SetToolMode(
            tool_call_id,
            default_tool_display_mode(&tool_name),
        )],
        event @ (AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }) => background_messages_for_event(event),
        _ => Vec::new(),
    }
}

fn structured_update_messages(
    active_tools: &HashSet<String>,
    event: &AgentEvent,
) -> Vec<ScrollbackMsg> {
    let AgentEvent::ToolExecutionUpdate {
        tool_call_id,
        partial_result,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !active_tools.contains(tool_call_id) {
        return Vec::new();
    }
    let Some(output) = structured_update_text(partial_result) else {
        return Vec::new();
    };
    let output = output
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if output.is_empty() {
        Vec::new()
    } else {
        vec![ScrollbackMsg::ToolUpdate {
            tool_call_id: tool_call_id.clone(),
            header: None,
            output,
        }]
    }
}

fn tool_update_messages(
    active_tools: &HashSet<String>,
    tool_headers: &mut HashMap<String, String>,
    event: &AgentEvent,
) -> Vec<ScrollbackMsg> {
    let structured = structured_update_messages(active_tools, event);
    if !structured.is_empty() {
        return structured;
    }
    let AgentEvent::ToolExecutionUpdate {
        tool_call_id,
        partial_result,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !active_tools.contains(tool_call_id)
        || (partial_result
            .get("status")
            .and_then(serde_json::Value::as_str)
            .is_some()
            && structured_update_text(partial_result).is_none())
    {
        return Vec::new();
    }
    let Some(header) = tool_headers.get_mut(tool_call_id) else {
        return Vec::new();
    };
    header.push_str(&format!(
        " | update: {}",
        serde_json::to_string(partial_result).unwrap_or_default()
    ));
    vec![ScrollbackMsg::ToolUpdate {
        tool_call_id: tool_call_id.clone(),
        header: Some(header.clone()),
        output: Vec::new(),
    }]
}

fn structured_update_text(result: &serde_json::Value) -> Option<String> {
    result
        .get("output")
        .and_then(serde_json::Value::as_str)
        .or_else(|| result.get("content").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

#[allow(
    clippy::too_many_lines,
    reason = "background lifecycle formatting keeps Grok card variants explicit"
)]
fn background_messages_for_event(event: AgentEvent) -> Vec<ScrollbackMsg> {
    match event {
        AgentEvent::BackgroundWorkStarted {
            work_id,
            description,
            background,
        } => vec![ScrollbackMsg::ToolStart {
            tool_call_id: work_id,
            header: format!(
                "Subagent {}: {description:?}",
                if background { "started" } else { "running" }
            ),
            activity: None,
        }],
        AgentEvent::BackgroundWorkProgress {
            work_id,
            description,
            activity,
        } => vec![ScrollbackMsg::ToolUpdate {
            tool_call_id: work_id,
            header: Some(format!("Subagent running: {description:?} — {activity}")),
            output: Vec::new(),
        }],
        AgentEvent::BackgroundWorkFinished {
            work_id,
            description,
            is_error,
            elapsed_ms,
            error,
        } => {
            let mut messages = vec![ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent {}{}{}: {description:?}",
                    if is_error { "failed" } else { "completed" },
                    format_elapsed(elapsed_ms),
                    format_error(is_error, error.as_deref())
                ),
                activity: None,
                output: Vec::new(),
            }];
            if is_error {
                messages.push(ScrollbackMsg::MarkToolError(work_id));
            }
            messages
        }
        AgentEvent::BackgroundWorkCancelled {
            work_id,
            description,
            elapsed_ms,
        } => vec![
            ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent cancelled{}: {description:?}",
                    format_elapsed(elapsed_ms)
                ),
                activity: None,
                output: Vec::new(),
            },
            ScrollbackMsg::MarkToolError(work_id),
        ],
        _ => Vec::new(),
    }
}

impl Default for ScrollbackActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ScrollbackActor;
    use crate::widgets::{Line, LineKind, ScrollbackMsg};
    use runie_core::types::AgentEvent;

    #[tokio::test]
    async fn actor_publishes_acknowledged_feed_snapshot() {
        let actor = ScrollbackActor::new();
        let index = actor.snapshot().lines().len();
        actor
            .apply(ScrollbackMsg::Append(Line::new(
                LineKind::Assistant,
                "hello",
            )))
            .await;
        assert_eq!(actor.snapshot().lines()[index].text, "hello");
    }

    #[tokio::test]
    async fn bus_owned_actor_clears_on_reset() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        actor
            .apply(ScrollbackMsg::Append(Line::new(
                LineKind::Assistant,
                "hello",
            )))
            .await;
        bus.publish(AgentEvent::Reset);
        snapshot.changed().await.expect("scrollback bus projection");
        snapshot
            .changed()
            .await
            .expect("scrollback reset projection");
        assert!(actor.snapshot().lines().is_empty());
    }

    #[tokio::test]
    async fn bus_owned_actor_projects_theme_changes() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::GrokDay,
        });
        snapshot
            .changed()
            .await
            .expect("scrollback theme projection");
        assert_eq!(
            actor.snapshot().theme(),
            runie_core::types::ThemeKind::GrokDay
        );
    }

    #[tokio::test]
    async fn bus_owned_actor_projects_background_lifecycle() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::BackgroundWorkStarted {
            work_id: "worker-1".into(),
            description: "inspect files".into(),
            background: true,
        });
        snapshot
            .changed()
            .await
            .expect("background start projection");
        assert!(actor
            .snapshot()
            .lines()
            .iter()
            .any(|line| line.text.contains("Subagent started")));
    }

    #[tokio::test]
    async fn bus_owned_actor_projects_structured_tool_updates() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        });
        snapshot.changed().await.expect("tool start projection");
        bus.publish(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "tool-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
            partial_result: serde_json::json!({"output": "line one\nline two"}),
        });
        snapshot.changed().await.expect("tool update projection");
        assert!(actor
            .snapshot()
            .lines()
            .iter()
            .any(|line| line.text == "line one"));
    }

    #[tokio::test]
    async fn bus_owned_actor_projects_non_structured_tool_updates() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        actor
            .apply(ScrollbackMsg::ToolStart {
                tool_call_id: "tool-2".into(),
                header: "Bash pwd".into(),
                activity: None,
            })
            .await;
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-2".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"command": "pwd"}),
        });
        snapshot.changed().await.expect("tool start projection");
        bus.publish(AgentEvent::ToolExecutionUpdate {
            tool_call_id: "tool-2".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"command": "pwd"}),
            partial_result: serde_json::json!({"step": 2}),
        });
        snapshot
            .changed()
            .await
            .expect("tool header update projection");
        assert!(actor
            .snapshot()
            .lines()
            .iter()
            .any(|line| line.text.contains("update")));
    }

    #[tokio::test]
    async fn actor_reduces_completed_turn_summary_atomically() {
        let actor = ScrollbackActor::new();
        actor
            .apply(ScrollbackMsg::AppendTurnSummary("Worked for 1.0s".into()))
            .await;
        let lines = actor.snapshot();
        assert_eq!(lines.lines().len(), 1);
        assert_eq!(lines.lines()[0].kind, LineKind::TurnSummary);
        assert_eq!(lines.lines()[0].text, "Worked for 1.0s");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn actor_reduces_parallel_tool_rows_by_tool_id() {
        let actor = ScrollbackActor::new();
        actor
            .apply_batch(vec![
                ScrollbackMsg::ToolStart {
                    tool_call_id: "a".into(),
                    header: "Read a.txt".into(),
                    activity: Some("◈ Read 1 file".into()),
                },
                ScrollbackMsg::ToolStart {
                    tool_call_id: "b".into(),
                    header: "Read b.txt".into(),
                    activity: Some("◈ Read 2 files".into()),
                },
                ScrollbackMsg::ToolUpdate {
                    tool_call_id: "a".into(),
                    header: Some("Read a.txt (1 lines)".into()),
                    output: vec!["a".into()],
                },
                ScrollbackMsg::ToolEnd {
                    tool_call_id: "b".into(),
                    header: "Read b.txt (1 lines)".into(),
                    activity: None,
                    output: vec![(LineKind::ToolOutput, "b".into())],
                },
            ])
            .await;
        let lines = actor.snapshot();
        assert_eq!(
            lines
                .lines()
                .iter()
                .find(|line| line.tool_call_id.as_deref() == Some("a"))
                .expect("tool a")
                .text,
            "Read a.txt (1 lines)"
        );
        assert_eq!(
            lines
                .lines()
                .iter()
                .find(|line| line.tool_call_id.as_deref() == Some("b"))
                .expect("tool b")
                .text,
            "Read b.txt (1 lines)"
        );
        assert!(lines.lines().iter().any(|line| line.text == "a"));
        assert!(lines.lines().iter().any(|line| line.text == "b"));
    }
}
