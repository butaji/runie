use super::{parse_mcp_event_stream, McpHttpClient, McpStreamEvent, MCP_SESSION_HEADER};

impl McpHttpClient {
    /// Send a streamable-HTTP request and reduce its bounded SSE body into
    /// ordered MCP events. Authentication and session headers match `request`.
    pub async fn stream_events(
        &self,
        request: serde_json::Value,
    ) -> Result<Vec<McpStreamEvent>, String> {
        self.stream_events_with_session(request, None)
            .await
            .map(|(events, _)| events)
    }

    pub(crate) async fn stream_events_with_session(
        &self,
        request: serde_json::Value,
        session_id: Option<&str>,
    ) -> Result<(Vec<McpStreamEvent>, Option<String>), String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut call = client
            .post(&self.endpoint)
            .header("accept", "text/event-stream")
            .json(&request);
        if let Some(token) = &self.bearer_token {
            call = call.bearer_auth(token);
        }
        if let Some(session_id) = session_id {
            call = call.header(MCP_SESSION_HEADER, session_id);
        }
        let response = call
            .send()
            .await
            .map_err(|error| format!("MCP stream request: {error}"))?;
        decode_stream_response(response).await
    }
}

async fn decode_stream_response(
    response: reqwest::Response,
) -> Result<(Vec<McpStreamEvent>, Option<String>), String> {
    let status = response.status();
    let response_session = response
        .headers()
        .get(MCP_SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("MCP stream body: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "MCP stream HTTP status {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    Ok((parse_mcp_event_stream(&body)?, response_session))
}
