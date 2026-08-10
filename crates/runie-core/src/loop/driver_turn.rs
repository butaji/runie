use super::*;

async fn recover_context(context: AgentContext, deps: &RunLoopDeps) -> Option<AgentContext> {
    let Some(recover) = &deps.context_recovery else {
        return Some(context);
    };
    match recover(context).await {
        Ok(context) => Some(context),
        Err(error) => {
            publish_and_apply(deps, AgentEvent::Error { message: error }).await;
            None
        }
    }
}

pub(super) async fn run_assistant_turn(
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
    let context = recover_context(context, deps).await?;
    let wire_context = prepare_wire_context(&context, deps).await;
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

pub(super) async fn prepare_wire_context(
    context: &AgentContext,
    deps: &RunLoopDeps,
) -> AgentContext {
    let effective = match &deps.transform_context {
        Some(transform) => transform(context.messages.clone()).await,
        None => context.messages.clone(),
    };
    let wire_messages = match &deps.convert_to_llm {
        Some(convert) => convert(effective.clone()).await,
        None => default_convert_to_llm(&effective),
    };
    AgentContext {
        system_prompt: context.system_prompt.clone(),
        messages: wire_to_agent(&wire_messages),
        tools: context.tools.clone(),
    }
}

pub(super) fn merge_context_messages(
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

pub(super) async fn finish_assistant_turn(
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

pub(super) async fn apply_turn_hooks(
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

pub(super) async fn inject_steering_messages(
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> bool {
    let messages = drain_queue(deps.steering_mode, &deps.steering).await;
    inject_messages(messages, deps, all_new).await
}

pub(super) async fn inject_follow_up_messages(
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) -> bool {
    let messages = drain_follow_up(deps.follow_up_mode, &deps.follow_up).await;
    inject_messages(messages, deps, all_new).await
}

pub(super) async fn inject_messages(
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

pub(super) async fn run_tool_batch(
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

pub(super) async fn publish_tool_outcome(
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
    publish_tool_messages(&outcome.tool_results, deps, all_new).await;
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

pub(super) async fn publish_tool_messages(
    results: &[ToolResultMessage],
    deps: &RunLoopDeps,
    all_new: &mut Vec<AgentMessage>,
) {
    for result in results {
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
}

pub(super) async fn execute_tool_calls(
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

pub(super) async fn drain_queue(mode: QueueMode, queue: &SteeringQueueActor) -> Vec<AgentMessage> {
    match mode {
        QueueMode::OneAtATime => queue.drain_one().await.into_iter().collect(),
        QueueMode::All => queue.drain_all().await,
    }
}

pub(super) async fn drain_follow_up(
    mode: QueueMode,
    queue: &FollowUpQueueActor,
) -> Vec<AgentMessage> {
    match mode {
        QueueMode::OneAtATime => queue.drain_one().await.into_iter().collect(),
        QueueMode::All => queue.drain_all().await,
    }
}
