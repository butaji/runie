//! MiniMax API types.

#[derive(Debug, serde::Deserialize)]
pub struct MiniMaxStreamChunk {
    pub choices: Vec<MiniMaxStreamChoice>,
    #[serde(default)]
    pub usage: Option<crate::token_usage::TokenUsage>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct MiniMaxStreamChoice {
    pub delta: Option<MiniMaxDelta>,
    pub finish_reason: Option<String>,
}

/// Holds pending tool call data when ID is not yet available
#[derive(Debug)]
pub struct PendingToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct MiniMaxDelta {
    pub content: Option<String>,
    pub role: Option<String>,
    pub tool_calls: Option<Vec<MiniMaxToolCallDelta>>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct MiniMaxToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<MiniMaxFunctionDelta>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct MiniMaxFunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MiniMaxResponse {
    pub choices: Vec<MiniMaxChoice>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MiniMaxChoice {
    pub message: MiniMaxMessageResponse,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct MiniMaxMessageResponse {
    pub role: String,
    pub content: Option<String>,
}
