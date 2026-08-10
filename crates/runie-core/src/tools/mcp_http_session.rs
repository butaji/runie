use super::{McpHttpClient, MCP_SESSION_HEADER};

/// Stateful streamable-HTTP MCP session. Session ownership is explicit and
/// closure is awaited by the caller, so cleanup is not detached.
pub struct McpHttpSession {
    client: McpHttpClient,
    session_id: Option<String>,
}

impl McpHttpSession {
    pub fn new(client: McpHttpClient) -> Self {
        Self {
            client,
            session_id: None,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn request(
        &mut self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (value, session_id) = self
            .client
            .request_with_session(request, self.session_id.as_deref())
            .await?;
        if session_id.is_some() {
            self.session_id = session_id;
        }
        Ok(value)
    }

    pub async fn close(self) -> Result<(), String> {
        let Some(session_id) = self.session_id else {
            return Ok(());
        };
        let client = reqwest::Client::builder()
            .timeout(self.client.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut request = client
            .delete(&self.client.endpoint)
            .header(MCP_SESSION_HEADER, session_id);
        if let Some(token) = &self.client.bearer_token {
            request = request.bearer_auth(token);
        }
        request
            .send()
            .await
            .map_err(|error| format!("MCP session close: {error}"))?
            .error_for_status()
            .map(|_| ())
            .map_err(|error| format!("MCP session close: {error}"))
    }
}
