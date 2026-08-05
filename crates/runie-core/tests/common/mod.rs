//! Shared test harness: `MockStreamFn`, `TestLoop` builder, event recorder.

use std::sync::Arc;

use futures::stream;
use parking_lot::Mutex;

use runie_core::events::EventBus;
use runie_core::hooks::TurnHooks;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::driver::{run_loop, RunLoopDeps};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::ToolExecutorActor;
use runie_core::tools::ToolRegistry;
use runie_core::types::{
    AgentContext, AgentEvent, AgentMessage, AssistantMessageEvent, Model, QueueMode, StopReason,
    ToolExecutionMode, Usage,
};

/// `StreamFn` impl that replays a fixed sequence of `AssistantMessageEvent`s.
pub struct MockStreamFn {
    pub events: Vec<AssistantMessageEvent>,
}

impl MockStreamFn {
    pub fn new(events: Vec<AssistantMessageEvent>) -> Self {
        Self { events }
    }

    pub fn hello() -> Self {
        Self::new(vec![
            AssistantMessageEvent::Start,
            AssistantMessageEvent::TextDelta {
                delta: "Hello".into(),
            },
            AssistantMessageEvent::TextDelta {
                delta: " world".into(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
            },
        ])
    }
}

#[async_trait::async_trait]
impl StreamFn for MockStreamFn {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<runie_core::types::SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let s = stream::iter(self.events.clone());
        Ok(Box::pin(s))
    }
}

/// Records bus events into a shared `Vec` for assertions.
pub struct EventRecorder {
    pub events: Arc<Mutex<Vec<AgentEvent>>>,
}

pub struct TestLoop {
    pub actor: LoopActor,
    pub events: Arc<Mutex<Vec<AgentEvent>>>,
    pub state: AgentStateActor,
    pub steering: SteeringQueueActor,
    pub follow_up: FollowUpQueueActor,
    pub tool_executor: ToolExecutorActor,
}

pub struct TestLoopBuilder {
    pub stream_fn: Arc<dyn StreamFn>,
    pub tools: Vec<Arc<dyn runie_core::types::AgentTool>>,
    pub hooks: ToolExecHooks,
    pub turn_hooks: TurnHooks,
    pub transform_context: Option<
        Arc<
            dyn Fn(Vec<AgentMessage>) -> futures::future::BoxFuture<'static, Vec<AgentMessage>>
                + Send
                + Sync,
        >,
    >,
    pub tool_execution: ToolExecutionMode,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
}

impl TestLoopBuilder {
    pub fn new(stream_fn: Arc<dyn StreamFn>) -> Self {
        Self {
            stream_fn,
            tools: vec![],
            hooks: ToolExecHooks::default(),
            turn_hooks: TurnHooks::default(),
            transform_context: None,
            tool_execution: ToolExecutionMode::Parallel,
            steering_mode: QueueMode::OneAtATime,
            follow_up_mode: QueueMode::OneAtATime,
        }
    }

    pub fn tool(mut self, t: Arc<dyn runie_core::types::AgentTool>) -> Self {
        self.tools.push(t);
        self
    }

    pub fn turn_hooks(mut self, hooks: TurnHooks) -> Self {
        self.turn_hooks = hooks;
        self
    }

    pub fn transform_context(
        mut self,
        f: impl Fn(Vec<AgentMessage>) -> futures::future::BoxFuture<'static, Vec<AgentMessage>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.transform_context = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> TestLoop {
        let state = AgentStateActor::new();
        let steering = SteeringQueueActor::new();
        let follow_up = FollowUpQueueActor::new();
        let mut registry = ToolRegistry::new();
        for t in self.tools {
            registry.register(t);
        }
        let registry = Arc::new(registry);
        let tool_executor = ToolExecutorActor::new(registry);
        let provider = ProviderActor::new(self.stream_fn);
        let bus = EventBus::new();

        let deps = LoopDeps {
            state: state.clone(),
            steering: steering.clone(),
            follow_up: follow_up.clone(),
            tool_executor: tool_executor.clone(),
            provider,
            bus: bus.clone(),
            subscribers: runie_core::events::SubscriberRegistry::new(),
            hooks: self.hooks,
            turn_hooks: self.turn_hooks,
            transform_context: self.transform_context,
            tool_execution_mode: self.tool_execution,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
        };
        let actor = LoopActor::new(deps);

        // Subscribe a recording task that drains the bus.
        let events: Arc<Mutex<Vec<AgentEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let mut rx = bus.subscribe();
        let events_clone = events.clone();
        // OWNER: EventRecorder — drives the recording loop.
        tokio::spawn(async move {
            while let Ok(ev) = rx.recv().await {
                events_clone.lock().push(ev);
            }
        });

        TestLoop {
            actor,
            events,
            state,
            steering,
            follow_up,
            tool_executor,
        }
    }
}

#[allow(dead_code)]
pub fn default_model() -> Model {
    Model {
        id: "test-model".into(),
        name: "test".into(),
        api: "test".into(),
        provider: "test".into(),
        base_url: String::new(),
        reasoning: false,
        context_window: 0,
        max_tokens: 0,
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn echo_tool() -> Arc<dyn runie_core::types::AgentTool> {
    use runie_core::types::{AgentTool, AgentToolResult, ToolResultContent};
    struct EchoTool;
    #[async_trait::async_trait]
    impl AgentTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn label(&self) -> &str {
            "Echo"
        }
        fn description(&self) -> &str {
            "Echoes input."
        }
        async fn execute(
            &self,
            _id: &str,
            args: serde_json::Value,
            _signal: Option<tokio_util::sync::CancellationToken>,
            _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
        ) -> Result<AgentToolResult, String> {
            Ok(AgentToolResult {
                content: vec![ToolResultContent::Text {
                    text: args.to_string(),
                }],
                details: serde_json::Value::Null,
                usage: None,
                added_tool_names: vec![],
                terminate: false,
            })
        }
    }
    Arc::new(EchoTool)
}

/// Helper to extract the kinds of events (without payload).
pub fn event_kinds(events: &[AgentEvent]) -> Vec<&'static str> {
    events
        .iter()
        .map(|e| match e {
            AgentEvent::AgentStart => "AgentStart",
            AgentEvent::AgentEnd { .. } => "AgentEnd",
            AgentEvent::TurnStart => "TurnStart",
            AgentEvent::TurnEnd { .. } => "TurnEnd",
            AgentEvent::MessageStart { .. } => "MessageStart",
            AgentEvent::MessageUpdate { .. } => "MessageUpdate",
            AgentEvent::MessageEnd { .. } => "MessageEnd",
            AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
            AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
            AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
        })
        .collect()
}

#[allow(dead_code)]
pub async fn run_loop_with_deps(
    prompts: Vec<AgentMessage>,
    deps: RunLoopDeps,
) -> Vec<AgentMessage> {
    run_loop(prompts, AgentContext::default(), deps)
        .await
        .new_messages
}
