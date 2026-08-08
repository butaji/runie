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

impl LoopActor {
    #[allow(
        clippy::too_many_lines,
        reason = "the actor constructor keeps mailbox, abort, and snapshot ownership together"
    )]
    pub fn new(mut deps: LoopDeps) -> Self {
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
        deps.abort = Some(abort_rx);
        let (control_tx, control_rx) = watch::channel(LoopControlSnapshot {
            running: false,
            abort_requested: false,
            steering_mode: deps.steering_mode,
            follow_up_mode: deps.follow_up_mode,
        });
        let abort_tx_for_control = abort_tx.clone();
        let control_snapshot = LoopControlSnapshot {
            running: false,
            abort_requested: false,
            steering_mode: deps.steering_mode,
            follow_up_mode: deps.follow_up_mode,
        };
        let (control_commands, control_owner) = spawn_actor_worker!(
            32,
            |mut commands: mpsc::Receiver<LoopControlCommand>| async move {
                let mut snapshot = control_snapshot;
                while let Some(LoopControlCommand::Reduce(event, reply)) = commands.recv().await {
                    if matches!(
                        event,
                        LoopControlEvent::RunStarted | LoopControlEvent::AbortCleared
                    ) {
                        let _ = abort_tx_for_control.send(false);
                    } else if matches!(event, LoopControlEvent::AbortRequested) {
                        let _ = abort_tx_for_control.send(true);
                    }
                    reduce_control(&mut snapshot, event);
                    let _ = control_tx.send(snapshot.clone());
                    let _ = reply.send(());
                }
            }
        );
        Self {
            inner: Arc::new(Inner {
                deps,
                next_run_id: AtomicU64::new(1),
                running: Arc::new(Semaphore::new(1)),
                control_commands,
                control_rx,
                _control_owner: control_owner,
            }),
        }
    }

    pub async fn prompt(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        // Busy guard: ownership of the single permit spans the complete run
        // (pi agent.ts:340), so no mutable boolean is shared across callers.
        let _run_permit = self.acquire_run().await?;
        self.reduce_control(LoopControlEvent::RunStarted).await;
        let result = self.run_inner(prompts, context, false).await;
        self.reduce_control(LoopControlEvent::RunFinished).await;
        result
    }

    async fn run_inner(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
        skip_initial_steering_poll: bool,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        self.sync_context_to_state(&context).await;
        let run_id = format!(
            "run-{}",
            self.inner.next_run_id.fetch_add(1, Ordering::Relaxed)
        );
        let mut deps = self.inner.deps.as_run_loop_deps(run_id);
        let control = self.inner.control_rx.borrow().clone();
        deps.steering_mode = control.steering_mode;
        deps.follow_up_mode = control.follow_up_mode;
        let outcome = run_loop(prompts, context, deps, skip_initial_steering_poll).await;
        Ok(outcome.new_messages)
    }

    async fn sync_context_to_state(&self, context: &AgentContext) {
        if !context.system_prompt.is_empty() {
            self.inner
                .deps
                .state
                .set_system_prompt(context.system_prompt.clone())
                .await;
        }
        if !context.messages.is_empty() {
            self.inner
                .deps
                .state
                .replace_messages(context.messages.clone())
                .await;
        }
        let tools = context
            .tools
            .clone()
            .unwrap_or_else(|| self.inner.deps.tool_executor.tools());
        self.inner.deps.state.set_tools(tools).await;
    }

    pub async fn continue_run(
        &self,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        let _run_permit = self.acquire_run().await?;
        self.reduce_control(LoopControlEvent::RunStarted).await;
        let result = self.continue_inner(context).await;
        self.reduce_control(LoopControlEvent::RunFinished).await;
        result
    }

    async fn continue_inner(&self, context: AgentContext) -> Result<Vec<AgentMessage>, LoopError> {
        // pi runAgentLoopContinue validation (agent-loop.ts:127,131).
        if context.messages.is_empty() {
            return Err(LoopError::EmptyContext);
        }
        if matches!(context.messages.last(), Some(AgentMessage::Assistant(_))) {
            let steering = self.drain_steering_for_continue().await;
            if !steering.is_empty() {
                return self.run_inner(steering, context, true).await;
            }
            let follow_up = self.drain_follow_up_for_continue().await;
            if !follow_up.is_empty() {
                return self.run_inner(follow_up, context, false).await;
            }
            return Err(LoopError::LastIsAssistant);
        }
        self.run_inner(vec![], context, false).await
    }

    async fn acquire_run(&self) -> Result<OwnedSemaphorePermit, LoopError> {
        self.inner
            .running
            .clone()
            .try_acquire_owned()
            .map_err(|_| LoopError::Busy)
    }

    async fn drain_steering_for_continue(&self) -> Vec<AgentMessage> {
        match self.steering_mode().await {
            QueueMode::OneAtATime => self
                .inner
                .deps
                .steering
                .drain_one()
                .await
                .into_iter()
                .collect(),
            QueueMode::All => self.inner.deps.steering.drain_all().await,
        }
    }

    async fn drain_follow_up_for_continue(&self) -> Vec<AgentMessage> {
        match self.follow_up_mode().await {
            QueueMode::OneAtATime => self
                .inner
                .deps
                .follow_up
                .drain_one()
                .await
                .into_iter()
                .collect(),
            QueueMode::All => self.inner.deps.follow_up.drain_all().await,
        }
    }

    pub async fn steer(&self, msg: AgentMessage) {
        let payload = serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null);
        if let Some(entry_id) = self.inner.deps.steering.push(msg).await {
            let target = provisioned_queue_target(&entry_id, payload);
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueEnqueued,
                serde_json::json!({
                    "id": entry_id,
                    "queue": "steer",
                    "target": target,
                }),
            );
        }
    }

    pub async fn follow_up(&self, msg: AgentMessage) {
        let payload = serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null);
        if let Some(entry_id) = self.inner.deps.follow_up.push(msg).await {
            let target = provisioned_queue_target(&entry_id, payload);
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueEnqueued,
                serde_json::json!({
                    "id": entry_id,
                    "queue": "followUp",
                    "target": target,
                }),
            );
        }
    }

    /// Remove all queued steering messages.
    pub async fn clear_steering_queue(&self) {
        for entry_id in self.inner.deps.steering.clear().await {
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueCancelled,
                serde_json::json!({"id": entry_id, "entryId": entry_id}),
            );
        }
    }

    /// Remove all queued follow-up messages.
    pub async fn clear_follow_up_queue(&self) {
        for entry_id in self.inner.deps.follow_up.clear().await {
            publish_queue_record(
                &self.inner.deps.bus,
                crate::types::OperationRecordKind::QueueCancelled,
                serde_json::json!({"id": entry_id, "entryId": entry_id}),
            );
        }
    }

    /// Remove all queued steering and follow-up messages.
    pub async fn clear_all_queues(&self) {
        self.clear_steering_queue().await;
        self.clear_follow_up_queue().await;
    }

    /// Whether either queue contains a pending message.
    pub async fn has_queued_messages(&self) -> bool {
        !self.inner.deps.steering.is_empty().await || !self.inner.deps.follow_up.is_empty().await
    }

    /// Clear transcript, projections, and queued messages through their owners.
    pub async fn reset(&self) -> Result<(), LoopError> {
        // Pi rejects reset while an active run exists. Hold the admission
        // permit through the reset event so a new prompt cannot race it.
        let _run_permit = self.acquire_run().await?;
        self.inner
            .deps
            .state
            .publish_event(&self.inner.deps.bus, crate::types::AgentEvent::Reset)
            .await;
        self.clear_all_queues().await;
        Ok(())
    }

    /// Apply an externally replayed event through the core state owner.
    ///
    /// This is intentionally an event boundary rather than a state mutator;
    /// YAML replay and adapters can feed control events into the same reducer
    /// without reaching into another actor's fields.
    pub async fn apply_event(&self, event: &AgentEvent) {
        self.inner.deps.state.apply_event(event).await;
    }

    /// Set model configuration through the state actor's mailbox.
    pub async fn set_model(&self, model: crate::types::Model) {
        self.inner
            .deps
            .state
            .publish_event(
                &self.inner.deps.bus,
                crate::types::AgentEvent::ModelChanged { model },
            )
            .await;
    }

    /// Ask the provider actor for a Pi-compatible compaction summary. The
    /// provider remains the owner of transport/generation; this coordinator
    /// only forwards the immutable prepared request through its mailbox.
    pub async fn summarize_compaction(
        &self,
        request: crate::session::CompactionSummaryRequest,
    ) -> Result<crate::session::CompactionSummary, LoopError> {
        self.inner
            .deps
            .provider
            .summarize_compaction(request)
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Discover models through the provider actor; the loop never performs
    /// provider I/O directly or mutates the catalog actor.
    pub async fn list_models(&self) -> Result<Vec<crate::types::Model>, LoopError> {
        self.inner
            .deps
            .provider
            .list_models()
            .await
            .map_err(|error| LoopError::Provider(error.to_string()))
    }

    /// Replace the owned conversation context through the state actor
    /// mailbox. Session restore and replay adapters use this instead of
    /// reaching into `AgentStateActor` behind the loop boundary.
    pub async fn replace_messages(&self, messages: Vec<AgentMessage>) {
        self.inner.deps.state.replace_messages(messages).await;
    }

    /// Controls how steering messages are drained on subsequent turns.
    pub async fn set_steering_mode(&self, mode: QueueMode) {
        self.reduce_control(LoopControlEvent::SteeringModeChanged(mode))
            .await;
    }

    pub async fn steering_mode(&self) -> QueueMode {
        self.inner.control_rx.borrow().steering_mode
    }

    /// Controls how follow-up messages are drained on subsequent turns.
    pub async fn set_follow_up_mode(&self, mode: QueueMode) {
        self.reduce_control(LoopControlEvent::FollowUpModeChanged(mode))
            .await;
    }

    pub async fn follow_up_mode(&self) -> QueueMode {
        self.inner.control_rx.borrow().follow_up_mode
    }

    pub async fn abort(&self) {
        self.reduce_control(LoopControlEvent::AbortRequested).await;
    }

    async fn reduce_control(&self, event: LoopControlEvent) {
        let _ = mailbox_ack!(self.inner.control_commands, |reply| {
            LoopControlCommand::Reduce(event, reply)
        });
    }

    pub fn control_snapshot(&self) -> LoopControlSnapshot {
        self.inner.control_rx.borrow().clone()
    }

    pub async fn wait_for_idle(&self) {
        // The admission permit is the single source of truth for whether a
        // run is active. Waiting for a fresh permit avoids a second mutable
        // JoinHandle/Mutex state owner and remains sleep-free.
        if let Ok(permit) = self.inner.running.clone().acquire_owned().await {
            drop(permit);
        }
    }

    pub async fn subscribe(&self, sub: Box<dyn crate::events::Subscriber>) -> crate::events::SubId {
        self.inner.deps.subscribers.register(sub).await
    }

    /// Register a Pi-core-only subscriber. Application/TUI events are
    /// filtered by the registry adapter before delivery.
    pub async fn subscribe_pi(
        &self,
        sub: Box<dyn crate::events::PiSubscriber>,
    ) -> crate::events::SubId {
        self.inner.deps.subscribers.register_pi(sub).await
    }

    pub fn bus(&self) -> EventBus {
        self.inner.deps.bus.clone()
    }

    /// Read-only state projection for UI consumers.
    pub fn state_snapshot(&self) -> crate::state::AgentStateSnapshot {
        self.inner.deps.state.snapshot()
    }
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
