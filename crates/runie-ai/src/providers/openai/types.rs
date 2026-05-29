//! OpenAI API types.

#[derive(Debug, serde::Deserialize)]
pub struct OpenAIStreamChunk {
    pub choices: Vec<StreamChoice>,
    #[serde(default)]
    pub usage: Option<crate::token_usage::TokenUsage>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct StreamChoice {
    pub delta: Option<Delta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug)]
pub struct PendingToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct Delta {
    pub content: Option<String>,
    pub role: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Choice {
    pub message: MessageResponse,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct MessageResponse {
    pub role: String,
    pub content: Option<String>,
}
