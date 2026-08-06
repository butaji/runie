//! Actor-owned transcript projection.

use std::sync::Arc;

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
        let mut events = bus.subscribe();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        let messages = match event {
                            AgentEvent::Reset => vec![ScrollbackMsg::Clear],
                            AgentEvent::ThemeChanged { theme } => {
                                vec![ScrollbackMsg::SetTheme(theme)]
                            }
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
                            _ => Vec::new(),
                        };
                        if !messages.is_empty()
                            && !mailbox_ack!(tx, |reply| Command::ApplyBatch(messages, reply))
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
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

fn default_tool_display_mode(tool_name: &str) -> runie_core::types::ToolDisplayMode {
    if matches!(tool_name, "bash" | "shell" | "exec" | "run") {
        runie_core::types::ToolDisplayMode::Truncated
    } else {
        runie_core::types::ToolDisplayMode::Collapsed
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
