//! Public `LoopActor` API.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use tokio::sync::{mpsc, oneshot, watch, OwnedSemaphorePermit, Semaphore};

use crate::events::{EventBus, SubscriberRegistry};
use crate::hooks::TurnHooks;
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::driver::{
    run_loop, ApiKeyResolver, ConvertToLlm, RunLoopDeps, TransformContext,
};
use crate::state::AgentStateActor;
#[cfg(test)]
use crate::task_owner::spawn_owned_worker;
use crate::task_owner::{mailbox_ack, spawn_actor_worker, TaskOwner};
use crate::tools::executor::ToolExecHooks;
use crate::tools::ToolExecutorActor;
use crate::types::{AgentContext, AgentEvent, AgentMessage, QueueMode, ToolExecutionMode};

#[derive(Debug, thiserror::Error)]
pub enum LoopError {
    #[error("aborted")]
    Aborted,
    #[error("internal: {0}")]
    Internal(String),
    #[error("provider: {0}")]
    Provider(String),
    /// pi: `Agent is already processing a prompt. Use steer() or followUp()
    /// to queue messages, or wait for completion.` (agent.ts:340).
    #[error("Agent is already processing a prompt. Use steer() or followUp() to queue messages, or wait for completion.")]
    Busy,
    /// pi: `Cannot continue: no messages in context` (agent-loop.ts:127).
    #[error("Cannot continue: no messages in context")]
    EmptyContext,
    /// pi: `Cannot continue from message role: assistant` (agent-loop.ts:131).
    #[error("Cannot continue from message role: assistant")]
    LastIsAssistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopControlSnapshot {
    pub running: bool,
    pub abort_requested: bool,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopControlEvent {
    RunStarted,
    RunFinished,
    AbortRequested,
    AbortCleared,
    SteeringModeChanged(QueueMode),
    FollowUpModeChanged(QueueMode),
}

fn reduce_control(snapshot: &mut LoopControlSnapshot, event: LoopControlEvent) {
    match event {
        LoopControlEvent::RunStarted => {
            snapshot.running = true;
            snapshot.abort_requested = false;
        }
        LoopControlEvent::RunFinished => snapshot.running = false,
        LoopControlEvent::AbortRequested => snapshot.abort_requested = true,
        LoopControlEvent::AbortCleared => snapshot.abort_requested = false,
        LoopControlEvent::SteeringModeChanged(mode) => snapshot.steering_mode = mode,
        LoopControlEvent::FollowUpModeChanged(mode) => snapshot.follow_up_mode = mode,
    }
}

fn publish_queue_record(
    bus: &EventBus,
    kind: crate::types::OperationRecordKind,
    data: serde_json::Value,
) {
    bus.publish(AgentEvent::TypedOperationRecordCreated { kind, data });
}

enum LoopControlCommand {
    Reduce(LoopControlEvent, oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct LoopDeps {
    pub state: AgentStateActor,
    pub steering: SteeringQueueActor,
    pub follow_up: FollowUpQueueActor,
    pub tool_executor: ToolExecutorActor,
    pub provider: ProviderActor,
    pub bus: EventBus,
    pub subscribers: SubscriberRegistry,
    pub hooks: ToolExecHooks,
    pub turn_hooks: TurnHooks,
    pub transform_context: Option<TransformContext>,
    pub convert_to_llm: Option<ConvertToLlm>,
    pub api_key_resolver: Option<ApiKeyResolver>,
    pub stream_options: crate::types::SimpleStreamOptions,
    /// Abort signal receiver; `LoopActor::new` injects its own channel.
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    pub tool_execution_mode: ToolExecutionMode,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
}

impl LoopDeps {
    pub fn as_run_loop_deps(&self, run_id: String) -> RunLoopDeps {
        RunLoopDeps {
            run_id,
            state: self.state.clone(),
            steering: self.steering.clone(),
            follow_up: self.follow_up.clone(),
            tool_executor: self.tool_executor.clone(),
            provider: self.provider.clone(),
            bus: self.bus.clone(),
            subscribers: self.subscribers.clone(),
            hooks: self.hooks.clone(),
            turn_hooks: self.turn_hooks.clone(),
            transform_context: self.transform_context.clone(),
            convert_to_llm: self.convert_to_llm.clone(),
            api_key_resolver: self.api_key_resolver.clone(),
            stream_options: self.stream_options.clone(),
            abort: self.abort.clone(),
            tool_execution_mode: self.tool_execution_mode,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
        }
    }
}

#[derive(Clone)]
pub struct LoopActor {
    inner: Arc<Inner>,
}

struct Inner {
    deps: LoopDeps,
    next_run_id: AtomicU64,
    /// True while a run is in flight; guards concurrent `prompt()` (pi's
    /// "Agent is already processing a prompt" rejection).
    running: Arc<Semaphore>,
    /// Abort channel sender (pi `Agent.abort()`).
    control_commands: mpsc::Sender<LoopControlCommand>,
    control_rx: watch::Receiver<LoopControlSnapshot>,
    _control_owner: Arc<TaskOwner>,
}

#[path = "actor_runtime.rs"]
mod actor_runtime;

fn spawn_control_worker(
    initial_snapshot: LoopControlSnapshot,
    abort_tx: watch::Sender<bool>,
    control_tx: watch::Sender<LoopControlSnapshot>,
) -> (mpsc::Sender<LoopControlCommand>, Arc<TaskOwner>) {
    spawn_actor_worker!(
        32,
        |mut commands: mpsc::Receiver<LoopControlCommand>| async move {
            let mut snapshot = initial_snapshot;
            while let Some(LoopControlCommand::Reduce(event, reply)) = commands.recv().await {
                let abort_requested = matches!(event, LoopControlEvent::AbortRequested);
                if abort_requested
                    || matches!(
                        event,
                        LoopControlEvent::RunStarted | LoopControlEvent::AbortCleared
                    )
                {
                    let _ = abort_tx.send(abort_requested);
                }
                reduce_control(&mut snapshot, event);
                let _ = control_tx.send(snapshot.clone());
                let _ = reply.send(());
            }
        }
    )
}

fn provisioned_queue_target(entry_id: &str, mut target: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(fields) = &mut target {
        fields.insert("id".into(), serde_json::Value::String(entry_id.into()));
    }
    target
}

#[cfg(test)]
fn spawn_subscriber_bridge(bus: &EventBus, subscribers: &SubscriberRegistry) -> Arc<TaskOwner> {
    let mut events = bus.subscribe();
    let subscribers = subscribers.clone();
    // OWNER: LoopActor — retained in Inner and aborted with the actor.
    spawn_owned_worker!(async move {
        while let Ok(event) = events.recv().await {
            subscribers.dispatch(&event).await;
        }
    })
}

#[cfg(test)]
fn spawn_pi_subscriber_bridge(bus: &EventBus, subscribers: &SubscriberRegistry) -> Arc<TaskOwner> {
    let mut events = bus.subscribe_pi();
    let subscribers = subscribers.clone();
    // OWNER: LoopActor — retained in Inner and aborted with the actor.
    spawn_owned_worker!(async move {
        while let Ok(event) = events.recv().await {
            subscribers.dispatch_pi(&event).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentEvent;

    #[test]
    fn queue_record_kind_has_only_pi_wire_names() {
        assert_eq!(
            crate::types::OperationRecordKind::QueueEnqueued.wire_name(),
            "queue_enqueued"
        );
        assert_eq!(
            crate::types::OperationRecordKind::QueueCancelled.wire_name(),
            "queue_cancelled"
        );
    }

    #[test]
    fn queue_targets_carry_actor_provisioned_identity() {
        let target = provisioned_queue_target(
            "steer-1",
            serde_json::json!({"role": "user", "content": "steer this turn"}),
        );
        assert_eq!(target["id"], "steer-1");
        assert_eq!(target["role"], "user");
        assert_eq!(target["content"], "steer this turn");
    }

    #[test]
    fn control_events_reduce_to_one_snapshot() {
        let mut snapshot = LoopControlSnapshot {
            running: false,
            abort_requested: false,
            steering_mode: QueueMode::OneAtATime,
            follow_up_mode: QueueMode::OneAtATime,
        };
        reduce_control(
            &mut snapshot,
            LoopControlEvent::SteeringModeChanged(QueueMode::All),
        );
        reduce_control(
            &mut snapshot,
            LoopControlEvent::FollowUpModeChanged(QueueMode::All),
        );
        reduce_control(&mut snapshot, LoopControlEvent::RunStarted);
        reduce_control(&mut snapshot, LoopControlEvent::AbortRequested);
        assert_eq!(snapshot.steering_mode, QueueMode::All);
        assert_eq!(snapshot.follow_up_mode, QueueMode::All);
        assert!(snapshot.running);
        assert!(snapshot.abort_requested);
        reduce_control(&mut snapshot, LoopControlEvent::RunFinished);
        assert!(!snapshot.running);
    }

    #[test]
    fn context_recovery_plan_uses_loop_message_projection() {
        let messages = vec![crate::types::AgentMessage::User(
            crate::types::UserMessage {
                content: vec![crate::types::UserContent::Text {
                    text: "x".repeat(4_001),
                }],
                timestamp: 0,
            },
        )];
        let plan = LoopActor::context_recovery_plan(
            &messages,
            1_000,
            crate::session::CompactionSettings {
                enabled: true,
                reserve_tokens: 100,
                keep_recent_tokens: 20,
            },
        );
        assert_eq!(
            plan.action,
            crate::session::CompactionRecoveryAction::Prepare {
                keep_recent_tokens: 20
            }
        );
    }

    struct DeliverySubscriber {
        delivered: Option<tokio::sync::oneshot::Sender<AgentEvent>>,
    }

    struct PiDeliverySubscriber {
        delivered: Option<tokio::sync::oneshot::Sender<crate::PiAgentEvent>>,
    }

    #[async_trait::async_trait]
    impl crate::events::Subscriber for DeliverySubscriber {
        async fn handle(&mut self, event: &crate::types::AgentEvent) {
            if let Some(delivered) = self.delivered.take() {
                let _ = delivered.send(event.clone());
            }
        }
    }

    #[async_trait::async_trait]
    impl crate::events::PiSubscriber for PiDeliverySubscriber {
        async fn handle_pi(&mut self, event: &crate::PiAgentEvent) {
            if let Some(delivered) = self.delivered.take() {
                let _ = delivered.send(event.clone());
            }
        }
    }

    #[tokio::test]
    async fn new_loop_actor_is_constructed() {
        // Smoke test: ensure the builder doesn't panic. Full behaviour is
        // covered by the driver + integration tests.
        let _a = std::mem::size_of::<LoopActor>();
    }

    #[tokio::test]
    async fn subscriber_bridge_delivers_bus_events() {
        let bus = EventBus::new();
        let subscribers = SubscriberRegistry::new();
        let (tx, rx) = tokio::sync::oneshot::channel();
        subscribers
            .register(Box::new(DeliverySubscriber {
                delivered: Some(tx),
            }))
            .await;
        let _owner = spawn_subscriber_bridge(&bus, &subscribers);
        bus.publish(crate::types::AgentEvent::AgentStart);
        assert!(matches!(
            rx.await.unwrap(),
            crate::types::AgentEvent::AgentStart
        ));
    }

    #[tokio::test]
    async fn pi_subscriber_bridge_filters_compatibility_events() {
        let bus = EventBus::new();
        let subscribers = SubscriberRegistry::new();
        let (tx, rx) = tokio::sync::oneshot::channel();
        subscribers
            .register_pi(Box::new(PiDeliverySubscriber {
                delivered: Some(tx),
            }))
            .await;
        let _owner = spawn_pi_subscriber_bridge(&bus, &subscribers);
        bus.publish(AgentEvent::ThemeChanged {
            theme: crate::types::ThemeKind::GrokNight,
        });
        bus.publish_pi(crate::PiAgentEvent::TurnStart);
        assert!(matches!(rx.await.unwrap(), crate::PiAgentEvent::TurnStart));
    }
}
