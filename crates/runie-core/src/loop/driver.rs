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
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantContent,
    AssistantMessage, AssistantMessageEvent, Model, QueueMode, SimpleStreamOptions, StopReason,
    ToolCall, ToolExecutionMode, ToolResultContent, ToolResultMessage, WireMessage,
};

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

/// Bag of dependencies the driver needs.
#[derive(Clone)]
pub struct RunLoopDeps {
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
    /// Static provider options inherited by each request; dynamic credentials
    /// are applied on top immediately before the call.
    pub stream_options: SimpleStreamOptions,
    /// Abort signal: when it flips true the loop stops before the next turn
    /// (pi `Agent.abort()`).
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    pub tool_execution_mode: ToolExecutionMode,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
}

#[derive(Debug, Default)]
pub struct RunLoopOutcome {
    pub new_messages: Vec<AgentMessage>,
}

/// Run a full agent loop for the supplied prompts. Mirrors
/// `pi-agent-core`'s `prompt("X")` event sequence.
#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "the loop keeps Pi's ordered turn, hook, queue, and settlement transitions together"
)]
pub async fn run_loop(
    prompts: Vec<AgentMessage>,
    context: AgentContext,
    deps: RunLoopDeps,
    skip_initial_steering_poll: bool,
) -> RunLoopOutcome {
    publish_pi_and_apply(&deps, PiAgentEvent::AgentStart).await;
    publish_pi_and_apply(&deps, PiAgentEvent::TurnStart).await;
    let mut override_ctx = initial_context_override(context, &prompts);
    let mut all_new = initialize_run(prompts, &deps, skip_initial_steering_poll).await;

    // Overrides applied by `prepareNextTurn` (pi `AgentLoopTurnUpdate`).
    let mut override_model: Option<Model> = None;

    loop {
        if check_abort(&deps).await {
            break;
        }

        let Some((assistant, ctx, turn_tool_results, has_more_tool_calls)) = run_assistant_turn(
            override_model.clone(),
            override_ctx.clone(),
            &deps,
            &mut all_new,
        )
        .await
        else {
            break;
        };
        // Carry the completed context into the next provider request. It is
        // rebuilt from the actor projection after each turn, while preserving
        // any caller-supplied context that was authoritative for this run.
        override_ctx = Some(ctx.clone());

        // pi turn hooks (agent-loop.ts:232,247): run after turn_end, before the
        // steering/follow-up poll.
        let hook_ctx = ShouldStopAfterTurnContext {
            message: assistant.clone(),
            tool_results: turn_tool_results.clone(),
            context: ctx.clone(),
            new_messages: all_new.clone(),
        };
        if apply_turn_hooks(&deps, hook_ctx, &mut override_model, &mut override_ctx).await {
            return end_run(all_new, &deps).await;
        }

        if continue_after_turn(has_more_tool_calls, &deps, &mut all_new).await {
            publish_pi_and_apply(&deps, PiAgentEvent::TurnStart).await;
            continue;
        }
        break;
    }

    end_run(all_new, &deps).await
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
    publish_pi_and_apply(
        deps,
        PiAgentEvent::AgentEnd {
            messages: all_new.clone(),
        },
    )
    .await;
    RunLoopOutcome {
        new_messages: all_new,
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

async fn run_assistant_turn(
    override_model: Option<Model>,
    override_ctx: Option<AgentContext>,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> Option<(AssistantMessage, AgentContext, Vec<ToolResultMessage>, bool)> {
    let snap = deps.state.snapshot();
    let model = override_model.unwrap_or_else(|| snap.model.clone());
    let base_context = AgentContext {
        system_prompt: snap.system_prompt,
        messages: snap.messages,
        tools: Some(snap.tools),
    };
    let context = override_ctx.unwrap_or(base_context);
    let effective = match &deps.transform_context {
        Some(transform) => transform(context.messages.clone()).await,
        None => context.messages.clone(),
    };
    let wire_messages = match &deps.convert_to_llm {
        Some(convert) => convert(effective.clone()).await,
        None => default_convert_to_llm(&effective),
    };
    let wire_context = AgentContext {
        system_prompt: context.system_prompt.clone(),
        messages: wire_to_agent(&wire_messages),
        tools: context.tools.clone(),
    };
    let assistant = stream_assistant(model, wire_context, deps).await?;
    let assistant_message = AgentMessage::Assistant(assistant.clone());
    all_new.push(assistant_message);

    let (results, more) = finish_assistant_turn(assistant.clone(), deps, all_new).await?;
    // pi passes hooks the context after the completed turn, including the
    // assistant and any tool results. Rebuild it from the actor-owned
    // projection rather than exposing the pre-turn context.
    deps.state.sync().await;
    let post_turn = deps.state.snapshot();
    let hook_context = AgentContext {
        system_prompt: post_turn.system_prompt,
        messages: merge_context_messages(&context.messages, &post_turn.messages),
        tools: Some(post_turn.tools),
    };
    Some((assistant, hook_context, results, more))
}

fn merge_context_messages(
    existing: &[AgentMessage],
    projected: &[AgentMessage],
) -> Vec<AgentMessage> {
    let overlap = (0..=existing.len().min(projected.len()))
        .rev()
        .find(|size| existing[existing.len() - size..] == projected[..*size])
        .unwrap_or(0);
    let mut merged = existing.to_vec();
    merged.extend_from_slice(&projected[overlap..]);
    merged
}

async fn finish_assistant_turn(
    assistant: AssistantMessage,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> Option<(Vec<ToolResultMessage>, bool)> {
    let tool_calls = assistant
        .content
        .iter()
        .filter_map(|content| match content {
            crate::types::AssistantContent::ToolCall(call) => Some(call.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    match decide_next_turn(&deps.state.snapshot(), tool_calls, false, false) {
        TurnPlan::ToolBatch { calls } => run_tool_batch(assistant, calls, deps, all_new).await,
        TurnPlan::Stop { .. } => {
            publish_pi_and_apply(
                deps,
                PiAgentEvent::TurnEnd {
                    message: AgentMessage::Assistant(assistant),
                    tool_results: vec![],
                },
            )
            .await;
            None
        }
        TurnPlan::Continue => {
            publish_pi_and_apply(
                deps,
                PiAgentEvent::TurnEnd {
                    message: AgentMessage::Assistant(assistant),
                    tool_results: vec![],
                },
            )
            .await;
            Some((vec![], false))
        }
    }
}

async fn apply_turn_hooks(
    deps: &RunLoopDeps,
    hook_ctx: ShouldStopAfterTurnContext,
    override_model: &mut Option<Model>,
    override_ctx: &mut Option<AgentContext>,
) -> bool {
    let should_stop = if let Some(stop) = &deps.turn_hooks.should_stop_after_turn_async {
        stop(hook_ctx.clone()).await
    } else {
        deps.turn_hooks
            .should_stop_after_turn
            .as_ref()
            .is_some_and(|stop| stop(hook_ctx.clone()))
    };
    if should_stop {
        return true;
    }
    let update = if let Some(prepare) = &deps.turn_hooks.prepare_next_turn_async {
        prepare(hook_ctx.clone()).await
    } else {
        deps.turn_hooks
            .prepare_next_turn
            .as_ref()
            .and_then(|prepare| prepare(hook_ctx))
    };
    let Some(update) = update else {
        return false;
    };
    if let Some(context) = update.context {
        *override_ctx = Some(context);
    }
    if let Some(model) = update.model {
        *override_model = Some(model);
    }
    if let Some(level) = update.thinking_level {
        let event = AgentEvent::ThinkingLevelChanged { level };
        publish_and_apply(deps, event).await;
    }
    false
}

async fn inject_steering_messages(deps: &RunLoopDeps, all_new: &mut Vec<AgentMessage>) -> bool {
    let messages = drain_queue(deps.steering_mode, &deps.steering).await;
    inject_messages(messages, deps, all_new).await
}

async fn inject_follow_up_messages(deps: &RunLoopDeps, all_new: &mut Vec<AgentMessage>) -> bool {
    let messages = drain_follow_up(deps.follow_up_mode, &deps.follow_up).await;
    inject_messages(messages, deps, all_new).await
}

async fn inject_messages(
    messages: Vec<AgentMessage>,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> bool {
    let injected = !messages.is_empty();
    for msg in messages {
        publish_input_message(&msg, deps).await;
        all_new.push(msg);
    }
    injected
}

async fn run_tool_batch(
    assistant: AssistantMessage,
    calls: Vec<ToolCall>,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> Option<(Vec<ToolResultMessage>, bool)> {
    let prepublished_starts = !matches!(assistant.stop_reason, Some(StopReason::MaxTokens));
    if prepublished_starts {
        for call in &calls {
            let event = PiAgentEvent::ToolExecutionStart {
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
                args: call.arguments.clone(),
            };
            publish_pi_and_apply(deps, event).await;
        }
    }
    let outcome = if matches!(assistant.stop_reason, Some(StopReason::MaxTokens)) {
        fail_truncated_calls(&calls)
    } else {
        execute_tool_calls(assistant.clone(), calls, deps).await?
    };
    publish_tool_outcome(outcome, prepublished_starts, assistant, deps, all_new).await
}

async fn publish_tool_outcome(
    outcome: ToolExecOutcome,
    prepublished_starts: bool,
    assistant: AssistantMessage,
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> Option<(Vec<ToolResultMessage>, bool)> {
    for event in outcome.events {
        if prepublished_starts && matches!(event, AgentEvent::ToolExecutionStart { .. }) {
            continue;
        }
        publish_pi_or_application(deps, event).await;
    }
    for result in &outcome.tool_results {
        let message = AgentMessage::ToolResult(result.clone());
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
        all_new.push(message);
    }
    let more = !outcome.tool_results.is_empty() && !outcome.all_terminated;
    publish_pi_and_apply(
        deps,
        PiAgentEvent::TurnEnd {
            message: AgentMessage::Assistant(assistant),
            tool_results: outcome.tool_results.clone(),
        },
    )
    .await;
    Some((outcome.tool_results, more))
}

async fn execute_tool_calls(
    assistant: AssistantMessage,
    calls: Vec<ToolCall>,
    deps: &RunLoopDeps,
) -> Option<ToolExecOutcome> {
    let snapshot = deps.state.snapshot();
    let context = AgentContext {
        system_prompt: snapshot.system_prompt,
        messages: snapshot.messages,
        tools: Some(snapshot.tools),
    };
    match deps
        .tool_executor
        .execute(
            assistant,
            context,
            deps.abort.clone(),
            Some(deps.bus.clone()),
            calls,
            deps.tool_execution_mode,
            deps.hooks.clone(),
        )
        .await
    {
        crate::tools::ToolOutcome::Completed {
            tool_results,
            events,
            all_terminated,
        } => Some(ToolExecOutcome {
            tool_results,
            events,
            all_terminated,
        }),
        crate::tools::ToolOutcome::Aborted { reason } => {
            publish_error(deps, &reason).await;
            None
        }
    }
}

async fn drain_queue(mode: QueueMode, queue: &SteeringQueueActor) -> Vec<AgentMessage> {
    match mode {
        QueueMode::OneAtATime => queue.drain_one().await.into_iter().collect(),
        QueueMode::All => queue.drain_all().await,
    }
}

async fn drain_follow_up(mode: QueueMode, queue: &FollowUpQueueActor) -> Vec<AgentMessage> {
    match mode {
        QueueMode::OneAtATime => queue.drain_one().await.into_iter().collect(),
        QueueMode::All => queue.drain_all().await,
    }
}

async fn stream_assistant(
    model: Model,
    context: AgentContext,
    deps: &RunLoopDeps,
) -> Option<AssistantMessage> {
    let options = stream_options(&model, deps).await;
    let mut receiver = match deps.provider.start(model.clone(), context, options).await {
        Some(receiver) => receiver,
        None => {
            publish_error(deps, "provider: no stream").await;
            return None;
        }
    };
    let mut assistant = AssistantMessage {
        content: vec![],
        // The streaming partial starts in `Pending` (pi proxy.ts:124).
        stop_reason: Some(StopReason::Pending),
        model: model.id.clone(),
        api: model.api.clone(),
        provider: model.provider.clone(),
        ..Default::default()
    };
    publish_pi_and_apply(
        deps,
        PiAgentEvent::MessageStart {
            message: assistant_message(&assistant),
        },
    )
    .await;

    drain_assistant_events(&mut receiver, &mut assistant, deps).await;
    publish_pi_and_apply(
        deps,
        PiAgentEvent::MessageEnd {
            message: assistant_message(&assistant),
        },
    )
    .await;
    Some(assistant)
}

async fn stream_options(model: &Model, deps: &RunLoopDeps) -> Option<SimpleStreamOptions> {
    let api_key = match &deps.api_key_resolver {
        Some(resolve) => resolve(model.provider.clone())
            .await
            .or_else(|| deps.stream_options.api_key.clone()),
        None => deps.stream_options.api_key.clone(),
    };
    let mut options = deps.stream_options.clone();
    options.api_key = api_key;
    options.signal = deps.abort.clone();
    options.max_tokens = options
        .max_tokens
        .or((model.max_tokens > 0).then_some(model.max_tokens));
    options.headers = merge_headers(&model.headers, options.headers.take());
    options.sampling_params = merge_sampling_params(
        model.sampling_params.as_ref(),
        options.sampling_params.take(),
    );
    Some(options)
}

/// Pi merges model/provider headers first and lets request headers override
/// matching keys. Keep the merge pure so replay can assert the effective
/// request without constructing a transport.
fn merge_headers(
    model: &std::collections::HashMap<String, String>,
    request: Option<std::collections::HashMap<String, String>>,
) -> Option<std::collections::HashMap<String, String>> {
    if model.is_empty() && request.is_none() {
        return None;
    }
    let mut merged = model.clone();
    if let Some(request) = request {
        merged.extend(request);
    }
    Some(merged)
}

/// Merge model defaults with per-request overrides using Pi's precedence:
/// request values win when both maps contain the same key.
fn merge_sampling_params(
    model: Option<&std::collections::HashMap<String, serde_json::Value>>,
    request: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Option<std::collections::HashMap<String, serde_json::Value>> {
    match (model, request) {
        (None, None) => None,
        (Some(model), None) => Some(model.clone()),
        (None, Some(request)) => Some(request),
        (Some(model), Some(request)) => {
            let mut merged = model.clone();
            merged.extend(request);
            Some(merged)
        }
    }
}

async fn drain_assistant_events(
    receiver: &mut tokio::sync::broadcast::Receiver<AssistantMessageEvent>,
    assistant: &mut AssistantMessage,
    deps: &RunLoopDeps,
) {
    let mut abort = deps.abort.clone();
    loop {
        let event = tokio::select! {
            result = receiver.recv() => result.ok(),
            aborted = wait_for_abort(&mut abort) => {
                if aborted {
                    assistant.stop_reason = Some(StopReason::Aborted);
                    assistant.error_message = Some("aborted".into());
                    break;
                }
                continue;
            }
        };
        let Some(event) = event else { break };
        if process_stream_event(assistant, event, deps).await {
            break;
        }
    }
}

async fn process_stream_event(
    assistant: &mut AssistantMessage,
    event: AssistantMessageEvent,
    deps: &RunLoopDeps,
) -> bool {
    apply_event(assistant, event.clone());
    let event = enrich_assistant_partial(event, assistant);
    if is_delta_event(&event) {
        let update = PiAgentEvent::MessageUpdate {
            message: assistant_message(assistant),
            event: event.clone(),
        };
        publish_pi_and_apply(deps, update).await;
    }
    matches!(
        event,
        AssistantMessageEvent::Done { .. } | AssistantMessageEvent::Error { .. }
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "keeps the exhaustive pi assistant event enrichment table together"
)]
fn enrich_assistant_partial(
    event: AssistantMessageEvent,
    assistant: &AssistantMessage,
) -> AssistantMessageEvent {
    let partial = assistant.clone();
    match event {
        AssistantMessageEvent::TextStart { index, .. } => {
            AssistantMessageEvent::TextStart { index, partial }
        }
        AssistantMessageEvent::TextDelta { index, delta, .. } => AssistantMessageEvent::TextDelta {
            index,
            delta,
            partial,
        },
        AssistantMessageEvent::TextEnd { index, content, .. } => AssistantMessageEvent::TextEnd {
            index,
            content,
            partial,
        },
        AssistantMessageEvent::ThinkingStart { index, .. } => {
            AssistantMessageEvent::ThinkingStart { index, partial }
        }
        AssistantMessageEvent::ThinkingDelta { index, delta, .. } => {
            AssistantMessageEvent::ThinkingDelta {
                index,
                delta,
                partial,
            }
        }
        AssistantMessageEvent::ThinkingEnd {
            index,
            content,
            elapsed_ms,
            ..
        } => AssistantMessageEvent::ThinkingEnd {
            index,
            content,
            elapsed_ms,
            partial,
        },
        AssistantMessageEvent::ToolCallEnd {
            index, tool_call, ..
        } => AssistantMessageEvent::ToolCallEnd {
            index,
            tool_call,
            partial,
        },
        AssistantMessageEvent::ToolCallDelta { index, delta, .. } => {
            AssistantMessageEvent::ToolCallDelta {
                index,
                delta,
                partial,
            }
        }
        other => other,
    }
}

fn is_delta_event(event: &AssistantMessageEvent) -> bool {
    matches!(
        event,
        AssistantMessageEvent::TextStart { .. }
            | AssistantMessageEvent::TextDelta { .. }
            | AssistantMessageEvent::TextEnd { .. }
            | AssistantMessageEvent::ThinkingStart { .. }
            | AssistantMessageEvent::ThinkingDelta { .. }
            | AssistantMessageEvent::ThinkingEnd { .. }
            | AssistantMessageEvent::ToolCallStart { .. }
            | AssistantMessageEvent::ToolCallDelta { .. }
            | AssistantMessageEvent::ToolCallEnd { .. }
    )
}

async fn wait_for_abort(abort: &mut Option<tokio::sync::watch::Receiver<bool>>) -> bool {
    let Some(receiver) = abort else {
        return std::future::pending::<bool>().await;
    };
    loop {
        if *receiver.borrow() {
            return true;
        }
        if receiver.changed().await.is_err() {
            return false;
        }
    }
}

fn assistant_message(assistant: &AssistantMessage) -> AgentMessage {
    AgentMessage::Assistant(assistant.clone())
}

/// Continue from existing context (no new prompt).
pub async fn run_loop_continue(context: AgentContext, deps: RunLoopDeps) -> RunLoopOutcome {
    let prompts = vec![];
    run_loop(prompts, context, deps, false).await
}

struct ToolExecOutcome {
    tool_results: Vec<ToolResultMessage>,
    events: Vec<AgentEvent>,
    /// True when every result had `terminate: true` (pi `shouldTerminateToolBatch`).
    all_terminated: bool,
}

/// Synthesize error results for every tool call in a message that was
/// truncated by the output token limit. Mirrors pi-agent-core's
/// `failToolCallsFromTruncatedMessage`: no tool is executed; each call is
/// reported as an error so the caller can re-issue it with complete
/// arguments.
fn fail_truncated_calls(calls: &[ToolCall]) -> ToolExecOutcome {
    let mut tool_results = Vec::with_capacity(calls.len());
    let mut events = Vec::with_capacity(calls.len() * 2);
    for call in calls {
        let result = truncated_result(call);
        events.push(AgentEvent::ToolExecutionStart {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            args: call.arguments.clone(),
        });
        events.push(AgentEvent::ToolExecutionEnd {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            result: serde_json::to_value(&result).unwrap_or_default(),
            is_error: true,
        });
        tool_results.push(ToolResultMessage {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            content: result.content,
            is_error: true,
            ..Default::default()
        });
    }
    ToolExecOutcome {
        tool_results,
        events,
        // pi failToolCallsFromTruncatedMessage returns terminate:false, so the
        // loop continues to a follow-up turn (agent-loop.ts:405).
        all_terminated: false,
    }
}

fn truncated_result(call: &ToolCall) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text { text: format!("Tool call \"{}\" was not executed: the response hit the output token limit, so its arguments may be truncated. Re-issue the tool call with complete arguments.", call.name) }],
        details: serde_json::json!({}),
        usage: None,
        added_tool_names: vec![],
        terminate: false,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the pure assistant event reducer keeps Pi variants together"
)]
fn apply_event(assistant: &mut AssistantMessage, event: AssistantMessageEvent) {
    use crate::types::AssistantContent;
    match event {
        AssistantMessageEvent::Start { .. } => {}
        // Sectional markers delimit content blocks; the deltas carry content.
        AssistantMessageEvent::TextStart { .. } | AssistantMessageEvent::TextEnd { .. } => {}
        AssistantMessageEvent::TextDelta { delta, .. } => {
            push_or_append(assistant, AssistantContent::Text { text: delta });
        }
        AssistantMessageEvent::ThinkingStart { .. } => {}
        AssistantMessageEvent::ThinkingEnd { elapsed_ms, .. } => {
            assistant.thinking_elapsed_ms = elapsed_ms;
        }
        AssistantMessageEvent::ThinkingDelta { delta, .. } => {
            push_or_append(assistant, AssistantContent::Thinking { text: delta });
        }
        AssistantMessageEvent::ToolCallStart { partial, .. }
        | AssistantMessageEvent::ToolCallDelta { partial, .. } => {
            merge_assistant_partial(assistant, partial);
        }
        AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
            upsert_tool_call(assistant, tool_call);
        }
        AssistantMessageEvent::Done {
            stop_reason,
            usage,
            message,
        } => {
            assistant.stop_reason = Some(stop_reason);
            assistant.usage = usage;
            if let Some(terminal) = message {
                *assistant = terminal;
            }
        }
        AssistantMessageEvent::Error { reason, error } => {
            assistant.stop_reason = Some(reason);
            assistant.error_message = error.error_message;
            assistant.usage = error.usage;
            assistant.raw_stop_reason = error.raw_stop_reason;
        }
    }
}

fn upsert_tool_call(assistant: &mut AssistantMessage, call: ToolCall) {
    use crate::types::AssistantContent;

    if let Some(existing) = assistant
        .content
        .iter_mut()
        .find_map(|content| match content {
            AssistantContent::ToolCall(existing) if existing.id == call.id => Some(existing),
            _ => None,
        })
    {
        *existing = call;
    } else {
        assistant.content.push(AssistantContent::ToolCall(call));
    }
}

fn merge_assistant_partial(assistant: &mut AssistantMessage, partial: AssistantMessage) {
    for content in partial.content {
        match content {
            AssistantContent::ToolCall(call) => upsert_tool_call(assistant, call),
            other => push_or_append(assistant, other),
        }
    }
    assistant.stop_reason = partial.stop_reason;
    assistant.usage = partial.usage;
    assistant.thinking_elapsed_ms = partial.thinking_elapsed_ms;
}

fn push_or_append(assistant: &mut AssistantMessage, content: AssistantContent) {
    use crate::types::AssistantContent;
    match (assistant.content.last_mut(), &content) {
        (Some(AssistantContent::Text { text }), AssistantContent::Text { text: new_text }) => {
            text.push_str(new_text.as_str());
            return;
        }
        (
            Some(AssistantContent::Thinking { text }),
            AssistantContent::Thinking { text: new_text },
        ) => {
            text.push_str(new_text.as_str());
            return;
        }
        _ => {}
    }
    assistant.content.push(content);
}

fn wire_to_agent(wire: &[WireMessage]) -> Vec<AgentMessage> {
    wire.iter()
        .map(|w| match w {
            WireMessage::User { content, timestamp } => {
                AgentMessage::User(crate::types::UserMessage {
                    content: content.clone(),
                    timestamp: *timestamp,
                })
            }
            WireMessage::Assistant {
                content,
                stop_reason,
                model,
                timestamp,
            } => AgentMessage::Assistant(crate::types::AssistantMessage {
                content: content.clone(),
                stop_reason: *stop_reason,
                model: model.clone(),
                timestamp: *timestamp,
                ..Default::default()
            }),
            WireMessage::ToolResult {
                tool_call_id,
                tool_name,
                content,
                is_error,
                timestamp,
            } => AgentMessage::ToolResult(ToolResultMessage {
                tool_call_id: tool_call_id.clone(),
                tool_name: tool_name.clone(),
                content: content.clone(),
                is_error: *is_error,
                timestamp: *timestamp,
                ..Default::default()
            }),
        })
        .collect()
}

#[allow(dead_code)]
fn _tools_marker(_t: &[Arc<dyn AgentTool>]) {}

#[cfg(test)]
mod event_reconstruction_tests {
    use super::*;

    #[test]
    fn sampling_params_merge_request_over_model_defaults() {
        let model: std::collections::HashMap<_, _> = [
            ("temperature".into(), serde_json::json!(0.2)),
            ("top_p".into(), serde_json::json!(0.9)),
        ]
        .into_iter()
        .collect();
        let request: std::collections::HashMap<_, _> =
            [("temperature".into(), serde_json::json!(0.7))]
                .into_iter()
                .collect();
        let merged = merge_sampling_params(Some(&model), Some(request)).unwrap();
        assert_eq!(merged["temperature"], serde_json::json!(0.7));
        assert_eq!(merged["top_p"], serde_json::json!(0.9));
    }

    #[test]
    fn headers_merge_request_values_over_model_defaults() {
        let model = [
            ("x-model".into(), "model".into()),
            ("x-shared".into(), "model".into()),
        ]
        .into_iter()
        .collect();
        let request = [
            ("x-request".into(), "request".into()),
            ("x-shared".into(), "request".into()),
        ]
        .into_iter()
        .collect();
        let merged = merge_headers(&model, Some(request)).expect("headers are preserved");
        assert_eq!(merged["x-model"], "model");
        assert_eq!(merged["x-request"], "request");
        assert_eq!(merged["x-shared"], "request");
    }

    #[test]
    fn stream_updates_replace_provider_placeholder_with_owned_partial() {
        let assistant = AssistantMessage {
            content: vec![AssistantContent::Text {
                text: "hello".into(),
            }],
            ..AssistantMessage::default()
        };
        let event = enrich_assistant_partial(
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hello".into(),
                partial: AssistantMessage::default(),
            },
            &assistant,
        );
        let AssistantMessageEvent::TextDelta { partial, .. } = event else {
            panic!("text delta remains a text delta");
        };
        assert_eq!(partial, assistant);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn tool_call_markers_reconstruct_one_content_block() {
        let mut assistant = AssistantMessage::default();
        let partial = ToolCall {
            id: "call-1".into(),
            name: "read".into(),
            arguments: serde_json::json!({"path": "a"}),
            thought_signature: None,
        };
        apply_event(
            &mut assistant,
            AssistantMessageEvent::ToolCallStart {
                index: 0,
                partial: AssistantMessage::with_tool_call(partial.clone()),
            },
        );
        apply_event(
            &mut assistant,
            AssistantMessageEvent::ToolCallDelta {
                index: 0,
                delta: "{\"path\":\"a.rs\"}".into(),
                partial: AssistantMessage::with_tool_call(ToolCall {
                    arguments: serde_json::json!({"path": "a.rs"}),
                    ..partial.clone()
                }),
            },
        );
        apply_event(
            &mut assistant,
            AssistantMessageEvent::ToolCallEnd {
                index: 0,
                tool_call: ToolCall {
                    arguments: serde_json::json!({"path": "a.rs"}),
                    ..partial
                },
                partial: AssistantMessage::default(),
            },
        );
        assert_eq!(assistant.content.len(), 1);
        assert!(matches!(
            &assistant.content[0],
            AssistantContent::ToolCall(call) if call.arguments == serde_json::json!({"path": "a.rs"})
        ));
    }
}
