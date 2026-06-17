//! FetchDocs tool — fetches documentation from context7.com.

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;

pub struct FetchDocsTool;

const SEARCH_URL: &str = "https://context7.com/api/v2/libs/search";
const DOC_BASE: &str = "https://context7.com";

#[async_trait]
impl Tool for FetchDocsTool {
    fn name(&self) -> &str {
        "fetch_docs"
    }

    fn description(&self) -> &str {
        "Fetch documentation for a library from context7.com."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "library": {
                    "type": "string",
                    "description": "Library name to fetch docs for (e.g., 'ramda', 'lodash')"
                }
            },
            "required": ["library"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let library = input["library"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("library is required"))?;

        let result = fetch_docs(library).await;

        let (content, status) = match result {
            Ok(output) => (output, ToolStatus::Success),
            Err(e) => (
                format!("Error fetching docs for '{}': {}", library, e),
                ToolStatus::Error,
            ),
        };

        Ok(ToolOutput {
            tool_name: "fetch_docs".to_string(),
            tool_args: serde_json::json!({ "library": library }),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status,
        })
    }
}

async fn fetch_docs(library: &str) -> anyhow::Result<String> {
    // Step 1: Search for the library ID
    let search_url = format!("{}?q={}", SEARCH_URL, library);
    let search_resp: serde_json::Value = reqwest::get(&search_url).await?.json().await?;

    let lib_id = search_resp["libs"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|lib| lib["libraryId"].as_str())
        .ok_or_else(|| anyhow::anyhow!("Library '{}' not found on context7", library))?;

    // Step 2: Fetch the documentation
    let doc_url = format!("{}/{}/llms.txt", DOC_BASE, lib_id);
    let doc_resp = reqwest::get(&doc_url).await?;
    let doc_text = doc_resp.text().await?;

    Ok(format!(
        "Source: {}/{}/llms.txt\n\n{}",
        DOC_BASE, lib_id, doc_text
    ))
}
