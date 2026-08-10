use super::*;
use crate::types::AssistantContent;
use futures::StreamExt;

#[tokio::test]
async fn responses_trace_maps_text_delta_and_completion_to_pi_events() {
    let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\
             data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\
             data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":7,\"total_tokens\":19,\"input_tokens_details\":{\"cached_tokens\":3},\"output_tokens_details\":{\"reasoning_tokens\":2}}}}\n",
        )
        .expect("Responses trace");
    let mut events = provider
        .stream(
            &Model::default(),
            &crate::types::AgentContext::default(),
            None,
        )
        .await
        .expect("replay stream");
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Start { .. })
    ));
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::TextDelta { delta, .. }) if delta == "hello"
    ));
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage,
            ..
        }) if usage.input == 9
            && usage.output == 7
            && usage.cache_read == 3
            && usage.reasoning == 2
            && usage.total_tokens == 19
    ));
}

#[test]
fn provider_usage_conformance_accepts_common_wire_shapes() {
    let cases = [
        (
            serde_json::json!({"response":{"usage":{"input_tokens":12,"output_tokens":7,"total_tokens":19,"input_tokens_details":{"cached_tokens":3}}}}),
            (9, 7, 3, 19),
        ),
        (
            serde_json::json!({"usage":{"prompt_tokens":12,"completion_tokens":7,"total_tokens":19,"prompt_tokens_details":{"cached_tokens":3}}}),
            (9, 7, 3, 19),
        ),
        (
            serde_json::json!({"usage":{"input_tokens":4,"output_tokens":2,"cached_tokens":1,"reasoning_tokens":1}}),
            (3, 2, 1, 6),
        ),
    ];
    for (payload, expected) in cases {
        let usage = response_usage(&payload);
        assert_eq!(
            (
                usage.input,
                usage.output,
                usage.cache_read,
                usage.total_tokens
            ),
            expected
        );
    }
}

#[tokio::test]
async fn deferred_replay_uses_provider_scoped_event_fixture() {
    let handle = crate::types::DeferredHandle {
        provider: "replay".into(),
        model_id: "model".into(),
        api: "responses".into(),
        id: "deferred-1".into(),
        expires_at: None,
        poll_after_ms: None,
        data: None,
    };
    let provider = ReplayProvider::from_sse_body(
        "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
    )
    .expect("ordinary replay trace")
    .with_deferred_events(vec![AssistantMessageEvent::Done {
        stop_reason: StopReason::Stop,
        usage: Usage::default(),
        message: None,
    }])
    .with_deferred_handle(handle.clone());
    let mut events = provider
        .fetch_deferred(&Model::default(), &handle, None)
        .await
        .expect("deferred replay capability");
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Done { .. })
    ));
    provider
        .cancel_deferred(&Model::default(), &handle, None)
        .await
        .expect("deferred cancellation capability");
    assert!(provider
        .fetch_deferred(&Model::default(), &handle, None)
        .await
        .is_err());
}

#[tokio::test]
async fn deferred_replay_rejects_a_foreign_provider_scope() {
    let expected = DeferredHandle {
        provider: "replay".into(),
        model_id: "model".into(),
        api: "responses".into(),
        id: "deferred-1".into(),
        expires_at: None,
        poll_after_ms: None,
        data: None,
    };
    let provider = ReplayProvider::from_sse_body(
        "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
    )
    .expect("ordinary replay trace")
    .with_deferred_events(vec![])
    .with_deferred_handle(expected);
    let foreign = DeferredHandle {
        provider: "other".into(),
        model_id: "model".into(),
        api: "responses".into(),
        id: "deferred-1".into(),
        expires_at: None,
        poll_after_ms: None,
        data: None,
    };
    assert!(provider
        .fetch_deferred(&Model::default(), &foreign, None)
        .await
        .is_err());
}

#[tokio::test]
async fn deferred_replay_consumes_ordered_poll_batches() {
    let (provider, handle) = ordered_poll_fixture();
    let mut first = provider
        .fetch_deferred(&Model::default(), &handle, None)
        .await
        .expect("first deferred poll");
    assert_deferred_stop(&mut first, StopReason::Deferred).await;
    let mut second = provider
        .fetch_deferred(&Model::default(), &handle, None)
        .await
        .expect("terminal deferred poll");
    assert_deferred_stop(&mut second, StopReason::Stop).await;
    assert!(provider
        .fetch_deferred(&Model::default(), &handle, None)
        .await
        .is_err());
}

fn ordered_poll_fixture() -> (ReplayProvider, DeferredHandle) {
    let handle = DeferredHandle {
        provider: "replay".into(),
        model_id: "model".into(),
        api: "responses".into(),
        id: "deferred-poll".into(),
        expires_at: None,
        poll_after_ms: Some(10),
        data: None,
    };
    let provider = ReplayProvider::from_sse_body(
        "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
    )
    .expect("ordinary replay trace")
    .with_deferred_handle(handle.clone())
    .with_deferred_poll_events(vec![
        vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Deferred,
            usage: Usage::default(),
            message: None,
        }],
        vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }],
    ]);
    (provider, handle)
}

async fn assert_deferred_stop(events: &mut AssistantMessageEventStream, expected: StopReason) {
    assert!(
        matches!(events.next().await, Some(AssistantMessageEvent::Done { stop_reason, .. }) if stop_reason == expected)
    );
}

#[tokio::test]
async fn websocket_messages_use_the_same_codex_responses_decoder() {
    let provider = ReplayProvider::from_websocket_messages([
        r#"{"type":"response.created","response":{"id":"ws-1"}}"#,
        r#"{"type":"response.output_text.delta","delta":"socket"}"#,
        r#"{"type":"response.completed","response":{"status":"completed"}}"#,
    ])
    .expect("WebSocket Responses messages");
    let mut events = provider
        .stream(
            &Model::default(),
            &crate::types::AgentContext::default(),
            None,
        )
        .await
        .expect("replay stream");
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Start { .. })
    ));
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::TextDelta { delta, .. }) if delta == "socket"
    ));
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            ..
        })
    ));
}

#[test]
fn websocket_decoder_rejects_malformed_or_non_object_messages() {
    let malformed = ReplayProvider::from_websocket_messages(["not-json"]);
    assert!(
        matches!(malformed, Err(StreamError::Invalid(message)) if message.contains("invalid Codex WebSocket JSON"))
    );

    let scalar = ReplayProvider::from_websocket_messages(["null"]);
    assert!(
        matches!(scalar, Err(StreamError::Invalid(message)) if message.contains("must be a JSON object"))
    );
}

#[tokio::test]
async fn responses_trace_collects_function_call_arguments_by_output_index() {
    let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.output_item.added\",\"output_index\":2,\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"echo\"}}\n\
             data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":2,\"delta\":\"{\\\"x\\\":\"}\n\
             data: {\"type\":\"response.function_call_arguments.done\",\"output_index\":2,\"arguments\":\"{\\\"x\\\":1}\"}\n\
             data: {\"type\":\"response.output_item.done\",\"output_index\":2,\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"echo\",\"arguments\":\"{\\\"x\\\":1}\"}}\n\
             data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n",
        )
        .expect("Responses tool trace");
    let mut events = provider
        .stream(
            &Model::default(),
            &crate::types::AgentContext::default(),
            None,
        )
        .await
        .expect("replay stream");
    let mut tool_call = None;
    while let Some(event) = events.next().await {
        if let AssistantMessageEvent::ToolCallDelta { partial, .. } = event {
            tool_call = partial
                .content
                .into_iter()
                .find_map(|content| match content {
                    AssistantContent::ToolCall(call) => Some(call),
                    _ => None,
                });
        }
    }
    let tool_call = tool_call.expect("tool call event");
    assert_eq!(tool_call.id, "call-1");
    assert_eq!(tool_call.name, "echo");
    assert_eq!(tool_call.arguments, serde_json::json!({"x": 1}));
}

#[test]
fn responses_failed_trace_preserves_provider_code_and_message() {
    let result = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"rate_limit\",\"message\":\"try later\"}}}\n",
        );
    let error = match result {
        Ok(_) => panic!("failed Responses trace must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        StreamError::Api(message) if message == "rate_limit: try later"
    ));
}

#[test]
fn raw_sse_error_preserves_provider_code_and_message() {
    let result = ReplayProvider::from_sse_body(
        "error: {\"code\":\"overloaded\",\"message\":\"try again\"}\n",
    );
    assert!(matches!(
        result,
        Err(StreamError::Api(message)) if message == "overloaded: try again"
    ));
}

#[test]
fn http_error_message_prefers_structured_provider_body() {
    assert_eq!(
        super::http_error_message(
            429,
            r#"{"error":{"code":"rate_limit","message":"try later"}}"#,
        ),
        "rate_limit: try later"
    );
    assert_eq!(super::http_error_message(500, "not JSON"), "HTTP 500");
}

#[test]
fn root_provider_message_is_not_serialized_as_json() {
    assert_eq!(
        super::response_error_message(&serde_json::json!({"message": "try again"})),
        "try again"
    );
}

#[tokio::test]
async fn responses_incomplete_max_output_maps_to_length() {
    let provider = ReplayProvider::from_sse_body(
            "data: {\"type\":\"response.incomplete\",\"response\":{\"status\":\"incomplete\",\"incomplete_details\":{\"reason\":\"max_output_tokens\"}}}\n",
        )
        .expect("incomplete Responses trace");
    let mut events = provider
        .stream(
            &Model::default(),
            &crate::types::AgentContext::default(),
            None,
        )
        .await
        .expect("replay stream");
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Start { .. })
    ));
    assert!(matches!(
        events.next().await,
        Some(AssistantMessageEvent::Done {
            stop_reason: StopReason::MaxTokens,
            ..
        })
    ));
}
