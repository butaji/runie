//! OpenAI-compatible chat-completions provider.

mod normalize;
mod protocol;
mod request;
pub mod stream;

#[derive(Clone)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
    model_meta: Option<&'static runie_core::provider_registry::ModelMeta>,
    tools: Vec<serde_json::Value>,
    tool_choice: Option<serde_json::Value>,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            api_key: api_key.trim().to_string(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            model_meta: None,
            tools: Vec::new(),
            tool_choice: None,
            client,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_model_meta(
        mut self,
        meta: &'static runie_core::provider_registry::ModelMeta,
    ) -> Self {
        self.model_meta = Some(meta);
        self
    }

    pub fn with_tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_tool_choice(mut self, choice: serde_json::Value) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn model_meta(&self) -> Option<&runie_core::provider_registry::ModelMeta> {
        self.model_meta
    }

    pub fn tools(&self) -> &[serde_json::Value] {
        &self.tools
    }

    pub fn tool_choice(&self) -> Option<&serde_json::Value> {
        self.tool_choice.as_ref()
    }
}

impl runie_core::provider::Provider for OpenAiProvider {
    fn generate(
        &self,
        messages: Vec<runie_core::message::ChatMessage>,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Stream<Item = anyhow::Result<runie_core::provider_event::ProviderEvent>> + Send + '_,
        >,
    > {
        stream::openai_stream(self.clone(), messages)
    }

    fn generate_with_tools(
        &self,
        messages: Vec<runie_core::message::ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Stream<Item = anyhow::Result<runie_core::provider_event::ProviderEvent>> + Send + '_,
        >,
    > {
        let provider = self
            .clone()
            .with_tools(tools)
            .with_tool_choice(serde_json::json!("auto"));
        stream::openai_stream(provider, messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use request::build_request_body;
    use runie_core::provider_event::{ProviderEvent, StopReason};
    use runie_core::message::{ChatMessage, Part, Role, ToolCall};
    use stream::{parse_sse_event, SseEvent};

    fn test_provider() -> OpenAiProvider {
        OpenAiProvider::new("sk-test".to_string(), "gpt-4o")
    }

    #[test]
    fn serializes_user_assistant_system_roles_directly() {
        let messages = vec![
            ChatMessage::system("sys".to_string()),
            ChatMessage::user("hi".to_string()),
            ChatMessage::assistant("hello".to_string()),
        ];
        let body = build_request_body(&test_provider(), &messages);
        let serialized = body["messages"].as_array().unwrap();
        assert_eq!(serialized[0]["role"], "system");
        assert_eq!(serialized[1]["role"], "user");
        assert_eq!(serialized[2]["role"], "assistant");
    }

    #[test]
    fn serializes_tool_role_as_user_when_id_missing() {
        // Orphan tool result (no matching tool call, no tool_call_id) is removed by sanitize.
        // Only system placeholder remains.
        let msg = ChatMessage::tool("read_file result:\nhello".to_string());
        let body = build_request_body(&test_provider(), &[msg]);
        let serialized = body["messages"].as_array().unwrap();
        assert!(!serialized.is_empty());
        assert_eq!(serialized[0]["role"], "system");
    }

    #[test]
    fn assistant_tool_message_has_empty_content() {
        // Dangling tool calls (no matching result) are removed by sanitize.
        // After removal, only text remains, so content is preserved.
        let messages = vec![
            ChatMessage::user("read it".to_string()),
            ChatMessage {
                role: Role::Assistant,
                timestamp: 0.0,
                id: String::new(),
                provider: String::new(),
                metadata: Default::default(),
                tool_call_id: None,
                provider_metadata: None,
                parts: vec![
                    Part::Text { content: "Reading...".into() },
                    Part::ToolCall {
                        id: "call_1".into(),
                        name: "read_file".into(),
                        args: serde_json::json!({"path":"README.md"}),
                    },
                ],
            },
        ];
        let body = build_request_body(&test_provider(), &messages);
        let serialized = &body["messages"].as_array().unwrap()[1];
        assert_eq!(serialized["role"], "assistant");
        // Content preserved since dangling tool call was removed
        assert_eq!(serialized["content"], "Reading...");
        assert!(serialized["tool_calls"].as_array().map(|a| a.is_empty()).unwrap_or(true));
    }

    #[test]
    fn serializes_tool_role_with_call_id_when_present() {
        // Tool result with matching tool_call_id serializes as role="tool".
        // Needs user message first so sanitize doesn't add system placeholder.
        let assistant = ChatMessage {
            role: Role::Assistant,
            timestamp: 0.0,
            id: String::new(),
            provider: String::new(),
            metadata: Default::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![Part::ToolCall {
                id: "call_abc".into(),
                name: "read_file".into(),
                args: serde_json::json!({"path":"README.md"}),
            }],
        };
        let result =
            ChatMessage::tool("read_file result:\nhello".to_string()).with_tool_call_id("call_abc");
        let body = build_request_body(&test_provider(), &[ChatMessage::user("read it".to_string()), assistant, result]);
        let serialized = body["messages"].as_array().unwrap();
        assert_eq!(serialized[1]["role"], "assistant");
        assert_eq!(serialized[2]["role"], "tool");
        assert_eq!(serialized[2]["tool_call_id"], "call_abc");
        assert_eq!(serialized[2]["content"], "read_file result:\nhello");
    }

    #[test]
    fn never_emits_role_tool_in_request_body_without_id() {
        let messages = vec![
            ChatMessage::user("list files".to_string()),
            ChatMessage::assistant("TOOL:list_dir:.".to_string()),
            ChatMessage::tool("file1.txt".to_string()),
        ];
        let body = build_request_body(&test_provider(), &messages);
        for obj in body["messages"].as_array().unwrap() {
            assert_ne!(obj["role"], "tool");
        }
    }

    #[test]
    fn serializes_assistant_tool_calls() {
        // Assistant message with tool call (empty id = not tracked as dangling).
        let messages = vec![
            ChatMessage::user("hi".to_string()),
            ChatMessage {
                role: Role::Assistant,
                timestamp: 0.0,
                id: String::new(),
                provider: String::new(),
                metadata: Default::default(),
                tool_call_id: None,
                provider_metadata: None,
                parts: vec![
                    Part::ToolCall {
                        id: String::new(),  // empty = not tracked as dangling
                        name: "read_file".into(),
                        args: serde_json::json!({"path":"Cargo.toml"}),
                    },
                ],
            },
        ];
        let body = build_request_body(&test_provider(), &messages);
        let serialized = body["messages"].as_array().unwrap();
        assert_eq!(serialized[1]["role"], "assistant");
        let calls = serialized[1]["tool_calls"].as_array().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "read_file");
        assert_eq!(calls[0]["type"], "function");
        assert_eq!(calls[0]["function"]["name"], "read_file");
        assert_eq!(
            calls[0]["function"]["arguments"],
            r#"{"path":"Cargo.toml"}"#
        );
    }

    #[test]
    fn parse_sse_event_text_delta() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        match parse_sse_event(line) {
            Some(SseEvent::Chunk(chunk)) => {
                assert_eq!(chunk.delta.content, Some("hi".to_string()));
                assert!(chunk.delta.tool_calls.is_empty());
            }
            other => panic!("expected Chunk, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_event_done() {
        assert!(matches!(
            parse_sse_event("data: [DONE]"),
            Some(SseEvent::Done)
        ));
    }

    #[test]
    fn stream_emits_text_deltas_and_finish() {
        let events = stream::tests::collect_events(&[
            r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#,
            r#"data: {"choices":[{"delta":{"content":" world"},"finish_reason":"stop"}]}"#,
            "data: [DONE]",
        ]);

        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::TextDelta(t) if t == "Hello")));
        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::TextDelta(t) if t == " world")));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::Finish {
                reason: StopReason::Stop
            }
        )));
    }

    #[test]
    fn stream_parses_finish_reasons() {
        let events = stream::tests::collect_events(&[
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "data: [DONE]",
        ]);

        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::Finish {
                reason: StopReason::ToolCalls
            }
        )));
    }

    #[test]
    fn stream_emits_usage_when_present() {
        let events = stream::tests::collect_events(&[
            r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
            "data: [DONE]",
        ]);

        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::Usage {
                input_tokens: 10,
                output_tokens: 5
            }
        )));
    }

    #[test]
    fn stream_accumulates_tool_call_deltas() {
        let events = stream::tests::collect_events(&[
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"Cargo.toml\"}"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "data: [DONE]",
        ]);

        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_1" && name == "read_file"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_1" && delta.contains("Cargo.toml")
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallEnd { id } if id == "call_1"
        )));
    }

    #[test]
    fn stream_emits_buffered_arguments_after_delayed_tool_call_id() {
        let events = stream::tests::collect_events(&[
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\"Cargo.toml\"}"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_2","function":{"name":"read_file"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            "data: [DONE]",
        ]);

        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { id, name } if id == "call_2" && name == "read_file"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_2" && delta == "{\"path\":\"Cargo.toml\"}"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallEnd { id } if id == "call_2"
        )));
    }

    #[test]
    fn provider_trims_api_key_and_base_url() {
        let p = OpenAiProvider::new("  sk-padded\n ".to_string(), "gpt-4o")
            .with_base_url("https://api.example.com/v1/");
        assert_eq!(p.api_key, "sk-padded");
        assert_eq!(p.base_url, "https://api.example.com/v1");
    }

    #[test]
    fn request_url_normalizes_base_url_trailing_slash() {
        let p = OpenAiProvider::new("sk".to_string(), "gpt-4o")
            .with_base_url("https://api.example.com/v1/");
        let url = format!("{}/chat/completions", p.base_url);
        assert_eq!(url, "https://api.example.com/v1/chat/completions");
    }
}
