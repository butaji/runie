//! Typed structs for OpenAI SSE chunk and error parsing.
//!
//! These structs use `serde::Deserialize` to parse JSON directly,
//! replacing manual `serde_json::Value` navigation.

use serde::Deserialize;

/// A parsed OpenAI SSE chunk (streaming response).
///
/// Example JSON:
/// ```json
/// {"choices":[{"delta":{"content":"hi"},"finish_reason":"stop"}]}
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChunkJson {
    #[serde(default)]
    pub choices: Vec<ChoiceJson>,
    #[serde(default)]
    pub usage: Option<UsageJson>,
}

/// A single choice in a streaming chunk.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChoiceJson {
    #[serde(rename = "delta", default)]
    pub delta: DeltaJson,
    #[serde(rename = "finish_reason", default)]
    pub finish_reason: Option<String>,
}

/// The delta content of a streaming chunk.
///
/// Supports both standard OpenAI fields and MiniMax-specific fields:
/// - `content` — standard text delta
/// - `reasoning_content` — MiniMax/OpenAI o-series thinking/reasoning
/// - `reasoning` — alternative field name for reasoning
/// - `tool_calls` — function calling deltas
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeltaJson {
    #[serde(default)]
    pub content: Option<String>,
    /// MiniMax / OpenAI o-series reasoning field.
    #[serde(rename = "reasoning_content", alias = "reasoning", default)]
    pub reasoning_content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub tool_calls: Vec<ToolCallJson>,
}

fn deserialize_nullable_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::Deserialize;
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// A tool call delta inside a delta.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolCallJson {
    /// Index of the tool call within the turn. OpenAI always sends it;
    /// Gemini's OpenAI-compatible stream omits it, so default to 0 (a
    /// missing index can only mean the first/only call).
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "function", default)]
    pub function: FunctionJson,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
}

/// The function part of a tool call delta.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FunctionJson {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageJson {
    #[serde(rename = "prompt_tokens", default)]
    pub prompt_tokens: Option<u64>,
    #[serde(rename = "completion_tokens", default)]
    pub completion_tokens: Option<u64>,
}

/// An SSE error body returned by OpenAI-compatible APIs.
///
/// Uses an untagged enum to handle both wrapped (`error.message`) and flat (`message`) shapes.
///
/// Example JSON (wrapped):
/// ```json
/// {"error":{"message":"Rate limit exceeded","type":"rate_limit_error","code":"rate_limit"}}
/// ```
///
/// Example JSON (flat / MiniMax-style):
/// ```json
/// {"message":"context length exceeded","code":"context_length_exceeded","type":"invalid_request_error"}
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ErrorBodyJson {
    /// Wrapped error with `error` object containing the details.
    Wrapped(WrappedError),
    /// Flat error with fields at the top level (MiniMax-style).
    Flat(FlatError),
}

/// Wrapped error structure with `error` object.
#[derive(Debug, Clone, Deserialize)]
pub struct WrappedError {
    pub error: ErrorDetailJson,
}

/// Flat error structure with fields at top level (MiniMax-style).
#[derive(Debug, Clone, Deserialize)]
pub struct FlatError {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(rename = "retry_after", default)]
    pub retry_after: Option<u64>,
}

/// The error detail inside an error body.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorDetailJson {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(rename = "retry_after", default)]
    pub retry_after: Option<u64>,
}

impl ErrorBodyJson {
    /// Returns the error message.
    pub fn message(&self) -> &str {
        match self {
            ErrorBodyJson::Wrapped(w) => w.error.message.as_deref().unwrap_or("unknown error"),
            ErrorBodyJson::Flat(f) => f.message.as_str(),
        }
    }

    /// Returns the error code string.
    pub fn code(&self) -> &str {
        match self {
            ErrorBodyJson::Wrapped(w) => w.error.code.as_deref().unwrap_or(""),
            ErrorBodyJson::Flat(f) => f.code.as_deref().unwrap_or(""),
        }
    }

    /// Returns the error type string.
    pub fn type_(&self) -> &str {
        match self {
            ErrorBodyJson::Wrapped(w) => w.error.type_.as_deref().unwrap_or(""),
            ErrorBodyJson::Flat(f) => f.type_.as_deref().unwrap_or(""),
        }
    }

    /// Returns the retry-after seconds.
    pub fn retry_after_secs(&self) -> Option<u32> {
        let secs = match self {
            ErrorBodyJson::Wrapped(w) => w.error.retry_after,
            ErrorBodyJson::Flat(f) => f.retry_after,
        };
        secs.map(|v| v as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_json_deserializes_content() {
        let json = serde_json::json!({
            "choices": [{"delta": {"content": "hello"}, "finish_reason": "stop"}]
        });
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("hello"));
        assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn chunk_json_deserializes_reasoning_content() {
        let json = serde_json::json!({
            "choices": [{"delta": {"reasoning_content": "thinking..."}}]
        });
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content.as_deref(),
            Some("thinking...")
        );
    }

    #[test]
    fn chunk_json_deserializes_reasoning_alias() {
        // Some providers use "reasoning" instead of "reasoning_content"
        let json = serde_json::json!({
            "choices": [{"delta": {"reasoning": "thinking..."}}]
        });
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        assert_eq!(
            chunk.choices[0].delta.reasoning_content.as_deref(),
            Some("thinking...")
        );
    }

    #[test]
    fn chunk_json_deserializes_tool_calls() {
        let json = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_abc",
                        "type": "function",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"x\"}"}
                    }]
                }
            }]
        });
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        let tc = &chunk.choices[0].delta.tool_calls[0];
        assert_eq!(tc.index, 0);
        assert_eq!(tc.id.as_deref(), Some("call_abc"));
        assert_eq!(tc.function.name.as_deref(), Some("read_file"));
        assert_eq!(tc.function.arguments.as_deref(), Some("{\"path\":\"x\"}"));
    }

    #[test]
    fn chunk_json_deserializes_usage() {
        let json = serde_json::json!({
            "choices": [{"delta": {}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 100, "completion_tokens": 50}
        });
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(50));
    }

    #[test]
    fn chunk_json_handles_missing_optional_fields() {
        // Empty chunk should deserialize without error
        let json = serde_json::json!({"choices": [{}]});
        let chunk: ChunkJson = serde_json::from_value(json).unwrap();
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].delta.reasoning_content.is_none());
        assert!(chunk.choices[0].delta.tool_calls.is_empty());
        assert!(chunk.choices[0].finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn error_body_json_with_error_wrapper() {
        let json = serde_json::json!({
            "error": {
                "message": "rate limit exceeded",
                "type": "rate_limit_error",
                "code": "rate_limit",
                "retry_after": 60
            }
        });
        let err: ErrorBodyJson = serde_json::from_value(json).unwrap();
        assert_eq!(err.message(), "rate limit exceeded");
        assert_eq!(err.code(), "rate_limit");
        assert_eq!(err.type_(), "rate_limit_error");
        assert_eq!(err.retry_after_secs(), Some(60));
    }

    #[test]
    fn error_body_json_flat_minimax_style() {
        // MiniMax-style flat error body (no `error` wrapper)
        let json = serde_json::json!({
            "message": "context length exceeded",
            "code": "context_length_exceeded",
            "type": "invalid_request_error"
        });
        let err: ErrorBodyJson = serde_json::from_value(json).unwrap();
        assert_eq!(err.message(), "context length exceeded");
        assert_eq!(err.code(), "context_length_exceeded");
        assert_eq!(err.type_(), "invalid_request_error");
        assert!(err.retry_after_secs().is_none());
    }

    #[test]
    fn error_body_message_falls_back_to_flat_field() {
        let json = serde_json::json!({
            "message": "server error",
            "code": "server_error"
        });
        let err: ErrorBodyJson = serde_json::from_value(json).unwrap();
        // error.message is absent, so flat message is used
        assert_eq!(err.message(), "server error");
    }
}
