//! Actor-owned transcript projection.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};

use runie_core::types::AgentEvent;
use runie_core::{
    mailbox_ack, spawn_actor_worker, spawn_owned_worker, task_owner::TaskOwner, EventMemo,
    SharedSnapshot,
};

use crate::widgets::{FeedSnapshot, Scrollback, ScrollbackMsg};
use runie_tui_model::FeedState;

use crate::scrollback_projection::{run_bus_projection, OwnedEventProjection};

pub(crate) enum Command {
    ApplyBatch(Vec<ScrollbackMsg>, oneshot::Sender<()>),
    ApplyEvent(Box<AgentEvent>, oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct ScrollbackActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<FeedSnapshot>,
    shared_snapshot: watch::Receiver<SharedSnapshot<FeedSnapshot>>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl ScrollbackActor {
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(FeedState::default().snapshot());
        let (shared_tx, shared_snapshot) =
            watch::channel(SharedSnapshot::new(FeedState::default().snapshot()));
        let (tx, owner) = spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Command>| async move {
            let mut memo = EventMemo::new(FeedState::default());
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
                    memo = memo.apply(message, |state, message| state.reduce(message.clone()));
                }
                let next_snapshot = memo.state().snapshot();
                let _ = snapshot_tx.send(next_snapshot.clone());
                let _ = shared_tx.send(SharedSnapshot::new(next_snapshot));
                let _ = reply.send(());
            }
        });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

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

impl Default for ScrollbackActor {
    fn default() -> Self {
        Self::new()
    }
}

include!("scrollback_shared.rs");

#[cfg(test)]
mod tests {
    use super::ScrollbackActor;
    use crate::widgets::{Line, LineKind, Scrollback, ScrollbackMsg};
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
        apply_parallel_tool_rows(&actor).await;
        let lines = actor.snapshot();
        assert_tool_text(&lines, "a", "Read a.txt (1 lines)");
        assert_tool_text(&lines, "b", "Read b.txt (1 lines)");
        assert!(lines.lines().iter().any(|line| line.text == "a"));
        assert!(lines.lines().iter().any(|line| line.text == "b"));
    }

    async fn apply_parallel_tool_rows(actor: &ScrollbackActor) {
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
    }

    fn assert_tool_text(lines: &Scrollback, id: &str, expected: &str) {
        assert_eq!(
            lines
                .lines()
                .iter()
                .find(|line| line.tool_call_id.as_deref() == Some(id))
                .expect("tool row")
                .text,
            expected
        );
    }
}
