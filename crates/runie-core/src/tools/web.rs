//! Provider-neutral web search boundary. Transport belongs to the owning app.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::time::Duration;

const MAX_WEB_SEARCH_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebSearchResponse {
    #[serde(default)]
    pub results: Vec<WebSearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebSourceCard {
    pub rank: u32,
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Build renderer-neutral source cards from the provider-neutral result set.
/// Cards are data; TUI and noninteractive consumers choose their own layout.
pub fn source_cards(response: &WebSearchResponse) -> Vec<WebSourceCard> {
    response
        .results
        .iter()
        .enumerate()
        .filter(|(_, result)| !result.url.trim().is_empty())
        .map(|(index, result)| WebSourceCard {
            rank: index as u32 + 1,
            title: if result.title.trim().is_empty() {
                result.url.clone()
            } else {
                result.title.clone()
            },
            url: result.url.clone(),
            snippet: result.snippet.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchHttpClient {
    pub endpoint: String,
    pub bearer_token: Option<String>,
    pub timeout: Duration,
}

impl WebSearchHttpClient {
    pub fn new(
        endpoint: impl Into<String>,
        bearer_token: Option<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err("web search endpoint must not be empty".into());
        }
        if timeout.is_zero() {
            return Err("web search timeout must be positive".into());
        }
        Ok(Self {
            endpoint,
            bearer_token,
            timeout,
        })
    }
    pub async fn search(&self, request: WebSearchRequest) -> Result<WebSearchResponse, String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut call = client.post(&self.endpoint).json(&request);
        if let Some(token) = &self.bearer_token {
            call = call.bearer_auth(token);
        }
        let response = call
            .send()
            .await
            .map_err(|error| format!("web search request: {error}"))?;
        let status = response.status();
        if let Some(length) = response.content_length() {
            validate_response_size(length as usize)?;
        }
        let body = response
            .text()
            .await
            .map_err(|error| format!("web search body: {error}"))?;
        validate_response_size(body.len())?;
        if !status.is_success() {
            return Err(format!("web search HTTP status {status}: {body}"));
        }
        serde_json::from_str(&body).map_err(|error| format!("invalid web search response: {error}"))
    }

    pub fn hook(self) -> crate::tools::executor::WebSearchHook {
        std::sync::Arc::new(move |request| {
            let client = self.clone();
            Box::pin(async move {
                serde_json::to_value(client.search(request).await?)
                    .map_err(|error| error.to_string())
            })
        })
    }
}

fn validate_response_size(bytes: usize) -> Result<(), String> {
    if bytes > MAX_WEB_SEARCH_RESPONSE_BYTES {
        return Err(format!(
            "web search response exceeds {} bytes",
            MAX_WEB_SEARCH_RESPONSE_BYTES
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WebSearchRequest {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    5
}

#[derive(Default)]
pub struct WebSearchTool;

#[async_trait::async_trait]
impl AgentTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }
    fn label(&self) -> &str {
        "Web search"
    }
    fn description(&self) -> &str {
        "Search the web through the owning application."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "minLength": 1 },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 20 }
            },
            "required": ["query"]
        }))
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let request: WebSearchRequest = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid web search: {error}"))?;
        if request.query.trim().is_empty() {
            return Err("query must not be empty".into());
        }
        if !(1..=20).contains(&request.max_results) {
            return Err("max_results must be between 1 and 20".into());
        }
        Ok(())
    }
    async fn execute(
        &self,
        _: &str,
        _: serde_json::Value,
        _: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("web_search requires an owning web search hook".into())
    }
}

pub(crate) fn result(value: serde_json::Value) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: value.to_string(),
        }],
        details: value,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_bounded_queries() {
        let tool = WebSearchTool;
        assert!(tool
            .validate_arguments(&serde_json::json!({"query":"rust actors"}))
            .is_ok());
        assert!(tool
            .validate_arguments(&serde_json::json!({"query":" "}))
            .is_err());
        assert!(tool
            .validate_arguments(&serde_json::json!({"query":"x","max_results":21}))
            .is_err());
    }

    #[test]
    fn source_cards_are_stable_ranked_data() {
        let cards = source_cards(&WebSearchResponse {
            results: vec![
                WebSearchResult {
                    title: "".into(),
                    url: "https://one.test".into(),
                    snippet: "first".into(),
                },
                WebSearchResult {
                    title: "ignored".into(),
                    url: " ".into(),
                    snippet: String::new(),
                },
                WebSearchResult {
                    title: "Three".into(),
                    url: "https://three.test".into(),
                    snippet: "third".into(),
                },
            ],
        });
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].rank, 1);
        assert_eq!(cards[0].title, "https://one.test");
        assert_eq!(cards[1].rank, 3);
    }

    #[test]
    fn response_size_boundary_rejects_unbounded_provider_data() {
        assert!(validate_response_size(MAX_WEB_SEARCH_RESPONSE_BYTES).is_ok());
        assert!(validate_response_size(MAX_WEB_SEARCH_RESPONSE_BYTES + 1).is_err());
    }

    #[tokio::test]
    async fn http_client_decodes_citation_results() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 2048];
            let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut request).await;
            let body = r#"{"results":[{"title":"Rust","url":"https://rust-lang.org","snippet":"language"}]}"#;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}", body.len(), body);
            tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await.unwrap();
        });
        let client =
            WebSearchHttpClient::new(format!("http://{address}"), None, Duration::from_secs(1))
                .unwrap();
        let response = client
            .search(WebSearchRequest {
                query: "rust".into(),
                max_results: 1,
            })
            .await
            .unwrap();
        tasks.join_next().await.unwrap().unwrap();
        assert_eq!(response.results[0].url, "https://rust-lang.org");
    }
}
