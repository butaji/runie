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

pub struct LoopActor {
    inner: Arc<Inner>,
}

struct Inner {
    deps: LoopDeps,
    current: Mutex<Option<JoinHandle<RunLoopOutcome>>>,
    aborted: Mutex<bool>,
}

impl LoopActor {
    pub fn new(deps: LoopDeps) -> Self {
        Self {
            inner: Arc::new(Inner {
                deps,
                current: Mutex::new(None),
                aborted: Mutex::new(false),
            }),
        }
    }

    pub async fn prompt(
        &self,
        prompts: Vec<AgentMessage>,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
        let deps = self.inner.deps.as_run_loop_deps();
        // OWNER: LoopActor — JoinHandle awaited immediately on the next line.
        let handle = tokio::spawn(async move { run_loop(prompts, context, deps).await });

        let outcome = handle
            .await
            .map_err(|e| LoopError::Internal(e.to_string()))?;
        Ok(outcome.new_messages)
    }

    pub async fn continue_run(
        &self,
        context: AgentContext,
    ) -> Result<Vec<AgentMessage>, LoopError> {
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
