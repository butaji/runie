use super::*;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

const SERVER_RETRY_AFTER_MS: &str = "1250.2";
const EXPECTED_RETRY_AFTER_MS: u64 = 1251;
const HTTP_DATE_DELAY_SECONDS: u64 = 2;
const NUMERIC_DELAY_MS: u64 = 250;
const MILLIS_PER_SECOND: u64 = 1_000;

type CapturedRequestOptions = (
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<u64>,
    std::collections::HashMap<String, serde_json::Value>,
);

struct CapturingHttp {
    body: Arc<Mutex<Option<String>>>,
}

struct PendingHttp;

struct HeaderCapturingHttp {
    headers: Arc<Mutex<Option<std::collections::HashMap<String, String>>>>,
    transport: Arc<Mutex<Option<ProviderTransport>>>,
    request_options: Arc<Mutex<Option<CapturedRequestOptions>>>,
}

#[async_trait::async_trait]
impl HttpActor for PendingHttp {
    async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
        std::future::pending().await
    }
}

#[async_trait::async_trait]
impl HttpActor for HeaderCapturingHttp {
    async fn post(&self, _body: String) -> Result<HttpResponse, StreamError> {
        unreachable!("post_request is the exercised boundary")
    }

    async fn post_request(&self, request: HttpRequest) -> Result<HttpResponse, StreamError> {
        *self.headers.lock().expect("headers lock") = Some(request.headers);
        *self.transport.lock().expect("transport lock") = request.transport;
        *self.request_options.lock().expect("request options lock") = Some((
            request.session_id,
            request.api_key,
            request.temperature,
            request.max_tokens,
            request.sampling_params,
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: String::new(),
        })
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

    assert_hooked_response(&body, &seen_payload, &seen_response, &response);
}

#[tokio::test]
async fn shared_http_boundary_applies_model_effort_profile() {
    let body = Arc::new(Mutex::new(None));
    let http = CapturingHttp { body: body.clone() };
    let model = Model {
        provider: "anthropic".into(),
        thinking_level_map: Some(crate::types::ThinkingLevelMap {
            high: Some("extended".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    http.post_with_options(
        r#"{"model":"demo"}"#.into(),
        model,
        Some(SimpleStreamOptions {
            reasoning: Some(crate::types::ThinkingLevel::High),
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&body.lock().unwrap().clone().unwrap()).unwrap();
    assert_eq!(payload["output_config"]["effort"], "extended");
}

fn assert_hooked_response(
    body: &Arc<Mutex<Option<String>>>,
    seen_payload: &Arc<Mutex<Option<serde_json::Value>>>,
    seen_response: &Arc<Mutex<Option<ProviderResponse>>>,
    response: &HttpResponse,
) {
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
async fn post_with_options_forwards_request_headers_to_transport() {
    let headers = Arc::new(Mutex::new(None));
    let transport = Arc::new(Mutex::new(None));
    let request_options = Arc::new(Mutex::new(None));
    HeaderCapturingHttp {
        headers: headers.clone(),
        transport: transport.clone(),
        request_options: request_options.clone(),
    }
    .post_with_options(
        "{}".into(),
        Model::default(),
        Some(header_forward_options()),
    )
    .await
    .expect("request succeeds");
    assert_forwarded_headers(&headers, &transport, &request_options);
}

fn header_forward_options() -> SimpleStreamOptions {
    SimpleStreamOptions {
        headers: Some([(String::from("x-trace"), String::from("replay-1"))].into()),
        transport: Some(ProviderTransport::Sse),
        session_id: Some("session-1".into()),
        api_key: Some("secret-1".into()),
        temperature: Some(0.6),
        max_tokens: Some(64),
        sampling_params: Some([(String::from("top_p"), serde_json::json!(0.9))].into()),
        ..Default::default()
    }
}

fn assert_forwarded_headers(
    headers: &Arc<Mutex<Option<std::collections::HashMap<String, String>>>>,
    transport: &Arc<Mutex<Option<ProviderTransport>>>,
    request_options: &Arc<Mutex<Option<CapturedRequestOptions>>>,
) {
    assert_eq!(
        headers
            .lock()
            .expect("headers lock")
            .as_ref()
            .and_then(|headers| headers.get("x-trace")),
        Some(&String::from("replay-1"))
    );
    assert_eq!(
        *transport.lock().expect("transport lock"),
        Some(ProviderTransport::Sse)
    );
    let options = request_options.lock().expect("request options lock");
    let (session_id, api_key, temperature, max_tokens, sampling_params) =
        options.as_ref().expect("typed request options");
    assert_eq!(session_id.as_deref(), Some("session-1"));
    assert_eq!(api_key.as_deref(), Some("secret-1"));
    assert_eq!(*temperature, Some(0.6));
    assert_eq!(*max_tokens, Some(64));
    assert_eq!(sampling_params.get("top_p"), Some(&serde_json::json!(0.9)));
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
async fn default_http_boundary_rejects_unsupported_websocket_transport() {
    let error = PendingHttp
        .post_request(HttpRequest {
            body: "{}".into(),
            session_id: None,
            api_key: None,
            temperature: None,
            max_tokens: None,
            reasoning: None,
            sampling_params: Default::default(),
            headers: Default::default(),
            env: Default::default(),
            metadata: Default::default(),
            transport: Some(ProviderTransport::Websocket),
            cache_retention: None,
            websocket_connect_timeout_ms: None,
        })
        .await
        .expect_err("generic HTTP must not silently emulate websocket");
    assert!(error.to_string().contains("provider-specific websocket"));
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
    assert_eq!(retry_after_delay_ms_at("0.25", now), Some(NUMERIC_DELAY_MS));
}

#[test]
fn retry_policy_accepts_an_explicit_clock_for_http_dates() {
    let now = httpdate::parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT").unwrap();
    let target =
        httpdate::fmt_http_date(now + std::time::Duration::from_secs(HTTP_DATE_DELAY_SECONDS));
    let error = StreamError::Provider {
        message: "rate limited".into(),
        status: Some(429),
        headers: [
            ("Retry-After".into(), target),
            ("X-Should-Retry".into(), "true".into()),
        ]
        .into_iter()
        .collect(),
    };
    assert!(matches!(
        super::provider_retry_delay_ms_with_jitter_at(&error, 0, None, Some(0.0), now),
        Some(Ok(value)) if value == HTTP_DATE_DELAY_SECONDS * MILLIS_PER_SECOND
    ));
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

#[test]
fn model_declared_effort_maps_to_provider_wire_value() {
    let model = Model {
        thinking_level_map: Some(crate::types::ThinkingLevelMap {
            high: Some("extended".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let options = SimpleStreamOptions {
        reasoning: Some(crate::types::ThinkingLevel::High),
        ..Default::default()
    };
    assert_eq!(
        mapped_reasoning(&model, Some(&options)),
        Some("extended".into())
    );
    let unmapped = SimpleStreamOptions {
        reasoning: Some(crate::types::ThinkingLevel::Low),
        ..options.clone()
    };
    assert_eq!(mapped_reasoning(&model, Some(&unmapped)), None);
    let payload = with_model_effort(
        serde_json::json!({"model":"demo", "input": []}),
        &model,
        Some(&options),
        "reasoning_effort",
    );
    assert_eq!(payload["reasoning_effort"], "extended");
    let untouched = with_model_effort(serde_json::json!([]), &model, Some(&options), "effort");
    assert!(untouched.is_array());
}

#[test]
fn typed_provider_effort_fields_preserve_adapter_wire_keys() {
    let model = Model {
        thinking_level_map: Some(crate::types::ThinkingLevelMap {
            high: Some("extended".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let options = SimpleStreamOptions {
        reasoning: Some(crate::types::ThinkingLevel::High),
        ..Default::default()
    };

    let responses = with_provider_effort(
        serde_json::json!({}),
        &model,
        Some(&options),
        EffortWireField::ReasoningEffort,
    );
    let chat = with_provider_effort(
        serde_json::json!({}),
        &model,
        Some(&options),
        EffortWireField::Reasoning,
    );
    assert_eq!(
        responses,
        serde_json::json!({"reasoning_effort": "extended"})
    );
    assert_eq!(chat, serde_json::json!({"reasoning": "extended"}));
}

#[test]
fn provider_transport_options_are_replayable_data() {
    for (transport, wire) in [
        (ProviderTransport::Sse, "sse"),
        (ProviderTransport::WebsocketCached, "websocket-cached"),
        (ProviderTransport::Auto, "auto"),
    ] {
        assert_eq!(serde_json::to_value(transport).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<ProviderTransport>(serde_json::json!(wire)).unwrap(),
            transport
        );
    }
    assert_eq!(serde_json::to_value(CacheRetention::Long).unwrap(), "long");
}
