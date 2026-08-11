use super::{MCP_HTTP_MAX_RESPONSE_BYTES, MCP_SESSION_HEADER};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHttpClient {
    pub(crate) endpoint: String,
    pub(crate) bearer_token: Option<String>,
    pub(crate) timeout: Duration,
}

impl McpHttpClient {
    pub fn new(
        endpoint: impl Into<String>,
        bearer_token: Option<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err("MCP endpoint must not be empty".into());
        }
        if timeout.is_zero() {
            return Err("MCP timeout must be positive".into());
        }
        Ok(Self {
            endpoint,
            bearer_token,
            timeout,
        })
    }

    pub async fn request(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        self.request_with_session(request, None)
            .await
            .map(|(value, _)| value)
    }

    pub(crate) async fn request_with_session(
        &self,
        request: serde_json::Value,
        session_id: Option<&str>,
    ) -> Result<(serde_json::Value, Option<String>), String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut call = client.post(&self.endpoint).json(&request);
        if let Some(token) = &self.bearer_token {
            call = call.bearer_auth(token);
        }
        if let Some(session_id) = session_id {
            call = call.header(MCP_SESSION_HEADER, session_id);
        }
        let response = call
            .send()
            .await
            .map_err(|error| format!("MCP HTTP request: {error}"))?;
        decode_http_response(response).await
    }
}

async fn decode_http_response(
    response: reqwest::Response,
) -> Result<(serde_json::Value, Option<String>), String> {
    let status = response.status();
    let response_session = response
        .headers()
        .get(MCP_SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("MCP HTTP body: {error}"))?;
    if body.len() > MCP_HTTP_MAX_RESPONSE_BYTES {
        return Err(format!(
            "MCP HTTP response exceeds {} bytes",
            MCP_HTTP_MAX_RESPONSE_BYTES
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "MCP HTTP status {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    serde_json::from_slice(&body)
        .map(|value| (value, response_session))
        .map_err(|error| format!("invalid MCP HTTP response: {error}"))
}
