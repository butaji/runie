//! Actor-owned transcript projection.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{mpsc, oneshot, watch};

use runie_core::types::AgentEvent;
use runie_core::{mailbox_ack, spawn_actor_worker, spawn_owned_worker, task_owner::TaskOwner};

use crate::widgets::{FeedSnapshot, Line, LineKind, Scrollback, ScrollbackMsg};
use runie_tui_model::FeedState;

enum Command {
    ApplyBatch(Vec<ScrollbackMsg>, oneshot::Sender<()>),
    ApplyEvent(Box<AgentEvent>, oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct ScrollbackActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<FeedSnapshot>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl ScrollbackActor {
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(FeedState::default().snapshot());
        let (tx, owner) = spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Command>| async move {
            let mut state = FeedState::default();
            // The reducer remains async/non-blocking: the unavoidable process
            // cwd query is isolated behind an awaited, actor-owned blocking
            // boundary before the projection starts consuming events.
            let workspace = tokio::task::spawn_blocking(|| {
                std::env::current_dir()
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .await
            .unwrap_or_default();
            let mut projection = OwnedEventProjection::new(workspace);
            while let Some(command) = rx.recv().await {
                let (messages, reply) = match command {
                    Command::ApplyBatch(messages, reply) => (messages, reply),
                    Command::ApplyEvent(event, reply) => {
                        let messages = projection.messages(*event);
                        (messages, reply)
                    }
                };
                for message in messages {
                    state.reduce(message);
                }
                let _ = snapshot_tx.send(state.snapshot());
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

    /// Non-blocking render-time delivery for measurements. Rendering must not
    /// await an actor, but the measurement still crosses the same mailbox
    /// boundary instead of mutating a widget-owned model.
    pub fn try_apply(&self, message: ScrollbackMsg) -> bool {
        let (reply, _receiver) = oneshot::channel();
        self.tx
            .try_send(Command::ApplyBatch(vec![message], reply))
            .is_ok()
    }

    pub async fn apply_event(&self, event: &AgentEvent) {
        let _ = mailbox_ack!(self.tx, |reply| Command::ApplyEvent(
            Box::new(event.clone()),
            reply
        ));
    }

    pub fn snapshot(&self) -> Scrollback {
        Scrollback::from_model_snapshot(self.snapshot.borrow().clone())
    }

    pub fn model_snapshot(&self) -> FeedSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<FeedSnapshot> {
        self.snapshot.clone()
    }
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "the bus projection keeps event ordering and actor ownership in one loop"
)]
async fn run_bus_projection(
    mut events: tokio::sync::broadcast::Receiver<AgentEvent>,
    tx: mpsc::Sender<Command>,
) {
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                let (reply, _ack) = oneshot::channel();
                if tx
                    .send(Command::ApplyBatch(
                        vec![ScrollbackMsg::Append(Line::new(
                            LineKind::System,
                            format!("event stream lagged ({count} events)"),
                        ))],
                        reply,
                    ))
                    .await
                    .is_err()
                {
                    break;
                }
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        };
        let (reply, _ack) = oneshot::channel();
        if tx
            .send(Command::ApplyEvent(Box::new(event), reply))
            .await
            .is_err()
        {
            break;
        }
    }
}

/// The bus bridge is deliberately stateless. Tool identity, lifecycle, and
/// activity counters belong to the ScrollbackActor reducer, so every event
/// changes one SSOT and the bridge cannot diverge from direct commands.
#[derive(Default)]
struct OwnedEventProjection {
    workspace: String,
    active_tools: HashSet<String>,
    tool_headers: HashMap<String, String>,
    tool_args: HashMap<String, serde_json::Value>,
    tool_names: HashMap<String, String>,
    active_tool_count: usize,
    activity_failures: usize,
    activity_dirs: usize,
    activity_files: usize,
    activity_commands: usize,
    activity_subagents: usize,
}

impl OwnedEventProjection {
    fn new(workspace: String) -> Self {
        Self {
            workspace,
            ..Self::default()
        }
    }

    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn messages(&mut self, event: AgentEvent) -> Vec<ScrollbackMsg> {
        if let AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } = &event
        {
            self.active_tools.insert(tool_call_id.clone());
            self.tool_headers.insert(
                tool_call_id.clone(),
                runie_tui_model::tool_header(tool_name, args, &self.workspace),
            );
            self.tool_args.insert(tool_call_id.clone(), args.clone());
            self.tool_names
                .insert(tool_call_id.clone(), tool_name.clone());
            match runie_tui_model::classify_activity_tool(tool_name) {
                Some(runie_tui_model::ActivityKind::Dir) => self.activity_dirs += 1,
                Some(runie_tui_model::ActivityKind::File) => self.activity_files += 1,
                Some(runie_tui_model::ActivityKind::Command) => self.activity_commands += 1,
                Some(runie_tui_model::ActivityKind::Subagent) => self.activity_subagents += 1,
                None => {}
            }
            self.active_tool_count += 1;
        }
        let completion = ordinary_tool_end_messages(
            &mut self.active_tools,
            &mut self.tool_headers,
            &mut self.tool_args,
            &mut self.tool_names,
            &mut self.active_tool_count,
            &mut self.activity_failures,
            (
                self.activity_dirs,
                self.activity_files,
                self.activity_commands,
                self.activity_subagents,
            ),
            &event,
        );
        if !completion.is_empty() {
            completion
        } else {
            let messages = bus_messages_for_event(event.clone());
            if messages.is_empty() {
                tool_update_messages(&self.active_tools, &mut self.tool_headers, &event)
            } else {
                messages
            }
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the exhaustive event-to-feed table keeps application no-ops explicit"
)]
fn bus_messages_for_event(event: AgentEvent) -> Vec<ScrollbackMsg> {
    if !runie_tui_model::is_actor_feed_event(&event) {
        return Vec::new();
    }
    match event {
        AgentEvent::Reset => vec![ScrollbackMsg::Clear],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(theme)],
        AgentEvent::ModelChanged { .. } => Vec::new(),
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id, mode)]
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            ..
        } => vec![
            ScrollbackMsg::SetToolName(tool_call_id.clone(), tool_name.clone()),
            ScrollbackMsg::SetToolMode(
                tool_call_id,
                runie_tui_model::default_tool_display_mode(&tool_name),
            ),
        ],
        event @ (AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }) => background_messages_for_event(event),
        AgentEvent::AgentStart
        | AgentEvent::AgentEnd { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageUpdate { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionUpdate { .. }
        | AgentEvent::ToolExecutionEnd { .. }
        | AgentEvent::WorkflowStarted { .. }
        | AgentEvent::WorkflowProgress { .. }
        | AgentEvent::WorkflowFinished { .. } => Vec::new(),
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
    let Some(output) = runie_tui_model::structured_update_text(partial_result) else {
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
        || runie_tui_model::is_transport_only_update(partial_result)
    {
        return Vec::new();
    }
    let Some(header) = tool_headers.get_mut(tool_call_id) else {
        return Vec::new();
    };
    *header = runie_tui_model::tool_update_header_text(header, partial_result);
    vec![ScrollbackMsg::ToolUpdate {
        tool_call_id: tool_call_id.clone(),
        header: Some(header.clone()),
        output: Vec::new(),
    }]
}

#[allow(
    clippy::too_many_lines,
    reason = "the pure completion fold keeps card, output, and activity variants atomic"
)]
#[allow(clippy::too_many_arguments)]
fn ordinary_tool_end_messages(
    active_tools: &mut HashSet<String>,
    tool_headers: &mut HashMap<String, String>,
    tool_args: &mut HashMap<String, serde_json::Value>,
    tool_names: &mut HashMap<String, String>,
    active_tool_count: &mut usize,
    activity_failures: &mut usize,
    (dirs, files, commands, subagents): (usize, usize, usize, usize),
    event: &AgentEvent,
) -> Vec<ScrollbackMsg> {
    let AgentEvent::ToolExecutionEnd {
        tool_call_id,
        tool_name,
        result,
        is_error,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !active_tools.remove(tool_call_id) {
        return Vec::new();
    }
    *active_tool_count = active_tool_count.saturating_sub(1);
    let pending = tool_headers.remove(tool_call_id).unwrap_or_default();
    let args = tool_args.remove(tool_call_id).unwrap_or_default();
    let name = tool_names
        .remove(tool_call_id)
        .unwrap_or_else(|| tool_name.clone());
    let header = if *is_error {
        *activity_failures += 1;
        pending
    } else {
        runie_tui_model::completed_tool_header_with_args(&pending, &name, &args, result)
    };
    let activity = if *active_tool_count == 0 && dirs + files + commands + subagents > 0 {
        Some(runie_tui_model::activity_text(
            dirs,
            files,
            commands,
            subagents,
            *activity_failures,
            false,
        ))
    } else {
        None
    };
    let output_kind = if runie_tui_model::is_output_tool(&name) {
        LineKind::ToolOutput
    } else {
        LineKind::ToolResult
    };
    let result_text = runie_tui_model::tool_result_text(result);
    let mut output = result_text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| (output_kind, line.to_owned()))
        .collect::<Vec<_>>();
    if !*is_error && matches!(name.as_str(), "web_search" | "web-search") {
        if let Some(sources) = runie_tui_model::web_search_sources_line(&result_text) {
            output.push((LineKind::ToolResult, sources));
        }
    }
    let mut messages = vec![ScrollbackMsg::ToolEnd {
        tool_call_id: tool_call_id.clone(),
        header,
        activity,
        output,
    }];
    if *is_error {
        messages.push(ScrollbackMsg::MarkToolError(tool_call_id.clone()));
    }
    messages
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
        } => vec![
            ScrollbackMsg::SetToolName(work_id.clone(), "subagent".into()),
            ScrollbackMsg::SetToolMode(
                work_id.clone(),
                runie_core::types::ToolDisplayMode::Collapsed,
            ),
            ScrollbackMsg::ToolStart {
                tool_call_id: work_id,
                header: format!(
                    "Subagent {}: {description:?}",
                    if background { "started" } else { "running" }
                ),
                activity: None,
            },
        ],
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
                    runie_tui_model::format_elapsed(elapsed_ms),
                    runie_tui_model::format_error(is_error, error.as_deref())
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
                    runie_tui_model::format_elapsed(elapsed_ms)
                ),
                activity: None,
                output: Vec::new(),
            },
            ScrollbackMsg::MarkToolError(work_id),
        ],
        AgentEvent::WorkflowStarted {
            run_id,
            name,
            objective,
        } => vec![
            ScrollbackMsg::SetToolName(run_id.clone(), "workflow".into()),
            ScrollbackMsg::WorkflowStart {
                run_id,
                name,
                objective,
            },
        ],
        AgentEvent::WorkflowProgress {
            run_id,
            phase,
            state,
            active_agents,
        } => vec![ScrollbackMsg::WorkflowProgress {
            run_id,
            phase,
            state,
            active_agents,
        }],
        AgentEvent::WorkflowFinished {
            run_id,
            status,
            elapsed_ms,
        } => vec![ScrollbackMsg::WorkflowEnd {
            run_id,
            status,
            elapsed_ms,
        }],
        AgentEvent::AgentStart
        | AgentEvent::AgentEnd { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::Reset
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::ThemeChanged { .. }
        | AgentEvent::ModelChanged { .. }
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::ToolDisplayModeChanged { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageUpdate { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionStart { .. }
        | AgentEvent::ToolExecutionUpdate { .. }
        | AgentEvent::ToolExecutionEnd { .. } => Vec::new(),
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
    async fn actor_publishes_renderer_independent_model_snapshot() {
        let actor = ScrollbackActor::new();
        actor
            .apply(ScrollbackMsg::Append(Line::new(LineKind::User, "hello")))
            .await;
        let snapshot = actor.model_snapshot();
        assert_eq!(snapshot.lines.len(), 1);
        assert_eq!(snapshot.lines[0].text, "hello");
        assert!(!snapshot.is_empty());
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
    async fn bus_owned_actor_does_not_replay_transcript_messages() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::User(runie_core::types::UserMessage {
                content: vec![runie_core::types::UserContent::Text {
                    text: "duplicate me".into(),
                }],
                timestamp: 0,
            }),
        });
        bus.publish(AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::GrokDay,
        });
        snapshot
            .changed()
            .await
            .expect("actor-scoped event projection");
        assert!(actor.snapshot().lines().is_empty());
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
        assert_eq!(
            actor.snapshot().tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Collapsed
        );
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
    async fn bus_owned_actor_reduces_tool_completion_and_activity() {
        let bus = runie_core::events::EventBus::new();
        let actor = ScrollbackActor::new_with_bus(&bus);
        actor
            .apply(ScrollbackMsg::ToolStart {
                tool_call_id: "tool-3".into(),
                header: "Read README.md".into(),
                activity: None,
            })
            .await;
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-3".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        });
        snapshot.changed().await.expect("tool start projection");
        bus.publish(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool-3".into(),
            tool_name: "read".into(),
            result: serde_json::json!({"output": "one\ntwo"}),
            is_error: false,
        });
        snapshot
            .changed()
            .await
            .expect("tool completion projection");
        let snapshot = actor.snapshot();
        let lines = snapshot.lines();
        assert!(lines
            .iter()
            .any(|line| line.text == "Read README.md (2 lines)"));
        assert!(lines.iter().any(|line| line.text == "one"));
        assert!(lines.iter().any(|line| line.text == "◈ Read 1 file"));
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
    async fn actor_appends_canonical_sources_row_for_successful_web_search() {
        let actor = ScrollbackActor::new();
        actor
            .apply_event(&AgentEvent::ToolExecutionStart {
                tool_call_id: "web-1".into(),
                tool_name: "web_search".into(),
                args: serde_json::json!({"query": "runie"}),
            })
            .await;
        actor
            .apply_event(&AgentEvent::ToolExecutionEnd {
                tool_call_id: "web-1".into(),
                tool_name: "web_search".into(),
                result: serde_json::json!({
                    "output": "https://docs.rs/runie https://docs.rs/ratatui https://rust-lang.org/learn https://github.com/runie"
                }),
                is_error: false,
            })
            .await;
        let lines = actor.snapshot();
        assert!(
            lines
                .lines()
                .iter()
                .any(|line| line.text == "  Sources: docs.rs, rust-lang.org, github.com"),
            "expected canonical Sources row, got {lines:?}"
        );
    }

    #[tokio::test]
    async fn actor_skips_sources_row_for_failed_web_search() {
        let actor = ScrollbackActor::new();
        actor
            .apply_event(&AgentEvent::ToolExecutionStart {
                tool_call_id: "web-err".into(),
                tool_name: "web_search".into(),
                args: serde_json::json!({"query": "runie"}),
            })
            .await;
        actor
            .apply_event(&AgentEvent::ToolExecutionEnd {
                tool_call_id: "web-err".into(),
                tool_name: "web_search".into(),
                result: serde_json::json!({"error": "denied"}),
                is_error: true,
            })
            .await;
        let lines = actor.snapshot();
        assert!(
            !lines
                .lines()
                .iter()
                .any(|line| line.text.starts_with("  Sources:")),
            "failed web searches must not emit a Sources row, got {lines:?}"
        );
    }

    #[tokio::test]
    async fn bus_owned_actor_appends_system_row_on_lag() {
        use runie_core::events::bus::{EventBus, BUS_CAPACITY};

        let bus = EventBus::new();
        // The keepalive subscriber keeps at least one receiver attached so
        // `publish` cannot drop events with no receivers and the actor's
        // subscription is forced to lag behind the broadcast tail.
        let _keepalive = bus.subscribe();
        let actor = ScrollbackActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();

        // Overflow the broadcast ring buffer so the actor's bus bridge sees
        // `RecvError::Lagged` on its next `recv`.
        for _ in 0..BUS_CAPACITY + 1 {
            bus.publish(AgentEvent::TurnStart);
        }

        // The bus bridge forwards the lag as a `LineKind::System` row before
        // it processes any post-lag tail event, so the first snapshot change
        // after `publish` carries the lag diagnostic row.
        snapshot.changed().await.expect("scrollback actor alive");
        let lines = actor.snapshot();
        assert!(
            lines.lines().iter().any(|line| {
                line.kind == LineKind::System && line.text.contains("event stream lagged")
            }),
            "expected a LineKind::System row containing 'event stream lagged', got {lines:?}"
        );
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
