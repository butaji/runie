//! Recorded response types for replay.

use serde::Deserialize;

/// Recorded MiniMax response types for replay
#[derive(Debug, Deserialize)]
pub struct RecordedResponse {
    pub id: Option<String>,
    #[serde(default)]
    pub choices: Option<Vec<RecordedChoice>>,
    pub usage: Option<RecordedUsage>,
    #[serde(default)]
    pub base_resp: Option<RecordedBaseResp>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedBaseResp {
    pub status_code: i32,
    pub status_msg: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordedChoice {
    pub finish_reason: Option<String>,
    pub index: usize,
    #[serde(default)]
    pub delta: Option<RecordedDelta>,
    pub message: Option<RecordedMessage>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedDelta {
    pub content: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<RecordedToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<RecordedToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedToolCall {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<RecordedFunction>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedUsage {
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
    #[serde(rename = "prompt_tokens", default)]
    pub prompt_tokens: Option<usize>,
    #[serde(rename = "completion_tokens", default)]
    pub completion_tokens: Option<usize>,
    #[serde(rename = "completion_tokens_details", default)]
    pub completion_tokens_details: Option<RecordedCompletionDetails>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedCompletionDetails {
    #[serde(rename = "reasoning_tokens")]
    pub reasoning_tokens: Option<usize>,
}

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
