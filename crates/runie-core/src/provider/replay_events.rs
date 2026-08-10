use super::*;

pub struct SseParseState {
    pub(super) finished: bool,
    pub(super) stop_reason: StopReason,
    pub(super) raw_stop_reason: Option<String>,
    pub(super) usage: Usage,
    pub(super) tool_calls: std::collections::BTreeMap<usize, (String, String, String)>,
}

impl Default for SseParseState {
    fn default() -> Self {
        Self {
            finished: false,
            stop_reason: StopReason::Stop,
            raw_stop_reason: None,
            usage: Usage::default(),
            tool_calls: Default::default(),
        }
    }
}

pub(super) fn consume_sse_line(
    line: &str,
    state: &mut SseParseState,
    events: &mut Vec<AssistantMessageEvent>,
) -> Result<bool, StreamError> {
    if let Some(raw_error) = line.strip_prefix("error:").map(str::trim_start) {
        return Err(match serde_json::from_str::<serde_json::Value>(raw_error) {
            Ok(value) => StreamError::Api(response_error_message(&value)),
            Err(_) => StreamError::Api(raw_error.to_owned()),
        });
    }
    let Some(raw) = line.strip_prefix("data:").map(str::trim_start) else {
        return Ok(false);
    };
    if raw == "[DONE]" {
        state.finished = true;
        return Ok(true);
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Ok(false);
    };
    if matches!(
        value.get("type").and_then(|value| value.as_str()),
        Some("error") | Some("response.failed")
    ) {
        return Err(StreamError::Api(response_error_message(&value)));
    }
    state.finished |= append_text_events(&value, events);
    collect_tool_calls(&value, &mut state.tool_calls);
    if has_terminal_marker(&value) {
        state.finished = true;
        // Chat tool-call traces historically finalize their typed stop state
        // from reconstructed tool content; keep that reducer behavior while
        // retaining the provider's raw finish value below. Responses-style
        // incomplete markers use the typed conformance mapping directly.
        state.stop_reason = if value.pointer("/choices/0/finish_reason").is_some() {
            StopReason::Stop
        } else {
            response_finish_reason(&value).unwrap_or(StopReason::Stop)
        };
        state.raw_stop_reason = raw_response_finish_reason(&value).map(str::to_owned);
        state.usage = response_usage(&value);
    }
    Ok(false)
}

pub(super) fn response_error_message(value: &serde_json::Value) -> String {
    let response = value.get("response").unwrap_or(value);
    let error = response.get("error").or_else(|| {
        response
            .get("code")
            .or_else(|| response.get("message"))
            .map(|_| response)
    });
    let code = error.and_then(|v| v.get("code")).and_then(|v| v.as_str());
    let message = error
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str());
    let reason = response
        .get("incomplete_details")
        .and_then(|v| v.get("reason"))
        .and_then(|v| v.as_str());
    match (code, message, reason) {
        (Some(code), Some(message), _) => format!("{code}: {message}"),
        (_, Some(message), _) => message.to_owned(),
        (_, _, Some(reason)) => format!("incomplete: {reason}"),
        _ => value.to_string(),
    }
}

pub(super) fn http_error_message(status: u16, body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .filter(|value| value.is_object())
        .map(|value| response_error_message(&value))
        .filter(|message| message != body)
        .unwrap_or_else(|| format!("HTTP {status}"))
}

pub(super) fn raw_response_finish_reason(value: &serde_json::Value) -> Option<&str> {
    value
        .pointer("/choices/0/finish_reason")
        .or_else(|| value.pointer("/delta/stop_reason"))
        .or_else(|| value.pointer("/response/incomplete_details/reason"))
        .and_then(serde_json::Value::as_str)
}

pub(super) fn response_finish_reason(value: &serde_json::Value) -> Option<StopReason> {
    let raw = raw_response_finish_reason(value)?;
    Some(match raw {
        "stop" | "end_turn" | "completed" => StopReason::Stop,
        "length" | "max_tokens" | "max_output_tokens" => StopReason::MaxTokens,
        "tool_calls" | "tool_use" => StopReason::ToolUse,
        "aborted" | "cancelled" => StopReason::Aborted,
        "error" | "content_filter" => StopReason::Error,
        _ => StopReason::Stop,
    })
}

pub(super) fn response_usage(value: &serde_json::Value) -> Usage {
    let Some(raw) = value
        .pointer("/response/usage")
        .or_else(|| value.pointer("/usage"))
    else {
        return Usage::default();
    };
    let input = raw
        .get("input_tokens")
        .or_else(|| raw.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let output = raw
        .get("output_tokens")
        .or_else(|| raw.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let cache_read = raw
        .pointer("/input_tokens_details/cached_tokens")
        .or_else(|| raw.pointer("/prompt_tokens_details/cached_tokens"))
        .or_else(|| raw.get("cached_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let reasoning = raw
        .pointer("/output_tokens_details/reasoning_tokens")
        .or_else(|| raw.get("reasoning_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    Usage {
        input: input.saturating_sub(cache_read),
        output,
        cache_read,
        total_tokens: raw
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(input.saturating_add(output)),
        reasoning,
        ..Usage::default()
    }
}

pub(super) fn finish_replay_events(
    events: &mut Vec<AssistantMessageEvent>,
    tool_calls: std::collections::BTreeMap<usize, (String, String, String)>,
    stop_reason: StopReason,
    raw_stop_reason: Option<String>,
    usage: Usage,
) {
    let has_tool_calls = !tool_calls.is_empty();
    for (_, (id, name, arguments)) in tool_calls {
        let args = serde_json::from_str(&arguments)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        events.push(AssistantMessageEvent::ToolCallDelta {
            index: 0,
            delta: arguments.clone(),
            partial: AssistantMessage::with_tool_call(ToolCall {
                id,
                name,
                arguments: args,
                thought_signature: None,
            }),
        });
    }
    events.push(AssistantMessageEvent::Done {
        stop_reason,
        usage,
        message: (!has_tool_calls)
            .then_some(raw_stop_reason)
            .flatten()
            .map(|raw_stop_reason| AssistantMessage {
                raw_stop_reason: Some(raw_stop_reason),
                ..AssistantMessage::default()
            }),
    });
}

pub(super) fn append_text_events(
    value: &serde_json::Value,
    events: &mut Vec<AssistantMessageEvent>,
) -> bool {
    for (pointer, thinking) in [
        ("/choices/0/delta/content", false),
        ("/delta/text", false),
        ("/choices/0/delta/reasoning_content", true),
        ("/delta/thinking", true),
        ("/delta/reasoning", true),
    ] {
        if let Some(text) = value
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            events.push(if thinking {
                AssistantMessageEvent::ThinkingDelta {
                    index: 1,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            } else {
                AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            });
        }
    }
    append_responses_text_event(value, events);
    false
}

pub(super) fn append_responses_text_event(
    value: &serde_json::Value,
    events: &mut Vec<AssistantMessageEvent>,
) {
    match value.get("type").and_then(|v| v.as_str()) {
        Some("response.output_text.delta") | Some("response.refusal.delta") => {
            if let Some(text) = value
                .get("delta")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                events.push(AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                });
            }
        }
        Some("response.reasoning_summary_text.delta") | Some("response.reasoning_text.delta") => {
            if let Some(text) = value
                .get("delta")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                events.push(AssistantMessageEvent::ThinkingDelta {
                    index: 1,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                });
            }
        }
        _ => {}
    }
}

pub(super) fn collect_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    collect_anthropic_tool_call(value, tool_calls);
    collect_openai_tool_calls(value, tool_calls);
    collect_responses_tool_call(value, tool_calls);
}

pub(super) fn collect_anthropic_tool_call(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    if let Some(partial) = value
        .pointer("/delta/partial_json")
        .and_then(|v| v.as_str())
    {
        tool_calls.entry(0).or_default().2.push_str(partial);
    }
    let Some(block) = value
        .pointer("/content_block")
        .filter(|v| v.get("type").and_then(|x| x.as_str()) == Some("tool_use"))
    else {
        return;
    };
    let entry = tool_calls
        .entry(value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
        .or_default();
    entry.0 = block
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("trace-tool")
        .into();
    entry.1 = block
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .into();
}

pub(super) fn collect_openai_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    let Some(calls) = value
        .pointer("/choices/0/delta/tool_calls")
        .and_then(|v| v.as_array())
    else {
        return;
    };
    for call in calls {
        let entry = tool_calls
            .entry(call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
            .or_default();
        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
            entry.0 = id.into();
        }
        if let Some(name) = call.pointer("/function/name").and_then(|v| v.as_str()) {
            entry.1 = name.into();
        }
        if let Some(args) = call.pointer("/function/arguments").and_then(|v| v.as_str()) {
            entry.2.push_str(args);
        }
    }
}

pub(super) fn collect_responses_tool_call(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    let Some(event_type) = value.get("type").and_then(|v| v.as_str()) else {
        return;
    };
    match event_type {
        "response.output_item.added" => collect_response_item(value, tool_calls, false),
        "response.function_call_arguments.delta" => {
            collect_response_arguments(value, tool_calls, false)
        }
        "response.function_call_arguments.done" => {
            collect_response_arguments(value, tool_calls, true)
        }
        "response.output_item.done" => collect_response_item(value, tool_calls, true),
        _ => {}
    }
}

pub(super) fn response_output_index(value: &serde_json::Value) -> usize {
    value
        .get("output_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize
}

pub(super) fn collect_response_arguments(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
    replace: bool,
) {
    let entry = tool_calls.entry(response_output_index(value)).or_default();
    let Some(arguments) = value
        .get(if replace { "arguments" } else { "delta" })
        .and_then(|v| v.as_str())
    else {
        return;
    };
    if replace {
        entry.2 = arguments.into();
    } else {
        entry.2.push_str(arguments);
    }
}

pub(super) fn collect_response_item(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
    include_arguments: bool,
) {
    let Some(item) = value.get("item") else {
        return;
    };
    if item.get("type").and_then(|v| v.as_str()) != Some("function_call") {
        return;
    }
    let entry = tool_calls.entry(response_output_index(value)).or_default();
    if let Some(id) = item
        .get("call_id")
        .or_else(|| item.get("id"))
        .and_then(|v| v.as_str())
    {
        entry.0 = id.into();
    }
    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
        entry.1 = name.into();
    }
    if include_arguments {
        if let Some(arguments) = item.get("arguments").and_then(|v| v.as_str()) {
            entry.2 = arguments.into();
        }
    }
}

pub(super) fn has_terminal_marker(value: &serde_json::Value) -> bool {
    value.get("type").and_then(|v| v.as_str()) == Some("message_stop")
        || value.get("type").and_then(|v| v.as_str()) == Some("response.completed")
        || value.get("type").and_then(|v| v.as_str()) == Some("response.incomplete")
        || value
            .pointer("/delta/stop_reason")
            .is_some_and(|v| !v.is_null())
        || value
            .pointer("/choices/0/finish_reason")
            .is_some_and(|v| !v.is_null())
}
