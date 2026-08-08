//! Pure wire helpers for Pi's OpenAI Codex Responses provider.

const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api";
pub const CODEX_WEBSOCKET_BETA_HEADER: &str = "responses_websockets=2026-02-06";

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
}
