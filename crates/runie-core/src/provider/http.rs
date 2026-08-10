//! Minimal transport boundary used by concrete providers and replay tests.

use std::path::Path;

use super::stream_fn::StreamError;
use crate::types::{
    CacheRetention, Model, ProviderResponse, ProviderTransport, SimpleStreamOptions, ThinkingLevel,
};

const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 60_000;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub body: String,
    pub session_id: Option<String>,
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub reasoning: Option<String>,
    pub sampling_params: std::collections::HashMap<String, serde_json::Value>,
    pub headers: std::collections::HashMap<String, String>,
    pub env: std::collections::HashMap<String, String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub transport: Option<ProviderTransport>,
    pub cache_retention: Option<CacheRetention>,
    pub websocket_connect_timeout_ms: Option<u64>,
}

#[async_trait::async_trait]
pub trait HttpActor: Send + Sync + 'static {
    async fn post(&self, body: String) -> Result<HttpResponse, StreamError>;

    async fn post_request(&self, request: HttpRequest) -> Result<HttpResponse, StreamError> {
        if matches!(
            request.transport,
            Some(ProviderTransport::Websocket | ProviderTransport::WebsocketCached)
        ) {
            return Err(StreamError::Invalid(
                "websocket transport requires a provider-specific websocket adapter".into(),
            ));
        }
        self.post(request.body).await
    }

    /// Apply pi-compatible request/response hooks at the transport boundary.
    /// Concrete adapters only implement `post`; the actor owns the side
    /// effect while hooks remain caller-provided observations/transformations.
    async fn post_with_options(
        &self,
        body: String,
        model: Model,
        options: Option<SimpleStreamOptions>,
    ) -> Result<HttpResponse, StreamError> {
        let request = prepare_request(body, &model, options.as_ref()).await?;
        let response = execute_with_retries(self, request, options.as_ref()).await?;
        if let Some(hook) = options
            .as_ref()
            .and_then(|options| options.on_response.clone())
        {
            hook(
                ProviderResponse {
                    status: response.status,
                    headers: response.headers.clone(),
                },
                model,
            )
            .await;
        }
        Ok(response)
    }
}

async fn prepare_request(
    body: String,
    model: &Model,
    options: Option<&SimpleStreamOptions>,
) -> Result<HttpRequest, StreamError> {
    let payload = apply_payload_hook(&body, model, options).await;
    let request_body = match payload {
        serde_json::Value::String(raw) => raw,
        value => serde_json::to_string(&value).map_err(|error| {
            StreamError::Invalid(format!("payload hook serialization failed: {error}"))
        })?,
    };
    Ok(HttpRequest {
        body: request_body,
        session_id: options.and_then(|options| options.session_id.clone()),
        api_key: options.and_then(|options| options.api_key.clone()),
        temperature: options.and_then(|options| options.temperature),
        max_tokens: options.and_then(|options| options.max_tokens),
        reasoning: mapped_reasoning(model, options),
        sampling_params: merged_sampling_params(model, options),
        headers: options
            .and_then(|options| options.headers.clone())
            .unwrap_or_default(),
        env: options
            .and_then(|options| options.env.clone())
            .unwrap_or_default(),
        metadata: options
            .and_then(|options| options.metadata.clone())
            .unwrap_or_default(),
        transport: options.and_then(|options| options.transport),
        cache_retention: options.and_then(|options| options.cache_retention),
        websocket_connect_timeout_ms: options
            .and_then(|options| options.websocket_connect_timeout_ms),
    })
}

async fn apply_payload_hook(
    body: &str,
    model: &Model,
    options: Option<&SimpleStreamOptions>,
) -> serde_json::Value {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .unwrap_or_else(|_| serde_json::Value::String(body.to_owned()));
    match options.and_then(|options| options.on_payload.clone()) {
        Some(hook) => hook(payload.clone(), model.clone())
            .await
            .unwrap_or(payload),
        None => payload,
    }
}

pub fn mapped_reasoning(model: &Model, options: Option<&SimpleStreamOptions>) -> Option<String> {
    let level = options.and_then(|options| options.reasoning)?;
    let map = model.thinking_level_map.as_ref()?;
    let value = match level {
        ThinkingLevel::Off => &map.off,
        ThinkingLevel::Minimal => &map.minimal,
        ThinkingLevel::Low => &map.low,
        ThinkingLevel::Medium => &map.medium,
        ThinkingLevel::High => &map.high,
        ThinkingLevel::XHigh => &map.xhigh,
        ThinkingLevel::Max => &map.max,
    };
    value.clone()
}

async fn execute_with_retries<A: HttpActor + ?Sized>(
    client: &A,
    request: HttpRequest,
    options: Option<&SimpleStreamOptions>,
) -> Result<HttpResponse, StreamError> {
    let retries = options.and_then(|o| o.max_retries).unwrap_or(0);
    let mut last_error = None;
    for retry_index in 0..=retries {
        match post_once(client, request.clone(), options.and_then(|o| o.timeout_ms)).await {
            Ok(response) => return Ok(response),
            Err(error) => {
                if retry_index < retries {
                    retry_after_error(&error, retry_index, options).await?;
                }
                last_error = Some(error);
            }
        }
    }
    Err(last_error.expect("at least one provider attempt must produce a result"))
}

async fn retry_after_error(
    error: &StreamError,
    retry_index: u32,
    options: Option<&SimpleStreamOptions>,
) -> Result<(), StreamError> {
    let Some(decision) = provider_retry_delay_ms_with_jitter(
        error,
        retry_index,
        options.and_then(|o| o.max_retry_delay_ms),
        options
            .and_then(|o| o.retry_jitter.as_ref())
            .map(|jitter| jitter()),
    ) else {
        return Ok(());
    };
    let delay = decision?;
    let signal = options.and_then(|o| o.signal.clone());
    if let Some(hook) = options.and_then(|o| o.retry_delay.clone()) {
        hook(delay, signal).await?;
    } else {
        abortable_retry_delay(delay, signal).await?;
    }
    Ok(())
}

fn merged_sampling_params(
    model: &Model,
    options: Option<&SimpleStreamOptions>,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut params = model.sampling_params.clone().unwrap_or_default();
    if let Some(overrides) = options.and_then(|options| options.sampling_params.as_ref()) {
        params.extend(overrides.clone());
    }
    params
}

async fn abortable_retry_delay(
    delay_ms: u64,
    mut signal: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<(), StreamError> {
    let duration = tokio::time::sleep(std::time::Duration::from_millis(delay_ms));
    tokio::pin!(duration);
    if signal.as_ref().is_some_and(|signal| *signal.borrow()) {
        return Err(StreamError::Aborted);
    }
    if let Some(signal) = signal.as_mut() {
        tokio::select! {
            _ = &mut duration => Ok(()),
            changed = signal.changed() => {
                if changed.is_ok() && *signal.borrow() { Err(StreamError::Aborted) } else { Ok(()) }
            }
        }
    } else {
        duration.await;
        Ok(())
    }
}

async fn post_once<A: HttpActor + ?Sized>(
    actor: &A,
    request: HttpRequest,
    timeout_ms: Option<u64>,
) -> Result<HttpResponse, StreamError> {
    if let Some(timeout_ms) = timeout_ms {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            actor.post_request(request),
        )
        .await
        .map_err(|_| StreamError::Network(format!("request timed out after {timeout_ms}ms")))?
    } else {
        actor.post_request(request).await
    }
}

/// Serves one recorded HTTP response body without opening a socket.
pub struct ReplayHttpActor {
    response: HttpResponse,
}

impl ReplayHttpActor {
    pub async fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let body = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| StreamError::Network(e.to_string()))?;
        Ok(Self {
            response: HttpResponse {
                status: 200,
                headers: std::collections::HashMap::new(),
                body,
            },
        })
    }
}

#[async_trait::async_trait]
impl HttpActor for ReplayHttpActor {
    async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
        if self.response.status >= 400 {
            return Err(StreamError::Provider {
                message: format!("HTTP {}", self.response.status),
                status: Some(self.response.status),
                headers: self.response.headers.clone(),
            });
        }
        Ok(self.response.clone())
    }
}

/// The metadata-only part of Pi's provider retry policy. Keeping this pure
/// makes retry decisions replayable without timers or a live transport.
pub fn provider_retry_delay_ms(
    error: &StreamError,
    retry_index: u32,
    max_retry_delay_ms: Option<u64>,
) -> Option<Result<u64, StreamError>> {
    provider_retry_delay_ms_with_jitter(error, retry_index, max_retry_delay_ms, None)
}

pub fn provider_retry_delay_ms_with_jitter(
    error: &StreamError,
    retry_index: u32,
    max_retry_delay_ms: Option<u64>,
    jitter: Option<f64>,
) -> Option<Result<u64, StreamError>> {
    provider_retry_delay_ms_with_jitter_at(
        error,
        retry_index,
        max_retry_delay_ms,
        jitter,
        std::time::SystemTime::now(),
    )
}

/// Deterministic retry-policy entry point for replay and provider adapters.
///
/// HTTP-date `Retry-After` values need a clock, while numeric delays and
/// exponential backoff do not. Keeping the clock explicit here lets replay
/// callers assert the same policy without depending on wall-clock time.
pub fn provider_retry_delay_ms_with_jitter_at(
    error: &StreamError,
    retry_index: u32,
    max_retry_delay_ms: Option<u64>,
    jitter: Option<f64>,
    now: std::time::SystemTime,
) -> Option<Result<u64, StreamError>> {
    let StreamError::Provider {
        message,
        status,
        headers,
    } = error
    else {
        return None;
    };
    if !is_retryable(status, headers) {
        return Some(Err(error.clone()));
    }
    let server_delay = server_retry_delay(headers, now);
    let delay = server_delay.unwrap_or_else(|| {
        let base = (500_u64.saturating_mul(1_u64 << retry_index.min(4))).min(8_000);
        let random = jitter.unwrap_or_else(rand::random::<f64>).clamp(0.0, 1.0);
        (base as f64 * (1.0 - random * 0.25)).max(0.0).ceil() as u64
    });
    let cap = max_retry_delay_ms.unwrap_or(DEFAULT_MAX_RETRY_DELAY_MS);
    if cap > 0 && delay > cap {
        return Some(Err(StreamError::Provider {
            message: format!("server requested {delay}ms retry delay (max: {cap}ms): {message}"),
            status: *status,
            headers: headers.clone(),
        }));
    }
    Some(Ok(delay))
}

fn is_retryable(status: &Option<u16>, headers: &std::collections::HashMap<String, String>) -> bool {
    match header_value(headers, "x-should-retry") {
        Some(value) if value.eq_ignore_ascii_case("true") => true,
        Some(value) if value.eq_ignore_ascii_case("false") => false,
        _ => {
            status.is_none_or(|value| value == 408 || value == 409 || value == 429 || value >= 500)
        }
    }
}

fn server_retry_delay(
    headers: &std::collections::HashMap<String, String>,
    now: std::time::SystemTime,
) -> Option<u64> {
    header_value(headers, "retry-after-ms")
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).ceil() as u64)
        .or_else(|| {
            header_value(headers, "retry-after")
                .and_then(|value| retry_after_delay_ms_at(value, now))
        })
}

fn retry_after_delay_ms_at(value: &str, now: std::time::SystemTime) -> Option<u64> {
    if let Ok(seconds) = value.parse::<f64>() {
        return Some((seconds.max(0.0) * 1000.0).ceil() as u64);
    }
    let target = httpdate::parse_http_date(value).ok()?;
    target
        .duration_since(now)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}

fn header_value<'a>(
    headers: &'a std::collections::HashMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    headers
        .iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value.as_str()))
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
