use super::*;
pub(super) async fn stream_assistant(
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

pub(super) async fn stream_options(
    model: &Model,
    deps: &RunLoopDeps,
) -> Option<SimpleStreamOptions> {
    let api_key = match &deps.api_key_resolver {
        Some(resolve) => resolve(model.provider.clone())
            .await
            .or_else(|| deps.stream_options.api_key.clone()),
        None => deps.stream_options.api_key.clone(),
    };
    let mut options = deps.stream_options.clone();
    options.api_key = api_key;
    options.signal = deps.abort.clone();
    options.reasoning = Some(deps.state.snapshot().thinking_level);
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
pub(super) fn merge_headers(
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
pub(super) fn merge_sampling_params(
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

pub(super) async fn drain_assistant_events(
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

pub(super) async fn process_stream_event(
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

pub(super) async fn wait_for_abort(abort: &mut Option<tokio::sync::watch::Receiver<bool>>) -> bool {
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

pub(super) fn assistant_message(assistant: &AssistantMessage) -> AgentMessage {
    AgentMessage::Assistant(assistant.clone())
}

/// Continue from existing context (no new prompt).
pub async fn run_loop_continue(context: AgentContext, deps: RunLoopDeps) -> RunLoopOutcome {
    let prompts = vec![];
    run_loop(prompts, context, deps, false).await
}

pub(super) struct ToolExecOutcome {
    pub(super) tool_results: Vec<ToolResultMessage>,
    pub(super) events: Vec<AgentEvent>,
    /// True when every result had `terminate: true` (pi `shouldTerminateToolBatch`).
    pub(super) all_terminated: bool,
}

/// Synthesize error results for every tool call in a message that was
/// truncated by the output token limit. Mirrors pi-agent-core's
/// `failToolCallsFromTruncatedMessage`: no tool is executed; each call is
/// reported as an error so the caller can re-issue it with complete
/// arguments.
pub(super) fn fail_truncated_calls(calls: &[ToolCall]) -> ToolExecOutcome {
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

pub(super) fn truncated_result(call: &ToolCall) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text { text: format!("Tool call \"{}\" was not executed: the response hit the output token limit, so its arguments may be truncated. Re-issue the tool call with complete arguments.", call.name) }],
        details: serde_json::json!({}),
        usage: None,
        added_tool_names: vec![],
        terminate: false,
    }
}

#[allow(dead_code)]
pub(super) fn _tools_marker(_t: &[Arc<dyn AgentTool>]) {}
