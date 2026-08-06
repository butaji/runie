//! Public `LoopActor` API.

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::events::{EventBus, SubscriberRegistry};
use crate::hooks::TurnHooks;
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::driver::{
    run_loop, ApiKeyResolver, ConvertToLlm, RunLoopDeps, RunLoopOutcome, TransformContext,
};
use crate::state::AgentStateActor;
use crate::task_owner::{spawn_owned_worker, TaskOwner};
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
    pub fn as_run_loop_deps(&self) -> RunLoopDeps {
        RunLoopDeps {
            state: self.state.clone(),
            steering: self.steering.clone(),
            follow_up: self.follow_up.clone(),
            tool_executor: self.tool_executor.clone(),
            provider: self.provider.clone(),
            bus: self.bus.clone(),
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
    steering_mode: Mutex<QueueMode>,
    follow_up_mode: Mutex<QueueMode>,
    current: Mutex<Option<JoinHandle<RunLoopOutcome>>>,
    /// True while a run is in flight; guards concurrent `prompt()` (pi's
    /// "Agent is already processing a prompt" rejection).
    running: Mutex<bool>,
    /// Abort channel sender (pi `Agent.abort()`).
    abort_tx: tokio::sync::watch::Sender<bool>,
    /// Owns the bus-to-registry dispatch task for this actor lifetime.
    _subscriber_bridge: Arc<TaskOwner>,
}

impl LoopActor {
    pub fn new(mut deps: LoopDeps) -> Self {
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
        deps.abort = Some(abort_rx);
        let steering_mode = deps.steering_mode;
        let follow_up_mode = deps.follow_up_mode;
        let subscriber_bridge = spawn_subscriber_bridge(&deps.bus, &deps.subscribers);
        Self {
            inner: Arc::new(Inner {
                deps,
                steering_mode: Mutex::new(steering_mode),
                follow_up_mode: Mutex::new(follow_up_mode),
                current: Mutex::new(None),
                running: Mutex::new(false),
                abort_tx,
                _subscriber_bridge: subscriber_bridge,
            }),
        }
    }

    pub async fn prompt(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        // Busy guard: only one run at a time (pi agent.ts:340).
        {
            let mut running = self.inner.running.lock().await;
            if *running {
                return Err(LoopError::Busy);
            }
            *running = true;
        }
        let result = self.run_inner(prompts, context, false).await;
        *self.inner.running.lock().await = false;
        result
    }

    async fn run_inner(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
        skip_initial_steering_poll: bool,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        self.sync_context_to_state(&context).await;
        let mut deps = self.inner.deps.as_run_loop_deps();
        deps.steering_mode = *self.inner.steering_mode.lock().await;
        deps.follow_up_mode = *self.inner.follow_up_mode.lock().await;
        // OWNER: LoopActor — handle stored in `current` for `wait_for_idle`.
        let handle = tokio::spawn(async move {
            run_loop(prompts, context, deps, skip_initial_steering_poll).await
        });
        *self.inner.current.lock().await = Some(handle);

        let outcome = {
            let handle = self
                .inner
                .current
                .lock()
                .await
                .take()
                .expect("run handle stored by prompt");
            handle
                .await
                .map_err(|e| LoopError::Internal(e.to_string()))?
        };
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
        self.acquire_run().await?;
        let result = self.continue_inner(context).await;
        self.release_run().await;
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

    async fn acquire_run(&self) -> Result<(), LoopError> {
        let mut running = self.inner.running.lock().await;
        if *running {
            return Err(LoopError::Busy);
        }
        *running = true;
        Ok(())
    }

    async fn release_run(&self) {
        *self.inner.running.lock().await = false;
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
        self.inner.deps.steering.push(msg).await;
    }

    pub async fn follow_up(&self, msg: AgentMessage) {
        self.inner.deps.follow_up.push(msg).await;
    }

    /// Remove all queued steering messages.
    pub async fn clear_steering_queue(&self) {
        self.inner.deps.steering.clear().await;
    }

    /// Remove all queued follow-up messages.
    pub async fn clear_follow_up_queue(&self) {
        self.inner.deps.follow_up.clear().await;
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
    pub async fn reset(&self) {
        self.inner
            .deps
            .state
            .publish_event(&self.inner.deps.bus, crate::types::AgentEvent::Reset)
            .await;
        self.clear_all_queues().await;
    }

    /// Apply an externally replayed event through the core state owner.
    ///
    /// This is intentionally an event boundary rather than a state mutator;
    /// YAML replay and adapters can feed control events into the same reducer
    /// without reaching into another actor's fields.
    pub async fn apply_event(&self, event: &AgentEvent) {
        self.inner.deps.state.apply_event(event).await;
    }

    /// Controls how steering messages are drained on subsequent turns.
    pub async fn set_steering_mode(&self, mode: QueueMode) {
        *self.inner.steering_mode.lock().await = mode;
    }

    pub async fn steering_mode(&self) -> QueueMode {
        *self.inner.steering_mode.lock().await
    }

    /// Controls how follow-up messages are drained on subsequent turns.
    pub async fn set_follow_up_mode(&self, mode: QueueMode) {
        *self.inner.follow_up_mode.lock().await = mode;
    }

    pub async fn follow_up_mode(&self) -> QueueMode {
        *self.inner.follow_up_mode.lock().await
    }

    pub fn abort(&self) {
        let _ = self.inner.abort_tx.send(true);
    }

    pub async fn wait_for_idle(&self) {
        let mut g = self.inner.current.lock().await;
        if let Some(handle) = g.take() {
            let _ = handle.await;
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
mod tests {
    use super::*;
    use crate::types::AgentEvent;

    struct DeliverySubscriber {
        delivered: Option<tokio::sync::oneshot::Sender<AgentEvent>>,
    }

    #[async_trait::async_trait]
    impl crate::events::Subscriber for DeliverySubscriber {
        async fn handle(&mut self, event: &crate::types::AgentEvent) {
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
}
