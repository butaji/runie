#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::providers::{MockProvider, OpenAiProvider, AnthropicProvider};
    use crate::Provider;
    use futures::StreamExt;
    use runie_core::{Message, ToolSchema, Event};

    #[test]
    fn test_mock_provider_name() {
        let provider = MockProvider::new();
        assert_eq!(provider.name(), "mock");
    }

    #[test]
    fn test_mock_provider_model() {
        let provider = MockProvider::new();
        assert_eq!(provider.model(), "mock-gpt-4");
    }

    #[tokio::test]
    async fn test_mock_provider_chat_simple() {
        let provider = MockProvider::new();
        let messages = vec![Message::User {
            content: "hello".to_string(),
            attachments: vec![],
        }];
        let result = provider.chat_simple(messages).await.unwrap();
        assert!(result.contains("Hello"));
    }

    #[tokio::test]
    async fn test_mock_provider_with_tools() {
        let provider = MockProvider::new();
        let messages = vec![Message::User {
            content: "edit this file".to_string(),
            attachments: vec![],
        }];
        let tools = vec![ToolSchema {
            name: "edit_file".to_string(),
            description: "Edit a file".to_string(),
            parameters: serde_json::json!({}),
        }];

        let stream = provider.chat(messages, tools).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Debug: print what events we got
        for e in &events {
            println!("Event: {:?}", std::mem::discriminant(e));
        }

        // Should have tool call events
        assert!(events.iter().any(|e| matches!(e, Event::ToolCallDelta { .. })),
            "Expected ToolCallDelta in events, got: {:?}", events);
    }

    #[test]
    fn test_openai_provider_capabilities() {
        let provider = OpenAiProvider::new("key".to_string(), "gpt-4o".to_string());
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_tokens(), 128_000);
    }

    #[test]
    fn test_anthropic_provider_capabilities() {
        let provider = AnthropicProvider::new("key".to_string(), "claude-3-5-sonnet".to_string());
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
        assert_eq!(provider.max_context_tokens(), 200_000);
    }
}
