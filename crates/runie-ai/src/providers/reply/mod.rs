//! ReplyProvider - loads recorded MiniMax API responses for testing.
//! Maps content/reasoning/tool_calls to Event types. Routes by input keyword.

pub mod generators;
pub mod chat;
pub mod types;
pub mod helpers;
pub mod tests;

pub use types::{
    RecordedResponse, RecordedBaseResp, RecordedChoice, RecordedDelta,
    RecordedMessage, RecordedToolCall, RecordedFunction, RecordedUsage,
    RecordedCompletionDetails,
};
pub use helpers::{extract_content_from_events, format_error_message};

use std::fs;
use std::path::PathBuf;
use runie_core::ProviderError;

use crate::Provider;

/// Routing keywords for selecting recorded response
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scenario {
    Simple,
    Tool,
    Stream,
    StreamTool,
    Error,
    Context,
    LongReasoning,
}

impl Scenario {
    /// Determine scenario from user input message.
    pub fn from_input(input: &str) -> Self {
        let lower = &input.to_lowercase();

        // Use lookup table for keyword -> scenario mapping
        for (keywords, scenario) in SCENARIO_KEYWORDS {
            if keywords.iter().any(|kw| lower.contains(kw)) {
                return *scenario;
            }
        }
        Scenario::Simple
    }
}

type ScenarioKeywords = (&'static [&'static str], Scenario);

const SCENARIO_KEYWORDS: &[ScenarioKeywords] = &[
    (&["calculate", "tool"], Scenario::Tool),
    (&["stream", "count"], Scenario::Stream),
    (&["bash", "ls", "list"], Scenario::StreamTool),
    (&["error", "fail"], Scenario::Error),
    (&["context", "memory"], Scenario::Context),
    (&["long", "peanut", "explain"], Scenario::LongReasoning),
];

/// ReplyProvider loads and replays recorded MiniMax responses.
pub struct ReplyProvider {
    model: String,
    simple_response: RecordedResponse,
    tool_response: RecordedResponse,
    stream_chunks: Vec<String>,
    stream_tool_chunks: Vec<String>,
    error_response: RecordedResponse,
    context_chunks: Vec<String>,
    long_reasoning_chunks: Vec<String>,
}

impl ReplyProvider {
    /// Create a new ReplyProvider with fixtures from the given directory.
    pub fn new(fixtures_dir: PathBuf) -> Result<Self, ProviderError> {
        let simple_response = load_simple_response(&fixtures_dir)?;
        let tool_response = load_tool_response(&fixtures_dir)?;
        let stream_chunks = load_stream_chunks(&fixtures_dir, "minimax_stream.json")?;
        let stream_tool_chunks = load_stream_chunks(&fixtures_dir, "minimax_stream_tool.json")?;
        let error_response = load_error_response(&fixtures_dir)?;
        let context_chunks = load_stream_chunks(&fixtures_dir, "minimax_context.json")?;
        let long_reasoning_chunks = load_stream_chunks(&fixtures_dir, "minimax_long_reasoning.json")?;

        Ok(Self {
            model: "MiniMax-M2.7-highspeed".to_string(),
            simple_response,
            tool_response,
            stream_chunks,
            stream_tool_chunks,
            error_response,
            context_chunks,
            long_reasoning_chunks,
        })
    }

    /// Create ReplyProvider from standard fixtures directory.
    pub fn with_default_fixtures() -> Result<Self, ProviderError> {
        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("providers")
            .join("reply")
            .join("fixtures");
        Self::new(fixtures_dir)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FILE LOADING HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

fn load_simple_response(fixtures_dir: &PathBuf) -> Result<RecordedResponse, ProviderError> {
    let path = fixtures_dir.join("minimax_simple.json");
    let content = fs::read_to_string(&path)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_simple.json: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_simple.json: {}", e)))
}

fn load_tool_response(fixtures_dir: &PathBuf) -> Result<RecordedResponse, ProviderError> {
    let path = fixtures_dir.join("minimax_tool.json");
    let content = fs::read_to_string(&path)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_tool.json: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_tool.json: {}", e)))
}

fn load_error_response(fixtures_dir: &PathBuf) -> Result<RecordedResponse, ProviderError> {
    let path = fixtures_dir.join("minimax_error.json");
    let content = fs::read_to_string(&path)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_error.json: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_error.json: {}", e)))
}

fn load_stream_chunks(fixtures_dir: &PathBuf, filename: &str) -> Result<Vec<String>, ProviderError> {
    let path = fixtures_dir.join(filename);
    let content = fs::read_to_string(&path)
        .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read {}: {}", filename, e)))?;
    Ok(content.lines()
        .filter(|l| l.starts_with("data: "))
        .map(|l| l[6..].to_string())
        .collect())
}
