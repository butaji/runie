use super::*;
use async_stream::stream;
use futures::stream::BoxStream;
use runie_core::{Event as LlmEvent, ToolSchema, ProviderError};

/// Provider that always requests a tool, causing infinite loop until max_turns
pub struct AlwaysToolProvider;
impl AlwaysToolProvider {
    pub fn new() -> Self { AlwaysToolProvider }
}

#[async_trait]
impl Provider for AlwaysToolProvider {
    fn name(&self) -> &str { "always_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that generates duplicate tool calls (same tool+args twice)
pub struct DuplicateToolProvider { _call_count: std::sync::Mutex<u32> }
impl DuplicateToolProvider {
    pub fn new() -> Self { DuplicateToolProvider { _call_count: std::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for DuplicateToolProvider {
    fn name(&self) -> &str { "duplicate_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::ToolCallDelta { id: "call_1".to_string(), name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that generates same tool call across turns
pub struct SameToolAcrossTurnsProvider { turn: tokio::sync::Mutex<u32> }
impl SameToolAcrossTurnsProvider {
    pub fn new() -> Self { SameToolAcrossTurnsProvider { turn: tokio::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for SameToolAcrossTurnsProvider {
    fn name(&self) -> &str { "same_tool_across_turns" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let mut turn = self.turn.lock().await;
        *turn += 1;
        let _current_turn = *turn;

        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that requests permission but never grants
pub struct PermissionNeverGrantedProvider;
impl PermissionNeverGrantedProvider {
    pub fn new() -> Self { PermissionNeverGrantedProvider }
}

#[async_trait]
impl Provider for PermissionNeverGrantedProvider {
    fn name(&self) -> &str { "permission_never_granted" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that tracks and reports token usage
pub struct TokenCountingProvider { turn: tokio::sync::Mutex<u32> }
impl TokenCountingProvider {
    pub fn new() -> Self { TokenCountingProvider { turn: tokio::sync::Mutex::new(0) } }
}

#[async_trait]
impl Provider for TokenCountingProvider {
    fn name(&self) -> &str { "token_counting" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let mut turn = self.turn.lock().await;
        *turn += 1;
        let current_turn = *turn;

        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };
        let tool_name_for_execution = tool_name.clone();

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::Usage { prompt_tokens: 100 * current_turn as usize, completion_tokens: 50 * current_turn as usize, total_tokens: 150 * current_turn as usize };
            yield LlmEvent::MessageEnd;
            if current_turn < 3 {
                yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
                yield LlmEvent::ToolExecutionStart { tool_call_id: format!("call_{}", current_turn), tool_name: tool_name_for_execution, args: serde_json::json!({}), timestamp: chrono::Utc::now() };
                yield LlmEvent::ToolExecutionEnd { tool_call_id: format!("call_{}", current_turn), result: ToolOutput { content: "done".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
            }
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that causes panic in tool prep
pub struct PanickingToolProvider;
impl PanickingToolProvider {
    pub fn new() -> Self { PanickingToolProvider }
}

#[async_trait]
impl Provider for PanickingToolProvider {
    fn name(&self) -> &str { "panicking_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider for simple tool execution tests
pub struct SimpleToolProvider;
impl SimpleToolProvider {
    pub fn new() -> Self { SimpleToolProvider }
}

#[async_trait]
impl Provider for SimpleToolProvider {
    fn name(&self) -> &str { "simple_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
            yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name: tool_name, args: serde_json::json!({}), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(), result: ToolOutput { content: "executed".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that sends tool call with "original" arg for modify hook test
pub struct ModifyArgsProvider;
impl ModifyArgsProvider {
    pub fn new() -> Self { ModifyArgsProvider }
}

#[async_trait]
impl Provider for ModifyArgsProvider {
    fn name(&self) -> &str { "modify_args" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };

        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name.clone(), arguments: r#"{"original": true}"#.to_string() };
            yield LlmEvent::MessageEnd;
            yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name: tool_name, args: serde_json::json!({"modified": true}), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(), result: ToolOutput { content: "modified args".to_string(), metadata: serde_json::json!({}), terminate: false }, timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}
