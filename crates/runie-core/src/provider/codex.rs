//! Pure wire helpers for Pi's OpenAI Codex Responses provider.

const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api";
pub const CODEX_WEBSOCKET_BETA_HEADER: &str = "responses_websockets=2026-02-06";

/// Provider-owned continuation state for one cached Codex session/account
/// connection. The socket itself remains an injected transport concern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebSocketContinuation {
    pub last_response_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketFailureDecision {
    RetryConnection,
    RetryContinuation,
    FallbackToSse,
    Propagate,
}

/// Classify the failure boundary used by Pi's Codex WebSocket adapter.
pub fn classify_websocket_failure(
    started: bool,
    error_code: Option<&str>,
    retried_connection_limit: bool,
    retried_missing_continuation: bool,
) -> WebSocketFailureDecision {
    if error_code == Some("previous_response_not_found") && !retried_missing_continuation {
        return WebSocketFailureDecision::RetryContinuation;
    }
    if !started
        && error_code == Some("websocket_connection_limit_reached")
        && !retried_connection_limit
    {
        return WebSocketFailureDecision::RetryConnection;
    }
    if started {
        WebSocketFailureDecision::Propagate
    } else {
        WebSocketFailureDecision::FallbackToSse
    }
}

/// Build the provider-owned WebSocket frame while preserving the request
/// body. A continuation is only attached when the cached connection has a
/// response id, matching Pi's delta request shape.
pub fn response_create_continuation_frame(
    mut body: serde_json::Value,
    continuation: Option<&WebSocketContinuation>,
) -> Result<serde_json::Value, String> {
    let object = body
        .as_object_mut()
        .ok_or_else(|| "Codex response body must be a JSON object".to_owned())?;
    if let Some(response_id) = continuation.and_then(|state| state.last_response_id.as_ref()) {
        object.insert(
            "previous_response_id".to_owned(),
            serde_json::Value::String(response_id.clone()),
        );
    }
    response_create_envelope(body)
}

pub fn resolve_responses_url(base_url: Option<&str>) -> String {
    let raw = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_CODEX_BASE_URL)
        .trim_end_matches('/');
    if raw.ends_with("/codex/responses") {
        raw.to_owned()
    } else if raw.ends_with("/codex") {
        format!("{raw}/responses")
    } else {
        format!("{raw}/codex/responses")
    }
}

pub fn resolve_websocket_url(base_url: Option<&str>) -> Result<String, String> {
    let url = resolve_responses_url(base_url);
    if let Some(rest) = url.strip_prefix("https://") {
        return Ok(format!("wss://{rest}"));
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return Ok(format!("ws://{rest}"));
    }
    if url.starts_with("ws://") || url.starts_with("wss://") {
        return Ok(url);
    }
    Err("unsupported Codex URL scheme".to_owned())
}

pub fn response_create_envelope(mut body: serde_json::Value) -> Result<serde_json::Value, String> {
    let object = body
        .as_object_mut()
        .ok_or_else(|| "Codex response body must be a JSON object".to_owned())?;
    object.insert("type".to_owned(), serde_json::json!("response.create"));
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_codex_http_and_websocket_urls_like_pi() {
        assert_eq!(
            resolve_responses_url(Some("https://example.test/api/")),
            "https://example.test/api/codex/responses"
        );
        assert_eq!(
            resolve_responses_url(Some("https://example.test/codex")),
            "https://example.test/codex/responses"
        );
        assert_eq!(
            resolve_websocket_url(Some("https://example.test/codex/responses")).unwrap(),
            "wss://example.test/codex/responses"
        );
    }

    #[test]
    fn response_create_envelope_overwrites_wire_type_only() {
        let envelope = response_create_envelope(serde_json::json!({
            "model": "gpt-5",
            "type": "response.other"
        }))
        .unwrap();
        assert_eq!(envelope["type"], "response.create");
        assert_eq!(envelope["model"], "gpt-5");
        assert!(response_create_envelope(serde_json::json!(null)).is_err());
    }

    #[test]
    fn continuation_frame_and_failure_policy_match_codex_boundaries() {
        let frame = response_create_continuation_frame(
            serde_json::json!({"input": []}),
            Some(&WebSocketContinuation {
                last_response_id: Some("resp-1".into()),
            }),
        )
        .unwrap();
        assert_eq!(frame["type"], "response.create");
        assert_eq!(frame["previous_response_id"], "resp-1");
        assert_eq!(
            classify_websocket_failure(
                false,
                Some("websocket_connection_limit_reached"),
                false,
                false
            ),
            WebSocketFailureDecision::RetryConnection
        );
        assert_eq!(
            classify_websocket_failure(false, Some("previous_response_not_found"), false, false),
            WebSocketFailureDecision::RetryContinuation
        );
        assert_eq!(
            classify_websocket_failure(false, Some("transport_error"), true, true),
            WebSocketFailureDecision::FallbackToSse
        );
        assert_eq!(
            classify_websocket_failure(true, Some("transport_error"), true, true),
            WebSocketFailureDecision::Propagate
        );
    }
}
