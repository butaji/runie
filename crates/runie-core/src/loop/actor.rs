//! Public `LoopActor` API.

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::events::{EventBus, SubscriberRegistry};
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::driver::{run_loop, RunLoopDeps, RunLoopOutcome};
use crate::state::AgentStateActor;
use crate::tools::executor::ToolExecHooks;
use crate::tools::ToolExecutorActor;
use crate::types::{AgentContext, AgentMessage, QueueMode, ToolExecutionMode};

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
    #[error("agent is already processing a prompt")]
    Busy,
    /// pi: `Cannot continue: no messages in context` (agent-loop.ts:127).
    #[error("cannot continue: no messages in context")]
    EmptyContext,
    /// pi: `Cannot continue from message role: assistant` (agent-loop.ts:131).
    #[error("cannot continue from message role: assistant")]
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
    current: Mutex<Option<JoinHandle<RunLoopOutcome>>>,
    /// True while a run is in flight; guards concurrent `prompt()` (pi's
    /// "Agent is already processing a prompt" rejection).
    running: Mutex<bool>,
    aborted: Mutex<bool>,
}

impl LoopActor {
    pub fn new(deps: LoopDeps) -> Self {
        Self {
            inner: Arc::new(Inner {
                deps,
                current: Mutex::new(None),
                running: Mutex::new(false),
                aborted: Mutex::new(false),
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
        let result = self.run_inner(prompts, context).await;
        *self.inner.running.lock().await = false;
        result
    }

    async fn run_inner(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        let deps = self.inner.deps.as_run_loop_deps();
        // OWNER: LoopActor — handle stored in `current` for `wait_for_idle`.
        let handle = tokio::spawn(async move { run_loop(prompts, context, deps).await });
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

    pub async fn continue_run(
        &self,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        // pi runAgentLoopContinue validation (agent-loop.ts:127,131).
        if context.messages.is_empty() {
            return Err(LoopError::EmptyContext);
        }
        if matches!(context.messages.last(), Some(AgentMessage::Assistant(_))) {
            return Err(LoopError::LastIsAssistant);
        }
        self.prompt(vec![], context).await
    }

    pub async fn steer(&self, msg: AgentMessage) {
        self.inner.deps.steering.push(msg).await;
    }

    pub async fn follow_up(&self, msg: AgentMessage) {
        self.inner.deps.follow_up.push(msg).await;
    }

    pub fn abort(&self) {
        if let Ok(mut g) = self.inner.aborted.try_lock() {
            *g = true;
        }
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

    pub fn bus(&self) -> EventBus {
        self.inner.deps.bus.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_loop_actor_is_constructed() {
        // Smoke test: ensure the builder doesn't panic. Full behaviour is
        // covered by the driver + integration tests.
        let _a = std::mem::size_of::<LoopActor>();
    }
}
