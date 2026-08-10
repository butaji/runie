//! Provider-neutral web search boundary. Transport belongs to the owning app.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};

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
}
