//! Agent loop driver. Pure `async fn` — no actor state. The owning
//! `LoopActor` spawns this and joins it via `JoinHandle`.
//!
//! Event sequence matches the TS README exactly.

use std::sync::Arc;

use crate::convert::default_convert_to_llm;
use crate::events::EventBus;
use crate::events::SubscriberRegistry;
use crate::hooks::{ShouldStopAfterTurnContext, TurnHooks};
use crate::pi_event::PiAgentEvent;
use crate::provider::ProviderActor;
use crate::queues::{FollowUpQueueActor, SteeringQueueActor};
use crate::r#loop::turn::{decide_next_turn, TurnPlan};
use crate::state::AgentStateActor;
use crate::tools::executor::ToolExecHooks;
use crate::tools::ToolExecutorActor;
use crate::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantMessage,
    AssistantMessageEvent, Model, QueueMode, SimpleStreamOptions, StopReason, ToolCall,
    ToolExecutionMode, ToolResultContent, ToolResultMessage, WireMessage,
};

mod driver_projection;
use driver_projection::{apply_event, enrich_assistant_partial, is_delta_event, wire_to_agent};

pub type TransformContext = Arc<
    dyn Fn(Vec<AgentMessage>) -> futures::future::BoxFuture<'static, Vec<AgentMessage>>
        + Send
        + Sync,
>;

pub type ConvertToLlm = Arc<
    dyn Fn(Vec<AgentMessage>) -> futures::future::BoxFuture<'static, Vec<WireMessage>>
        + Send
        + Sync,
>;

/// Resolves credentials immediately before each provider request (pi
/// `AgentLoopConfig.getApiKey`).
pub type ApiKeyResolver =
    Arc<dyn Fn(String) -> futures::future::BoxFuture<'static, Option<String>> + Send + Sync>;

/// Actor-owned context recovery invoked immediately before a provider turn.
/// The loop owns timing; the hook owner owns compaction state and events.
pub type ContextRecoveryHook = Arc<
    dyn Fn(AgentContext, Model) -> futures::future::BoxFuture<'static, Result<AgentContext, String>>
        + Send
        + Sync,
>;

/// Bag of dependencies the driver needs.
#[derive(Clone)]
pub struct RunLoopDeps {
    /// Actor-issued identity for the current Pi operation.
    pub run_id: String,
    pub state: AgentStateActor,
    pub steering: SteeringQueueActor,
    pub follow_up: FollowUpQueueActor,
    pub tool_executor: ToolExecutorActor,
    pub provider: ProviderActor,
    pub bus: EventBus,
    pub subscribers: SubscriberRegistry,
    pub hooks: ToolExecHooks,
    pub turn_hooks: TurnHooks,
    /// pi `transformContext` (agent-loop.ts:289): pre-processes the agent
    /// messages before `convert_to_llm` each turn.
    pub transform_context: Option<TransformContext>,
    /// pi `convertToLlm`: customize the final wire projection per request.
    pub convert_to_llm: Option<ConvertToLlm>,
    pub api_key_resolver: Option<ApiKeyResolver>,
    pub context_recovery: Option<ContextRecoveryHook>,
    /// Static provider options inherited by each request; dynamic credentials
    /// are applied on top immediately before the call.
    pub stream_options: SimpleStreamOptions,
    /// Abort signal: when it flips true the loop stops before the next turn
    /// (pi `Agent.abort()`).
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    pub tool_execution_mode: ToolExecutionMode,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    pub provider_events: Arc<tokio::sync::Mutex<Vec<AssistantMessageEvent>>>,
}

#[derive(Debug, Default)]
pub struct RunLoopOutcome {
    pub new_messages: Vec<AgentMessage>,
    pub provider_events: Vec<AssistantMessageEvent>,
}

fn publish_operation_record(
    deps: &RunLoopDeps,
    kind: crate::types::OperationRecordKind,
    data: serde_json::Value,
) {
    deps.bus
        .publish(AgentEvent::TypedOperationRecordCreated { kind, data });
}

/// Run a full agent loop for the supplied prompts. Mirrors
/// `pi-agent-core`'s `prompt("X")` event sequence.
pub async fn run_loop(
    prompts: Vec<AgentMessage>,
    context: AgentContext,
    deps: RunLoopDeps,
    skip_initial_steering_poll: bool,
) -> RunLoopOutcome {
    publish_pi_and_apply(&deps, PiAgentEvent::AgentStart).await;
    publish_operation_record(
        &deps,
        crate::types::OperationRecordKind::OperationStarted,
        serde_json::json!({"id": deps.run_id, "intent": {"kind": "run"}}),
    );
    publish_pi_and_apply(&deps, PiAgentEvent::TurnStart).await;
    let mut override_ctx = initial_context_override(context, &prompts);
    let mut all_new = initialize_run(prompts, &deps, skip_initial_steering_poll).await;

    let mut override_model = None;
    if let Some(outcome) =
        run_turns(&deps, &mut all_new, &mut override_ctx, &mut override_model).await
    {
        return outcome;
    }
    end_run(all_new, &deps).await
}

async fn run_turns(
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
    override_ctx: &mut Option<AgentContext>,
    override_model: &mut Option<Model>,
) -> Option<RunLoopOutcome> {
    loop {
        if check_abort(deps).await {
            return None;
        }
        let (assistant, ctx, tool_results, has_more) =
            run_assistant_turn(override_model.clone(), override_ctx.clone(), deps, all_new).await?;
        *override_ctx = Some(ctx.clone());
        let hook_ctx = ShouldStopAfterTurnContext {
            message: assistant,
            tool_results,
            context: ctx,
            new_messages: all_new.clone(),
        };
        if apply_turn_hooks(deps, hook_ctx, override_model, override_ctx).await {
            return Some(end_run(std::mem::take(all_new), deps).await);
        }
        if continue_after_turn(has_more, deps, all_new).await {
            publish_pi_and_apply(deps, PiAgentEvent::TurnStart).await;
            continue;
        }
        return None;
    }
}

fn initial_context_override(
    context: AgentContext,
    prompts: &[AgentMessage],
) -> Option<AgentContext> {
    let has_context = !context.messages.is_empty()
        || !context.system_prompt.is_empty()
        || context.tools.is_some();
    if !has_context {
        return None;
    }
    let mut context = context;
    context.messages.extend_from_slice(prompts);
    Some(context)
}

async fn continue_after_turn(
    has_more_tool_calls: bool,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> bool {
    let steering_injected = inject_steering_messages(deps, all_new).await;
    if steering_injected || has_more_tool_calls {
        return true;
    }

    // pi drains follow-up messages only after the agent would otherwise stop;
    // they must not be consumed while a tool batch is continuing.
    inject_follow_up_messages(deps, all_new).await
}

async fn end_run(all_new: Vec<AgentMessage>, deps: &RunLoopDeps) -> RunLoopOutcome {
    let assistant_aborted = all_new.iter().rev().any(|message| {
        matches!(
            message,
            AgentMessage::Assistant(assistant)
                if assistant.stop_reason == Some(StopReason::Aborted)
        )
    });
    let outcome = if deps.abort.as_ref().is_some_and(|abort| *abort.borrow()) || assistant_aborted {
        "aborted"
    } else if deps.state.snapshot().error_message.is_some() {
        "failed"
    } else {
        "completed"
    };
    publish_operation_record(
        deps,
        crate::types::OperationRecordKind::OperationFinished,
        serde_json::json!({"runId": deps.run_id, "outcome": outcome}),
    );
    publish_pi_and_apply(
        deps,
        PiAgentEvent::AgentEnd {
            messages: all_new.clone(),
        },
    )
    .await;
    RunLoopOutcome {
        new_messages: all_new,
        provider_events: deps.provider_events.lock().await.clone(),
    }
}

async fn check_abort(deps: &RunLoopDeps) -> bool {
    if deps.abort.as_ref().is_some_and(|abort| *abort.borrow()) {
        publish_error(deps, "aborted").await;
        return true;
    }
    false
}

async fn publish_error(deps: &RunLoopDeps, message: &str) {
    let event = AgentEvent::Error {
        message: message.to_owned(),
    };
    publish_and_apply(deps, event).await;
}

async fn publish_and_apply(deps: &RunLoopDeps, event: AgentEvent) {
    deps.state.publish_event(&deps.bus, event.clone()).await;
    deps.subscribers
        .dispatch_with_abort(&event, deps.abort.as_ref())
        .await;
}

async fn publish_pi_and_apply(deps: &RunLoopDeps, event: PiAgentEvent) {
    let application_event = event.clone().try_into_agent_event();
    deps.state.publish_pi_event(&deps.bus, event.clone()).await;
    deps.subscribers
        .dispatch_with_abort(&application_event, deps.abort.as_ref())
        .await;
    deps.subscribers.dispatch_pi(&event).await;
}

async fn publish_pi_or_application(deps: &RunLoopDeps, event: AgentEvent) {
    match PiAgentEvent::try_from(event) {
        Ok(event) => publish_pi_and_apply(deps, event).await,
        Err(event) => publish_and_apply(deps, event).await,
    }
}

async fn initialize_run(
    prompts: Vec<AgentMessage>,
    deps: &RunLoopDeps,
    skip_initial_steering_poll: bool,
) -> Vec<AgentMessage> {
    for message in &prompts {
        publish_input_message(message, deps).await;
    }
    let mut all_new = prompts;
    let steering = if skip_initial_steering_poll {
        Vec::new()
    } else {
        drain_queue(deps.steering_mode, &deps.steering).await
    };
    for message in steering {
        publish_input_message(&message, deps).await;
        all_new.push(message);
    }
    all_new
}

async fn publish_input_message(message: &AgentMessage, deps: &RunLoopDeps) {
    publish_pi_and_apply(
        deps,
        PiAgentEvent::MessageStart {
            message: message.clone(),
        },
    )
    .await;
    publish_pi_and_apply(
        deps,
        PiAgentEvent::MessageEnd {
            message: message.clone(),
        },
    )
    .await;
}

#[path = "driver_stream.rs"]
mod driver_stream;
#[path = "driver_turn.rs"]
mod driver_turn;
pub use driver_stream::run_loop_continue;
use driver_stream::*;
use driver_turn::*;
#[cfg(test)]
mod event_reconstruction_tests;
