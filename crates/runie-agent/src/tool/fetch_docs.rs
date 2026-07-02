//! FetchDocs tool — fetches documentation from context7.com.

use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::tool::ToolDef;
use runie_provider::http::build_client;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::time::Instant;

/// Input parameters for fetch_docs tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FetchDocsInput {
    /// Library name to fetch docs for (e.g., 'ramda', 'lodash')
    pub library: String,
}

pub struct FetchDocsTool;

const SEARCH_URL: &str = "https://context7.com/api/v2/libs/search";
const DOC_BASE: &str = "https://context7.com";

impl ToolDef for FetchDocsTool {
    type Input = FetchDocsInput;

    const NAME: &'static str = "fetch_docs";
    const DESCRIPTION: &'static str = "Fetch documentation for a library from context7.com.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, _ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let result = fetch_docs(&input.library).await;

        let (content, status) = match result {
            Ok(output) => (output, ToolStatus::Success),
            Err(e) => (
                format!("Error fetching docs for '{}': {}", input.library, e),
                ToolStatus::Error,
            ),
        };

        ToolOutput {
            tool_name: "fetch_docs".to_owned(),
            tool_args: serde_json::to_value(&input).unwrap_or_default(),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status,
        }
    }
}

async fn fetch_docs(library: &str) -> anyhow::Result<String> {
    let client = build_client();

    // Step 1: Search for the library ID
    let search_url = format!("{}?q={}", SEARCH_URL, library);
    let search_resp: serde_json::Value = client.get(&search_url).send().await?.json().await?;

    let lib_id = search_resp["libs"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|lib| lib["libraryId"].as_str())
        .ok_or_else(|| anyhow::anyhow!("Library '{}' not found on context7", library))?;

    // Step 2: Fetch the documentation
    let doc_url = format!("{}/{}/llms.txt", DOC_BASE, lib_id);
    let doc_resp = client.get(&doc_url).send().await?;
    let doc_text = doc_resp.text().await?;

    Ok(format!(
        "Source: {}/{}/llms.txt\n\n{}",
        DOC_BASE, lib_id, doc_text
    ))
}
