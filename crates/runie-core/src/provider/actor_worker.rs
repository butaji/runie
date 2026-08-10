use super::*;
pub(super) async fn run_provider_worker(
    mut rx: mpsc::Receiver<ProviderCommand>,
    stream_fn: Arc<dyn StreamFn>,
    websocket: Option<Arc<dyn WebSocketAdapter>>,
) {
    let mut pumps = JoinSet::new();
    let mut active_telemetry_span = None;
    while let Some(cmd) = rx.recv().await {
        while pumps.try_join_next().is_some() {}
        handle_provider_command(
            cmd,
            ProviderCommandContext {
                stream_fn: &stream_fn,
                websocket: websocket.as_ref(),
                active_telemetry_span: &mut active_telemetry_span,
                pumps: &mut pumps,
            },
        )
        .await;
    }
}

pub(super) async fn handle_provider_command(
    cmd: ProviderCommand,
    mut worker: ProviderCommandContext<'_>,
) {
    match cmd {
        ProviderCommand::Start {
            model,
            context,
            options,
            reply,
        } => {
            handle_provider_start(&mut worker, *model, *context, *options, reply).await;
        }
        ProviderCommand::Cancel { reply } => {
            handle_provider_cancel(worker.pumps, worker.active_telemetry_span, reply).await
        }
        ProviderCommand::FetchDeferred {
            model,
            handle,
            options,
            reply,
        } => handle_fetch_deferred_command(&mut worker, *model, handle, *options, reply).await,
        ProviderCommand::CancelDeferred {
            model,
            handle,
            options,
            reply,
        } => {
            handle_cancel_deferred(worker.stream_fn, *model, handle, *options, reply).await;
        }
        command => handle_capability_command(worker.stream_fn, command).await,
    }
}

async fn handle_fetch_deferred_command(
    worker: &mut ProviderCommandContext<'_>,
    model: Model,
    handle: crate::types::DeferredHandle,
    options: Option<SimpleStreamOptions>,
    reply: oneshot::Sender<
        Result<
            broadcast::Receiver<crate::types::AssistantMessageEvent>,
            crate::provider::stream_fn::StreamError,
        >,
    >,
) {
    handle_fetch_deferred(
        worker.stream_fn,
        model,
        handle,
        options,
        worker.pumps,
        reply,
    )
    .await;
}

pub(super) async fn handle_provider_start(
    worker: &mut ProviderCommandContext<'_>,
    model: Model,
    agent_context: AgentContext,
    options: Option<SimpleStreamOptions>,
    reply: oneshot::Sender<broadcast::Receiver<crate::types::AssistantMessageEvent>>,
) {
    worker.pumps.abort_all();
    settle_aborted_telemetry_span(worker.active_telemetry_span).await;
    let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
    let attributes = provider_telemetry_attributes(&model);
    let telemetry_span = start_provider_telemetry(&options, attributes).await;
    let stream_result = provider_stream(
        worker.stream_fn,
        worker.websocket,
        &model,
        &agent_context,
        options,
    )
    .await;
    finish_start_request(
        stream_result,
        event_tx,
        telemetry_span,
        worker.active_telemetry_span,
        worker.pumps,
        reply,
    )
    .await;
}

pub(super) async fn handle_capability_command(
    stream_fn: &Arc<dyn StreamFn>,
    command: ProviderCommand,
) {
    match command {
        ProviderCommand::SummarizeCompaction { request, reply } => {
            let _ = reply.send(stream_fn.summarize_compaction(&request).await);
        }
        ProviderCommand::ListModels { reply } => {
            let _ = reply.send(stream_fn.list_models().await);
        }
        _ => unreachable!("active provider commands are handled by the worker"),
    }
}

pub(super) async fn handle_cancel_deferred(
    stream_fn: &Arc<dyn StreamFn>,
    model: Model,
    handle: crate::types::DeferredHandle,
    options: Option<SimpleStreamOptions>,
    reply: oneshot::Sender<Result<(), crate::provider::stream_fn::StreamError>>,
) {
    let telemetry_span =
        start_pi_request_span(options.as_ref(), &model, "cancel_deferred", false, true).await;
    let result = stream_fn.cancel_deferred(&model, &handle, options).await;
    match &result {
        Ok(()) => finish_request_span_ok(telemetry_span).await,
        Err(error) => finish_request_span_error(telemetry_span, error).await,
    }
    let _ = reply.send(result);
}

pub(super) async fn handle_fetch_deferred(
    stream_fn: &Arc<dyn StreamFn>,
    model: Model,
    handle: crate::types::DeferredHandle,
    options: Option<SimpleStreamOptions>,
    pumps: &mut JoinSet<()>,
    reply: oneshot::Sender<
        Result<
            broadcast::Receiver<crate::types::AssistantMessageEvent>,
            crate::provider::stream_fn::StreamError,
        >,
    >,
) {
    let (event_tx, _) = broadcast::channel(STREAM_CAPACITY);
    let telemetry_span =
        start_pi_request_span(options.as_ref(), &model, "fetch_deferred", true, true).await;
    match stream_fn.fetch_deferred(&model, &handle, options).await {
        Ok(stream) => {
            let receiver = event_tx.subscribe();
            pumps.spawn(pump_stream(stream, event_tx, telemetry_span));
            let _ = reply.send(Ok(receiver));
        }
        Err(error) => {
            finish_request_span_error(telemetry_span, &error).await;
            let _ = reply.send(Err(error));
        }
    }
}

pub(super) async fn handle_provider_cancel(
    pumps: &mut JoinSet<()>,
    active_span: &mut Option<TelemetrySpan>,
    reply: oneshot::Sender<()>,
) {
    pumps.abort_all();
    settle_aborted_telemetry_span(active_span).await;
    let _ = reply.send(());
}

pub(super) async fn finish_start_request(
    stream_result: Result<AssistantMessageEventStream, crate::provider::stream_fn::StreamError>,
    event_tx: broadcast::Sender<crate::types::AssistantMessageEvent>,
    telemetry_span: Option<TelemetrySpan>,
    active_span: &mut Option<TelemetrySpan>,
    pumps: &mut JoinSet<()>,
    reply: oneshot::Sender<broadcast::Receiver<crate::types::AssistantMessageEvent>>,
) {
    match stream_result {
        Ok(stream) => {
            let receiver = event_tx.subscribe();
            *active_span = telemetry_span.clone();
            pumps.spawn(pump_stream(stream, event_tx, telemetry_span));
            let _ = reply.send(receiver);
        }
        Err(error) => {
            finish_request_span_error(telemetry_span, &error).await;
            let receiver = event_tx.subscribe();
            let _ = event_tx.send(crate::types::AssistantMessageEvent::Error {
                reason: crate::types::StopReason::Error,
                error: crate::types::AssistantMessage::with_error(
                    crate::types::StopReason::Error,
                    error.to_string(),
                ),
            });
            let _ = reply.send(receiver);
        }
    }
}

pub(super) async fn start_provider_telemetry(
    options: &Option<SimpleStreamOptions>,
    attributes: HashMap<String, serde_json::Value>,
) -> Option<crate::telemetry::TelemetrySpan> {
    let telemetry = options.as_ref()?.telemetry.clone()?;
    if crate::telemetry::validate_pi_ai_request_attributes(&attributes).is_ok() {
        telemetry
            .start_span(None, "pi.ai.request", attributes)
            .await
    } else {
        None
    }
}

pub(super) async fn provider_stream(
    stream_fn: &Arc<dyn StreamFn>,
    websocket: Option<&Arc<dyn WebSocketAdapter>>,
    model: &Model,
    context: &AgentContext,
    options: Option<SimpleStreamOptions>,
) -> Result<AssistantMessageEventStream, crate::provider::stream_fn::StreamError> {
    match options.as_ref().and_then(|options| options.transport) {
        Some(crate::types::ProviderTransport::Websocket)
        | Some(crate::types::ProviderTransport::WebsocketCached) => match websocket {
            Some(adapter) => adapter.stream_websocket(model, context, options).await,
            None => Err(crate::provider::stream_fn::StreamError::Invalid(
                "websocket transport requires a provider-specific websocket adapter".into(),
            )),
        },
        _ => stream_fn.stream(model, context, options).await,
    }
}

pub(super) fn provider_telemetry_attributes(model: &Model) -> HashMap<String, serde_json::Value> {
    HashMap::from([
        ("pi.ai.operation".into(), serde_json::json!("stream")),
        (
            "pi.ai.provider".into(),
            serde_json::json!(model.provider.clone()),
        ),
        ("pi.ai.model".into(), serde_json::json!(model.id.clone())),
        ("pi.ai.api".into(), serde_json::json!(model.api.clone())),
        ("pi.ai.streaming".into(), serde_json::json!(true)),
    ])
}

pub(super) async fn settle_aborted_telemetry_span(span: &mut Option<TelemetrySpan>) {
    if let Some(span) = span.take() {
        span.set_attributes(HashMap::from([(
            "pi.ai.error.type".into(),
            serde_json::json!("abort"),
        )]))
        .await;
        span.status_with_error(
            SpanStatus::Error,
            Some(SpanError {
                name: "AbortError".into(),
                message: "provider stream aborted".into(),
            }),
        )
        .await;
        span.end().await;
    }
}

pub(super) async fn start_pi_request_span(
    options: Option<&SimpleStreamOptions>,
    model: &Model,
    operation: &str,
    streaming: bool,
    deferred: bool,
) -> Option<TelemetrySpan> {
    let telemetry = options.and_then(|options| options.telemetry.clone())?;
    let attributes = HashMap::from([
        ("pi.ai.operation".into(), serde_json::json!(operation)),
        (
            "pi.ai.provider".into(),
            serde_json::json!(model.provider.clone()),
        ),
        ("pi.ai.model".into(), serde_json::json!(model.id.clone())),
        ("pi.ai.api".into(), serde_json::json!(model.api.clone())),
        ("pi.ai.streaming".into(), serde_json::json!(streaming)),
        ("pi.ai.deferred".into(), serde_json::json!(deferred)),
    ]);
    if crate::telemetry::validate_pi_ai_request_attributes(&attributes).is_err() {
        return None;
    }
    telemetry
        .start_span(None, "pi.ai.request", attributes)
        .await
}

pub(super) async fn finish_request_span_ok(span: Option<TelemetrySpan>) {
    if let Some(span) = span {
        span.status(SpanStatus::Ok).await;
        span.end().await;
    }
}

pub(super) async fn finish_request_span_error(span: Option<TelemetrySpan>, error: &StreamError) {
    if let Some(span) = span {
        let mut attributes =
            HashMap::from([("pi.ai.error.type".into(), serde_json::json!("provider"))]);
        if let StreamError::Provider {
            status: Some(status),
            ..
        } = error
        {
            attributes.insert("pi.ai.http.status_code".into(), serde_json::json!(status));
        }
        span.set_attributes(attributes).await;
        span.status_with_error(
            SpanStatus::Error,
            Some(SpanError {
                name: "ProviderError".into(),
                message: error.to_string(),
            }),
        )
        .await;
        span.end().await;
    }
}

pub(super) async fn pump_stream(
    mut stream: AssistantMessageEventStream,
    tx: broadcast::Sender<crate::types::AssistantMessageEvent>,
    telemetry_span: Option<TelemetrySpan>,
) {
    use futures::StreamExt;
    let started_at = std::time::Instant::now();
    let mut terminal_error = None;
    let mut chunk_count = 0u64;
    let mut time_to_first_chunk_ms = None;
    while let Some(event) = stream.next().await {
        update_pump_state(
            &event,
            started_at,
            &mut chunk_count,
            &mut time_to_first_chunk_ms,
            &mut terminal_error,
        );
        update_pump_telemetry(&event, &telemetry_span, chunk_count, time_to_first_chunk_ms).await;
        // Errors from the broadcast are non-fatal (no current receivers).
        let _ = tx.send(event);
    }
    if let Some(span) = telemetry_span {
        if let Some(message) = terminal_error {
            span.status_with_error(
                SpanStatus::Error,
                Some(SpanError {
                    name: "ProviderError".into(),
                    message,
                }),
            )
            .await;
        } else {
            span.status(SpanStatus::Ok).await;
        }
        span.end().await;
    }
}

pub(super) fn update_pump_state(
    event: &crate::types::AssistantMessageEvent,
    started_at: std::time::Instant,
    chunk_count: &mut u64,
    time_to_first_chunk_ms: &mut Option<u64>,
    terminal_error: &mut Option<String>,
) {
    if is_telemetry_chunk(event) {
        *chunk_count = (*chunk_count).saturating_add(1);
        if time_to_first_chunk_ms.is_none() {
            *time_to_first_chunk_ms = Some(started_at.elapsed().as_millis() as u64);
        }
    }
    if let crate::types::AssistantMessageEvent::Error { error, .. } = event {
        *terminal_error = Some(error.error_text());
    }
}
