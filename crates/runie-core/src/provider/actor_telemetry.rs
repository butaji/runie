use super::*;
pub(super) async fn update_pump_telemetry(
    event: &crate::types::AssistantMessageEvent,
    telemetry_span: &Option<TelemetrySpan>,
    chunk_count: u64,
    time_to_first_chunk_ms: Option<u64>,
) {
    let Some(span) = telemetry_span else { return };
    let mut attributes = telemetry_attributes_for_event(event);
    if matches!(
        event,
        crate::types::AssistantMessageEvent::Done { .. }
            | crate::types::AssistantMessageEvent::Error { .. }
    ) {
        attributes.insert(
            "pi.ai.stream.chunk_count".into(),
            serde_json::json!(chunk_count),
        );
        if let Some(elapsed_ms) = time_to_first_chunk_ms {
            attributes.insert(
                "pi.ai.stream.time_to_first_chunk_ms".into(),
                serde_json::json!(elapsed_ms),
            );
        }
    }
    if !attributes.is_empty() && validate_pi_ai_request_end_attributes(&attributes).is_ok() {
        span.set_attributes(attributes).await;
    }
}

pub(super) fn is_telemetry_chunk(event: &crate::types::AssistantMessageEvent) -> bool {
    !matches!(
        event,
        crate::types::AssistantMessageEvent::Start { .. }
            | crate::types::AssistantMessageEvent::Done { .. }
            | crate::types::AssistantMessageEvent::Error { .. }
    )
}

pub(super) fn telemetry_attributes_for_event(
    event: &crate::types::AssistantMessageEvent,
) -> HashMap<String, serde_json::Value> {
    match event {
        crate::types::AssistantMessageEvent::Done {
            stop_reason,
            usage,
            message,
        } => telemetry_done_attributes(*stop_reason, usage, message.as_ref()),
        crate::types::AssistantMessageEvent::Error { reason, error } => {
            telemetry_error_attributes(*reason, error)
        }
        _ => HashMap::new(),
    }
}

pub(super) fn telemetry_done_attributes(
    stop_reason: crate::types::StopReason,
    usage: &crate::types::Usage,
    message: Option<&crate::types::AssistantMessage>,
) -> HashMap<String, serde_json::Value> {
    let mut attributes = HashMap::from([(
        "pi.ai.response.stop_reason".into(),
        serde_json::json!(telemetry_stop_reason(stop_reason)),
    )]);
    attributes.extend(telemetry_usage_attributes(usage));
    if let Some(message) = message {
        attributes.extend(telemetry_response_attributes(message));
    }
    attributes
}

pub(super) fn telemetry_usage_attributes(
    usage: &crate::types::Usage,
) -> HashMap<String, serde_json::Value> {
    HashMap::from([
        (
            "pi.ai.usage.input_tokens".into(),
            serde_json::json!(usage.input),
        ),
        (
            "pi.ai.usage.output_tokens".into(),
            serde_json::json!(usage.output),
        ),
        (
            "pi.ai.usage.total_tokens".into(),
            serde_json::json!(usage.total_tokens),
        ),
        (
            "pi.ai.usage.cache_read_tokens".into(),
            serde_json::json!(usage.cache_read),
        ),
        (
            "pi.ai.usage.cache_write_tokens".into(),
            serde_json::json!(usage.cache_write),
        ),
        (
            "pi.ai.usage.reasoning_tokens".into(),
            serde_json::json!(usage.reasoning),
        ),
        (
            "pi.ai.usage.cost".into(),
            serde_json::json!(usage.cost.total),
        ),
    ])
}

pub(super) fn telemetry_error_attributes(
    reason: crate::types::StopReason,
    error: &crate::types::AssistantMessage,
) -> HashMap<String, serde_json::Value> {
    let mut attributes = HashMap::from([
        (
            "pi.ai.response.stop_reason".into(),
            serde_json::json!(telemetry_stop_reason(reason)),
        ),
        ("pi.ai.error.type".into(), serde_json::json!("provider")),
    ]);
    attributes.extend(telemetry_response_attributes(error));
    attributes
}

pub(super) fn telemetry_response_attributes(
    message: &crate::types::AssistantMessage,
) -> HashMap<String, serde_json::Value> {
    let mut attributes = HashMap::new();
    if let Some(model) = message
        .response_model
        .as_ref()
        .filter(|model| !model.is_empty())
    {
        attributes.insert("pi.ai.response.model".into(), serde_json::json!(model));
    }
    if let Some(response_id) = message.response_id.as_ref().filter(|id| !id.is_empty()) {
        attributes.insert("pi.ai.response.id".into(), serde_json::json!(response_id));
    }
    attributes
}

pub(super) fn telemetry_stop_reason(reason: crate::types::StopReason) -> &'static str {
    match reason {
        crate::types::StopReason::Stop => "stop",
        crate::types::StopReason::ToolUse => "tool_use",
        crate::types::StopReason::MaxTokens => "length",
        crate::types::StopReason::Error => "error",
        crate::types::StopReason::Aborted => "aborted",
        crate::types::StopReason::Pending => "pending",
        crate::types::StopReason::Deferred => "deferred",
    }
}
