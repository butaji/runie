//! Context7 documentation fetcher.
//!
//! Searches context7.com for library documentation and returns the
//! `llms.txt` content — condensed, LLM-friendly docs.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const MAX_DOC_BYTES: usize = 50 * 1024;
const SEARCH_URL: &str = "https://context7.com/api/v2/libs/search";

static CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Clear the global documentation cache.
/// Primarily useful in tests.
pub fn clear_cache() {
    if let Ok(mut cache) = CACHE.lock() {
        cache.clear();
    }
}

/// Client for fetching documentation from context7.com.
pub struct Context7Client {
    client: reqwest::blocking::Client,
}

impl Context7Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Fetch documentation for a library query.
    ///
    /// 1. Checks the in-memory cache.
    /// 2. Searches context7.com for the library.
    /// 3. Fetches the `llms.txt` document.
    /// 4. Truncates to ~50 KB and caches the result.
    pub fn fetch(&self, query: &str) -> anyhow::Result<String> {
        if let Ok(cache) = CACHE.lock() {
            if let Some(cached) = cache.get(query) {
                return Ok(cached.clone());
            }
        }

        let library = self.search(query)?;
        let docs = self.fetch_docs(&library)?;
        let truncated = truncate_docs(&docs, MAX_DOC_BYTES);
        let output = format!(
            "Source: https://context7.com/{}/llms.txt\n\n{}",
            library, truncated
        );

        if let Ok(mut cache) = CACHE.lock() {
            cache.insert(query.to_string(), output.clone());
        }

        Ok(output)
    }

    fn search(&self, query: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .get(SEARCH_URL)
            .query(&[("q", query)])
            .send()?;

        let json: serde_json::Value = resp.json()?;
        parse_search_response(&json)
            .ok_or_else(|| anyhow::anyhow!("No results found for: {}", query))
    }

    fn fetch_docs(&self, library: &str) -> anyhow::Result<String> {
        let url = format!("https://context7.com/{}/llms.txt", library);
        let resp = self.client.get(&url).send()?;
        let text = resp.text()?;
        Ok(text)
    }
}

/// Extract the first library ID from a context7 search response.
pub fn parse_search_response(json: &serde_json::Value) -> Option<String> {
    json.get("results")?
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(|s| s.to_string())
}

/// Truncate documentation to a byte limit, adding a suffix when truncated.
pub fn truncate_docs(docs: &str, limit: usize) -> String {
    if docs.len() <= limit {
        docs.to_string()
    } else {
        let mut truncated: String = docs.chars().take(limit).collect();
        truncated.push_str("\n\n[Documentation truncated to fit context window]");
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_response_extracts_id() {
        let json = serde_json::json!({
            "results": [{ "id": "tokio", "name": "Tokio" }]
        });
        assert_eq!(parse_search_response(&json), Some("tokio".to_string()));
    }

    #[test]
    fn parse_search_response_no_results() {
        let json = serde_json::json!({ "results": [] });
        assert_eq!(parse_search_response(&json), None);
    }

    #[test]
    fn parse_search_response_missing_id() {
        let json = serde_json::json!({
            "results": [{ "name": "Tokio" }]
        });
        assert_eq!(parse_search_response(&json), None);
    }

    #[test]
    fn truncate_docs_under_limit() {
        let docs = "short docs";
        let result = truncate_docs(docs, MAX_DOC_BYTES);
        assert_eq!(result, "short docs");
    }

    #[test]
    fn truncate_docs_respects_limit() {
        let docs = "a".repeat(MAX_DOC_BYTES + 1000);
        let result = truncate_docs(&docs, MAX_DOC_BYTES);
        assert!(result.len() <= MAX_DOC_BYTES + 100); // allow suffix
        assert!(result.contains("[Documentation truncated to fit context window]"));
    }

    #[test]
    fn cache_hits_avoid_refetch() {
        clear_cache();
        {
            let mut cache = CACHE.lock().unwrap();
            cache.insert("tokio".to_string(), "cached tokio docs".to_string());
        }

        let client = Context7Client::new();
        let result = client.fetch("tokio").unwrap();
        assert_eq!(result, "cached tokio docs");
    }
}
