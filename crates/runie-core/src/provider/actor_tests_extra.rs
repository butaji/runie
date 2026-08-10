use super::*;
#[test]
fn telemetry_response_attributes_preserve_provider_identity() {
    let attributes = telemetry_response_attributes(&AssistantMessage {
        response_model: Some("routed-model".into()),
        response_id: Some("response-1".into()),
        ..AssistantMessage::default()
    });
    assert_eq!(attributes["pi.ai.response.model"], "routed-model");
    assert_eq!(attributes["pi.ai.response.id"], "response-1");
}

#[tokio::test]
async fn provider_stream_error_ends_telemetry_span_as_error() {
    let telemetry = crate::telemetry::TelemetryActor::new();
    let actor = ProviderActor::new(std::sync::Arc::new(ErrorEventFn));
    let options = SimpleStreamOptions {
        telemetry: Some(telemetry.clone()),
        ..Default::default()
    };
    let mut rx = actor
        .start(
            Model {
                provider: "test-provider".into(),
                id: "test-model".into(),
                api: "test-api".into(),
                ..Model::default()
            },
            AgentContext::default(),
            Some(options),
        )
        .await
        .unwrap();
    while rx.recv().await.is_ok() {}
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.spans[0].status, SpanStatus::Error);
    assert_eq!(snapshot.spans[0].attributes["pi.ai.error.type"], "provider");
    assert_eq!(
        snapshot.spans[0]
            .error
            .as_ref()
            .map(|error| error.message.as_str()),
        Some("stream failed")
    );
    assert!(snapshot.spans[0].ended);
}

#[tokio::test]
async fn provider_http_status_is_preserved_on_telemetry_error() {
    let telemetry = crate::telemetry::TelemetryActor::new();
    let span = telemetry
        .start_span(
            None,
            "pi.ai.request",
            HashMap::from([
                ("pi.ai.operation".into(), serde_json::json!("stream")),
                ("pi.ai.provider".into(), serde_json::json!("test-provider")),
                ("pi.ai.model".into(), serde_json::json!("test-model")),
                ("pi.ai.api".into(), serde_json::json!("test-api")),
                ("pi.ai.streaming".into(), serde_json::json!(true)),
            ]),
        )
        .await
        .expect("request span");
    finish_request_span_error(
        Some(span),
        &StreamError::Provider {
            message: "rate limited".into(),
            status: Some(429),
            headers: HashMap::new(),
        },
    )
    .await;
    assert_eq!(
        telemetry.snapshot().spans[0].attributes["pi.ai.http.status_code"],
        429
    );
}

#[tokio::test]
async fn cancel_aborts_owned_stream_pump() {
    let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
    let mut rx = actor
        .start(Model::default(), AgentContext::default(), None)
        .await
        .unwrap();
    actor.cancel().await;
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
        .await
        .expect("cancel should close the stream");
    assert!(result.is_err());
}

#[tokio::test]
async fn cancel_settles_the_owned_provider_telemetry_span() {
    let telemetry = crate::telemetry::TelemetryActor::new();
    let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
    let options = SimpleStreamOptions {
        telemetry: Some(telemetry.clone()),
        ..Default::default()
    };
    let mut events = actor
        .start(Model::default(), AgentContext::default(), Some(options))
        .await
        .unwrap();
    actor.cancel().await;
    assert!(events.recv().await.is_err());
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.spans.len(), 1);
    assert!(snapshot.spans[0].ended);
    assert_eq!(snapshot.spans[0].status, SpanStatus::Error);
    assert_eq!(snapshot.spans[0].error.as_ref().unwrap().name, "AbortError");
}

#[tokio::test]
async fn a_new_start_supersedes_the_previous_owned_stream() {
    let actor = ProviderActor::new(std::sync::Arc::new(PendingFn));
    let mut previous = actor
        .start(Model::default(), AgentContext::default(), None)
        .await
        .unwrap();
    let _current = actor
        .start(Model::default(), AgentContext::default(), None)
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), previous.recv())
        .await
        .expect("superseded stream should close");
    assert!(result.is_err());
}

#[tokio::test]
async fn startup_error_is_encoded_as_assistant_error_event() {
    let actor = ProviderActor::new(std::sync::Arc::new(ErrorFn));
    let mut rx = actor
        .start(Model::default(), AgentContext::default(), None)
        .await
        .unwrap();
    assert!(matches!(
        rx.recv().await.unwrap(),
        crate::types::AssistantMessageEvent::Error { error, .. }
            if error.error_text() == "api: bad request"
    ));
}

#[tokio::test]
async fn unsupported_deferred_commands_return_explicit_capability_errors() {
    let actor = ProviderActor::new(std::sync::Arc::new(ThreeEventFn));
    let handle = unsupported_deferred_handle();

    let fetch = actor
        .fetch_deferred(Model::default(), handle.clone(), None)
        .await;
    assert!(matches!(
        fetch,
        Err(StreamError::Invalid(message))
            if message == "provider does not support deferred responses"
    ));

    let cancel = actor.cancel_deferred(Model::default(), handle, None).await;
    assert!(matches!(
        cancel,
        Err(StreamError::Invalid(message))
            if message == "provider cannot cancel deferred responses"
    ));
}

fn unsupported_deferred_handle() -> DeferredHandle {
    DeferredHandle {
        provider: "test".into(),
        model_id: "test".into(),
        api: "test".into(),
        id: "deferred-1".into(),
        expires_at: None,
        poll_after_ms: None,
        data: None,
    }
}

#[tokio::test]
async fn deferred_fetch_and_cancel_use_the_injected_provider_capability() {
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let actor = ProviderActor::new(Arc::new(DeferredFn {
        cancelled: cancelled.clone(),
    }));
    let telemetry = crate::telemetry::TelemetryActor::new();
    let options = Some(SimpleStreamOptions {
        telemetry: Some(telemetry.clone()),
        ..Default::default()
    });
    let model = Model {
        provider: "test".into(),
        id: "test".into(),
        api: "test".into(),
        ..Model::default()
    };
    let handle = supported_deferred_handle();

    let mut events = actor
        .fetch_deferred(model.clone(), handle.clone(), options.clone())
        .await
        .expect("deferred fetch capability");
    assert!(matches!(
        events.recv().await.expect("deferred event"),
        AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            ..
        }
    ));

    actor
        .cancel_deferred(model, handle, options)
        .await
        .expect("deferred cancellation capability");
    assert!(cancelled.load(std::sync::atomic::Ordering::Acquire));
    assert_deferred_telemetry(&telemetry.snapshot().spans);
}

fn assert_deferred_telemetry(spans: &[crate::telemetry::SpanSnapshot]) {
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].attributes["pi.ai.operation"], "fetch_deferred");
    assert_eq!(spans[1].attributes["pi.ai.operation"], "cancel_deferred");
    assert!(spans
        .iter()
        .all(|span| span.ended && span.status == SpanStatus::Ok));
}

fn supported_deferred_handle() -> DeferredHandle {
    DeferredHandle {
        provider: "test".into(),
        model_id: "test".into(),
        api: "test".into(),
        id: "deferred-1".into(),
        expires_at: None,
        poll_after_ms: None,
        data: Some(serde_json::json!({"continuation": true})),
    }
}

#[tokio::test]
async fn compaction_summary_uses_owned_provider_capability() {
    let actor = ProviderActor::new(Arc::new(SummarizerFn));
    let summary = actor
        .summarize_compaction(crate::session::CompactionSummaryRequest {
            history: vec![],
            turn_prefix: vec![],
            retained_tail: vec![],
            tokens_before: 42,
            previous_summary: None,
            custom_instructions: None,
        })
        .await
        .expect("summary capability");
    assert_eq!(summary.summary, "42 tokens");
    assert_eq!(
        summary.details,
        Some(serde_json::json!({"adapter": "test"}))
    );
}

#[tokio::test]
async fn model_discovery_uses_owned_provider_capability() {
    let actor = ProviderActor::new(Arc::new(SummarizerFn));
    let models = actor.list_models().await.expect("model discovery");
    assert_eq!(models[0].id, "catalog-model");
}
