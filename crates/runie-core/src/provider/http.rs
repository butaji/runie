//! Minimal transport boundary used by concrete providers and replay tests.

use std::{fs, path::Path};

use super::stream_fn::StreamError;
use crate::types::{Model, ProviderResponse, SimpleStreamOptions};

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
        for _ in 0..=retries {
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
                Err(error) => last_error = Some(error),
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
            return Err(StreamError::Api(format!("HTTP {}", self.response.status)));
        }
        Ok(self.response.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

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
}
