//! OpenAI Chat Completions request-body construction.

use super::OpenAiProvider;
use runie_core::message::{ChatMessage, Part, ToolCall};
use runie_core::provider::ModelMeta;

const MAX_TOOL_CALL_ID_LEN: usize = 64;

#[derive(Debug, Clone, serde::Serialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunction,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OpenAiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<OpenAiToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

pub fn build_request_body(
    provider: &OpenAiProvider,
    messages: &[ChatMessage],
) -> serde_json::Value {
    let normalized = super::normalize::normalize_messages(messages.to_vec());
    let meta = request_metadata(provider.model_meta());
    let mut body = serde_json::json!({
        "model": provider.model(),
        "messages": serialize_messages(&normalized, meta.supports_system),
        "stream": true,
    });
    if let Some(limit) = meta.output_limit {
        if meta.is_thinking {
            body["max_completion_tokens"] = limit.into();
        } else {
            body["max_tokens"] = limit.into();
        }
    }
    if meta.supports_tools {
        let tools = provider.tools();
        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
            body["tool_choice"] = provider.tool_choice().cloned().unwrap_or_else(|| {
                serde_json::Value::String("auto".to_string())
            });
        }
    }
    body
}

fn request_metadata(meta: Option<&ModelMeta>) -> RequestMeta {
    let m = match meta {
        Some(m) => m,
        None => return RequestMeta::default(),
    };
    RequestMeta {
        is_thinking: m.supports_thinking,
        output_limit: if m.max_output_tokens > 0 {
            Some(m.max_output_tokens)
        } else {
            None
        },
        supports_system: m.supports_system,
        supports_tools: m.supports_tools,
    }
}

#[derive(Debug)]
struct RequestMeta {
    is_thinking: bool,
    output_limit: Option<usize>,
    supports_system: bool,
    supports_tools: bool,
}

impl Default for RequestMeta {
    fn default() -> Self {
        Self {
            is_thinking: false,
            output_limit: None,
            supports_system: true,
            supports_tools: false,
        }
    }
}

fn serialize_messages(messages: &[ChatMessage], supports_system: bool) -> Vec<serde_json::Value> {
    let valid_tool_ids = collect_tool_call_ids(messages);
    messages
        .iter()
        .map(|m| message_to_openai(m, &valid_tool_ids, supports_system))
        .collect()
}

fn collect_tool_call_ids(messages: &[ChatMessage]) -> std::collections::HashSet<String> {
    messages
        .iter()
        .flat_map(|m| m.tool_calls().into_iter().map(|c| c.id.clone()))
        .collect()
}

fn message_to_openai(
    message: &ChatMessage,
    valid_tool_ids: &std::collections::HashSet<String>,
    supports_system: bool,
) -> serde_json::Value {
    let role = message.role.as_str();
    if role == "tool" {
        return serialize_tool_message(message, valid_tool_ids);
    }

    let role = if role == "system" && !supports_system {
        "user"
    } else {
        role
    };

    let content = message.content();
    let tool_calls: Vec<ToolCall> = message.tool_calls();
    let has_tool_calls = !tool_calls.is_empty();

    serde_json::to_value(OpenAiMessage {
        role: role.to_string(),
        content: if has_tool_calls {
            Some(String::new())
        } else {
            Some(content.clone())
        },
        tool_calls: tool_calls.iter().map(|c| tool_call_to_openai(c)).collect(),
        tool_call_id: None,
    })
    .unwrap_or_else(|_| fallback_message(role, &content))
}

fn serialize_tool_message(
    message: &ChatMessage,
    valid_tool_ids: &std::collections::HashSet<String>,
) -> serde_json::Value {
    let content = message.content();
    match &message.tool_call_id {
        Some(id) if valid_tool_ids.contains(id) => serde_json::json!({
            "role": "tool",
            "content": content,
            "tool_call_id": sanitize_tool_call_id(id),
        }),
        _ => serde_json::json!({
            "role": "user",
            "content": content,
        }),
    }
}

fn tool_call_to_openai(call: &ToolCall) -> OpenAiToolCall {
    OpenAiToolCall {
        id: sanitize_tool_call_id(&call.id),
        call_type: "function".to_string(),
        function: OpenAiFunction {
            name: call.name.clone(),
            arguments: call.args.to_string(),
        },
    }
}

fn sanitize_tool_call_id(id: &str) -> String {
    let sanitized: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if sanitized.len() > MAX_TOOL_CALL_ID_LEN {
        sanitized[..MAX_TOOL_CALL_ID_LEN].to_string()
    } else {
        sanitized
    }
}

fn fallback_message(role: &str, content: &str) -> serde_json::Value {
    serde_json::json!({"role": role, "content": content})
}

pub async fn send_openai_request(
    client: &reqwest::Client,
    provider: &OpenAiProvider,
    messages: &[ChatMessage],
) -> anyhow::Result<reqwest::Response> {
    let url = format!("{}/chat/completions", provider.base_url);
    let body = build_request_body(provider, messages);

    let response = client
        .post(&url)
        .header(
            "Authorization",
            format!("Bearer {}", provider.api_key.trim()),
        )
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenAI request failed: {}", e))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenAI error: {}", text));
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::message::{MessageMetadata, Role};
    use runie_core::provider::ModelMeta;

    fn provider() -> OpenAiProvider {
        OpenAiProvider::new("sk".to_string(), "gpt-4o")
    }

    #[test]
    fn sanitize_tool_call_id_strips_invalid_chars() {
        assert_eq!(sanitize_tool_call_id("call_abc-123"), "call_abc-123");
        assert_eq!(sanitize_tool_call_id("call::abc"), "callabc");
        assert_eq!(sanitize_tool_call_id("a".repeat(100).as_str()).len(), 64);
    }

    #[test]
    fn assistant_message_with_tool_calls_omits_content() {
        // When assistant has both text and tool_calls, content is set to empty.
        // Dangling tool calls (no matching result) are removed by sanitize first,
        // so content is preserved when tool_calls become empty.
        let msg = ChatMessage {
            role: Role::Assistant,
            timestamp: 0.0,
            id: String::new(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![
                Part::Text { content: "I'll read it.".into() },
                Part::ToolCall { id: "call_1".into(), name: "read_file".into(), args: serde_json::json!({"path":"Cargo.toml"}) },
            ],
        };
        let body = build_request_body(&provider(), &[ChatMessage::user("hi".to_string()), msg]);
        let serialized = &body["messages"].as_array().unwrap()[1];
        assert_eq!(serialized["role"], "assistant");
        // Dangling tool call was removed, so content is preserved
        assert_eq!(serialized["content"], "I'll read it.");
        // No tool_calls since the dangling one was removed
        assert!(serialized["tool_calls"].as_array().map(|a| a.is_empty()).unwrap_or(true));
    }

    #[test]
    fn assistant_message_without_tool_calls_keeps_content() {
        // Assistant without tool_calls keeps its content.
        // Needs a user message first (sanitize ensures user/system first).
        let msg = ChatMessage::assistant("I'll read it.".to_string());
        let body = build_request_body(&provider(), &[ChatMessage::user("hi".to_string()), msg]);
        let serialized = &body["messages"].as_array().unwrap()[1];
        assert_eq!(serialized["role"], "assistant");
        assert_eq!(serialized["content"], "I'll read it.");
    }

    #[test]
    fn thinking_model_uses_max_completion_tokens() {
        static META: ModelMeta = ModelMeta::new("o1").with_thinking().with_output_limit(4096);
        let p = OpenAiProvider::new("sk".to_string(), "o1").with_model_meta(&META);
        let body = build_request_body(
            &p,
            &[
                ChatMessage::system("sys".to_string()),
                ChatMessage::user("hi".to_string()),
            ],
        );
        assert_eq!(body["max_completion_tokens"], 4096);
        assert!(body["max_tokens"].is_null());
    }

    #[test]
    fn no_system_model_maps_system_to_user() {
        static META: ModelMeta = ModelMeta::new("custom").with_no_system();
        let p = OpenAiProvider::new("sk".to_string(), "custom").with_model_meta(&META);
        let body = build_request_body(&p, &[ChatMessage::system("sys".to_string())]);
        let serialized = body["messages"].as_array().unwrap();
        assert_eq!(serialized[0]["role"], "user");
        assert_eq!(serialized[0]["content"], "sys");
    }

    #[test]
    fn non_thinking_model_with_output_limit_uses_max_tokens() {
        static META: ModelMeta = ModelMeta::new("gpt-4o").with_output_limit(2048);
        let p = OpenAiProvider::new("sk".to_string(), "gpt-4o").with_model_meta(&META);
        let body = build_request_body(&p, &[ChatMessage::user("hi".to_string())]);
        assert_eq!(body["max_tokens"], 2048);
        assert!(body["max_completion_tokens"].is_null());
    }

    #[test]
    fn supports_tools_emits_tools_and_tool_choice() {
        static META: ModelMeta = ModelMeta::new("gpt-4o").with_tools(true);
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": { "name": "bash", "description": "run shell commands" }
        })];
        let p = OpenAiProvider::new("sk".to_string(), "gpt-4o")
            .with_model_meta(&META)
            .with_tools(tools);
        let body = build_request_body(&p, &[ChatMessage::user("hi".to_string())]);
        assert!(body["tools"].is_array());
        assert_eq!(body["tool_choice"], "auto");
    }

    #[test]
    fn unsupported_tools_omits_tools_and_tool_choice() {
        static META: ModelMeta = ModelMeta::new("custom").with_tools(false);
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": { "name": "bash" }
        })];
        let p = OpenAiProvider::new("sk".to_string(), "custom")
            .with_model_meta(&META)
            .with_tools(tools);
        let body = build_request_body(&p, &[ChatMessage::user("hi".to_string())]);
        assert!(body["tools"].is_null());
        assert!(body["tool_choice"].is_null());
    }
}
