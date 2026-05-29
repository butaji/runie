//! Anthropic API types.

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum AnthropicStreamChunk {
    #[serde(rename = "message_start")]
    MessageStart(MessageStartBlock),
    #[serde(rename = "content_block_start")]
    ContentBlockStart(ContentBlockStart),
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta(ContentBlockDelta),
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta(MessageDelta),
    #[serde(rename = "message_stop")]
    MessageStop,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct MessageStartBlock { pub message: MessageStart }

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct MessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    #[serde(rename = "stop_reason")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ContentBlockStart {
    pub index: usize,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ContentBlockDelta {
    pub index: usize,
    #[serde(rename = "type")]
    pub type_: String,
    pub text: Option<String>,
    #[serde(rename = "partial_json")]
    pub partial_json: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct MessageDelta {
    #[serde(rename = "type")]
    pub type_: String,
    pub usage: Option<DeltaUsage>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct DeltaUsage {
    #[serde(rename = "input_tokens")]
    pub input_tokens: usize,
    #[serde(rename = "output_tokens")]
    pub output_tokens: usize,
}

#[derive(Debug, serde::Deserialize, Clone)]
#[allow(dead_code)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub role: String,
    pub content: Vec<ResponseContent>,
    pub model: String,
    #[serde(rename = "stop_reason")]
    pub stop_reason: Option<String>,
    pub usage: ResponseUsage,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResponseContent {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct ResponseUsage {
    #[serde(rename = "input_tokens")]
    pub input_tokens: usize,
    #[serde(rename = "output_tokens")]
    pub output_tokens: usize,
}
