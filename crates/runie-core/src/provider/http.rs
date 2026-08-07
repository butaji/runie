//! Minimal transport boundary used by concrete providers and replay tests.

use std::{fs, path::Path};

use super::stream_fn::StreamError;
use crate::types::{Model, ProviderResponse, SimpleStreamOptions};

const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 60_000;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

#[async_trait::async_trait]
pub trait HttpActor: Send + Sync + 'static {
    async fn post(&self, body: String) -> Result<HttpResponse, StreamError>;

    /// Apply pi-compatible request/response hooks at the transport boundary.
    /// Concrete adapters only implement `post`; the actor owns the side
    /// effect while hooks remain caller-provided observations/transformations.
    #[allow(
        clippy::too_many_lines,
        reason = "the transport boundary keeps payload hooks, timeout, and response hooks atomic"
    )]
    async fn post_with_options(
        &self,
        body: String,
        model: Model,
        options: Option<SimpleStreamOptions>,
    ) -> Result<HttpResponse, StreamError> {
        let mut payload = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| serde_json::Value::String(body.clone()));
        if let Some(hook) = options
            .as_ref()
            .and_then(|options| options.on_payload.clone())
        {
            if let Some(transformed) = hook(payload.clone(), model.clone()).await {
                payload = transformed;
            }
        }
        let request_body = match payload {
            serde_json::Value::String(raw) => raw,
            value => serde_json::to_string(&value).map_err(|error| {
                StreamError::Invalid(format!("payload hook serialization failed: {error}"))
            })?,
        };
        let retries = options.as_ref().and_then(|o| o.max_retries).unwrap_or(0);
        let mut response = None;
        let mut last_error = None;
        for retry_index in 0..=retries {
            match post_once(
                self,
                request_body.clone(),
                options.as_ref().and_then(|o| o.timeout_ms),
            )
            .await
            {
                Ok(value) => {
                    response = Some(value);
                    break;
                }
                Err(error) => {
                    if retry_index < retries {
                        if let Some(decision) = provider_retry_delay_ms_with_jitter(
                            &error,
                            retry_index,
                            options.as_ref().and_then(|o| o.max_retry_delay_ms),
                            options
                                .as_ref()
                                .and_then(|o| o.retry_jitter.as_ref())
                                .map(|jitter| jitter()),
                        ) {
                            let delay = decision?;
                            let signal = options.as_ref().and_then(|o| o.signal.clone());
                            if let Some(hook) = options.as_ref().and_then(|o| o.retry_delay.clone())
                            {
                                hook(delay, signal).await?;
                            } else {
                                abortable_retry_delay(delay, signal).await?;
                            }
                        }
                    }
                    last_error = Some(error);
                }
            }
        }
        let response = response.ok_or_else(|| {
            last_error.expect("at least one provider attempt must produce a result")
        })?;
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
    body: String,
    timeout_ms: Option<u64>,
) -> Result<HttpResponse, StreamError> {
    if let Some(timeout_ms) = timeout_ms {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            actor.post(body),
        )
        .await
        .map_err(|_| StreamError::Network(format!("request timed out after {timeout_ms}ms")))?
    } else {
        actor.post(body).await
    }
}

/// Serves one recorded HTTP response body without opening a socket.
pub struct ReplayHttpActor {
    response: HttpResponse,
}

impl ReplayHttpActor {
    pub fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let body = fs::read_to_string(path).map_err(|e| StreamError::Network(e.to_string()))?;
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
    let StreamError::Provider {
        message,
        status,
        headers,
    } = error
    else {
        return None;
    };
    let should_retry = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("x-should-retry"))
        .map(|(_, value)| value.as_str());
    let retryable = match should_retry {
        Some("true") => true,
        Some("false") => false,
        _ => {
            status.is_none_or(|value| value == 408 || value == 409 || value == 429 || value >= 500)
        }
    };
    if !retryable {
        return Some(Err(error.clone()));
    }
    let server_delay = header_value(headers, "retry-after-ms")
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.max(0.0).ceil() as u64)
        .or_else(|| header_value(headers, "retry-after").and_then(retry_after_delay_ms));
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

fn retry_after_delay_ms(value: &str) -> Option<u64> {
    retry_after_delay_ms_at(value, std::time::SystemTime::now())
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
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

    const SERVER_RETRY_AFTER_MS: &str = "1250.2";
    const EXPECTED_RETRY_AFTER_MS: u64 = 1251;
    const HTTP_DATE_DELAY_SECONDS: u64 = 2;
    const NUMERIC_DELAY_MS: u64 = 250;
    const MILLIS_PER_SECOND: u64 = 1_000;

    struct CapturingHttp {
        body: Arc<Mutex<Option<String>>>,
    }

    struct PendingHttp;

    #[async_trait::async_trait]
    impl HttpActor for PendingHttp {
        async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
            std::future::pending().await
        }
    }

    struct FlakyHttp {
        attempts: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl HttpActor for FlakyHttp {
        async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
            let attempt = self
                .attempts
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if attempt < 2 {
                Err(StreamError::Network("transient".into()))
            } else {
                Ok(HttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: String::new(),
                })
            }
        }
    }

    #[async_trait::async_trait]
    impl HttpActor for CapturingHttp {
        async fn post(&self, body: String) -> Result<HttpResponse, StreamError> {
            *self.body.lock().expect("body lock") = Some(body);
            Ok(HttpResponse {
                status: 201,
                headers: [("x-request-id".into(), "replay-1".into())]
                    .into_iter()
                    .collect(),
                body: String::new(),
            })
        }
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the hook contract test keeps request and response assertions together"
    )]
    async fn post_with_options_runs_payload_and_response_hooks() {
        let body = Arc::new(Mutex::new(None));
        let seen_payload = Arc::new(Mutex::new(None));
        let seen_response = Arc::new(Mutex::new(None));
        let payload_capture = seen_payload.clone();
        let response_capture = seen_response.clone();
        let options = SimpleStreamOptions {
            on_payload: Some(Arc::new(move |payload, _model| {
                *payload_capture.lock().expect("payload lock") = Some(payload);
                async { Some(serde_json::json!({"rewritten": true})) }.boxed()
            })),
            on_response: Some(Arc::new(move |response, _model| {
                *response_capture.lock().expect("response lock") = Some(response);
                async {}.boxed()
            })),
            ..Default::default()
        };
        let http = CapturingHttp { body: body.clone() };
        let response = http
            .post_with_options(
                serde_json::json!({"original": true}).to_string(),
                Model::default(),
                Some(options),
            )
            .await
            .expect("hooked request");

        assert_eq!(
            body.lock().expect("body lock").as_deref(),
            Some(r#"{"rewritten":true}"#)
        );
        assert_eq!(
            seen_payload
                .lock()
                .expect("payload lock")
                .as_ref()
                .and_then(|value| value.get("original"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        let seen = seen_response.lock().expect("response lock");
        assert_eq!(seen.as_ref().map(|response| response.status), Some(201));
        assert_eq!(
            seen.as_ref()
                .and_then(|response| response.headers.get("x-request-id")),
            Some(&"replay-1".to_string())
        );
        assert_eq!(response.status, 201);
    }

    #[tokio::test]
    async fn post_with_options_enforces_pi_timeout_ms() {
        let error = PendingHttp
            .post_with_options(
                "{}".into(),
                Model::default(),
                Some(SimpleStreamOptions {
                    timeout_ms: Some(1),
                    ..Default::default()
                }),
            )
            .await
            .expect_err("pending request must time out");
        assert!(matches!(error, StreamError::Network(message) if message.contains("1ms")));
    }

    #[tokio::test]
    async fn post_with_options_retries_transient_transport_errors() {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let response = FlakyHttp {
            attempts: attempts.clone(),
        }
        .post_with_options(
            "{}".into(),
            Model::default(),
            Some(SimpleStreamOptions {
                max_retries: Some(2),
                ..Default::default()
            }),
        )
        .await
        .expect("third attempt succeeds");
        assert_eq!(response.status, 200);
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn provider_retry_delay_is_abortable() {
        let (sender, receiver) = tokio::sync::watch::channel(true);
        drop(sender);
        assert!(matches!(
            abortable_retry_delay(0, Some(receiver)).await,
            Err(StreamError::Aborted)
        ));
    }

    #[test]
    fn retry_after_accepts_http_dates_and_numeric_seconds() {
        let now = httpdate::parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT").unwrap();
        let target =
            httpdate::fmt_http_date(now + std::time::Duration::from_secs(HTTP_DATE_DELAY_SECONDS));
        assert_eq!(
            retry_after_delay_ms_at(&target, now),
            Some(HTTP_DATE_DELAY_SECONDS * MILLIS_PER_SECOND)
        );
        assert_eq!(retry_after_delay_ms("0.25"), Some(NUMERIC_DELAY_MS));
    }

    #[test]
    fn provider_retry_policy_preserves_headers_and_honors_overrides() {
        let error = StreamError::Provider {
            message: "rate limited".into(),
            status: Some(429),
            headers: [
                ("Retry-After-Ms".into(), SERVER_RETRY_AFTER_MS.into()),
                ("X-Should-Retry".into(), "true".into()),
            ]
            .into_iter()
            .collect(),
        };
        assert!(matches!(
            provider_retry_delay_ms(&error, 0, None),
            Some(Ok(EXPECTED_RETRY_AFTER_MS))
        ));
        assert!(matches!(
            provider_retry_delay_ms(&error, 0, Some(1000)),
            Some(Err(StreamError::Provider {
                status: Some(429),
                ..
            }))
        ));
    }

    #[test]
    fn provider_retry_policy_respects_explicit_no_retry() {
        let error = StreamError::Provider {
            message: "temporary".into(),
            status: Some(500),
            headers: [("x-should-retry".into(), "false".into())]
                .into_iter()
                .collect(),
        };
        assert!(matches!(
            provider_retry_delay_ms(&error, 0, None),
            Some(Err(StreamError::Provider {
                status: Some(500),
                ..
            }))
        ));
    }

    #[test]
    fn exponential_retry_jitter_is_bounded_and_injectable() {
        let error = StreamError::Provider {
            message: "overloaded".into(),
            status: Some(500),
            headers: Default::default(),
        };
        assert!(matches!(
            provider_retry_delay_ms_with_jitter(&error, 0, None, Some(0.0)),
            Some(Ok(500))
        ));
        assert!(matches!(
            provider_retry_delay_ms_with_jitter(&error, 0, None, Some(1.0)),
            Some(Ok(375))
        ));
    }
}
