//! Tests for ReplyProvider

#[cfg(test)]
mod tests {
    use crate::providers::ReplyProvider;
    use crate::providers::reply::Scenario;
    use runie_core::Event;

    #[test]
    fn test_scenario_routing() {
        // Simple scenarios
        assert_eq!(Scenario::from_input("hello"), Scenario::Simple);
        assert_eq!(Scenario::from_input("Hello there!"), Scenario::Simple);
        assert_eq!(Scenario::from_input("hi"), Scenario::Simple);
        assert_eq!(Scenario::from_input("how are you"), Scenario::Simple);

        // Tool scenarios
        assert_eq!(Scenario::from_input("calculate 5 + 3"), Scenario::Tool);
        assert_eq!(Scenario::from_input("use a tool"), Scenario::Tool);
        assert_eq!(Scenario::from_input("run the calculator tool"), Scenario::Tool);

        // Stream scenarios
        assert_eq!(Scenario::from_input("stream the response"), Scenario::Stream);
        assert_eq!(Scenario::from_input("count to 10"), Scenario::Stream);

        // Stream tool scenarios
        assert_eq!(Scenario::from_input("bash ls"), Scenario::StreamTool);
        assert_eq!(Scenario::from_input("ls -la"), Scenario::StreamTool);
        assert_eq!(Scenario::from_input("list files"), Scenario::StreamTool);

        // Error scenarios
        assert_eq!(Scenario::from_input("error test"), Scenario::Error);
        assert_eq!(Scenario::from_input("fail this"), Scenario::Error);

        // Context scenarios
        assert_eq!(Scenario::from_input("context test"), Scenario::Context);
        assert_eq!(Scenario::from_input("memory test"), Scenario::Context);

        // Long reasoning scenarios
        assert_eq!(Scenario::from_input("long response"), Scenario::LongReasoning);
        assert_eq!(Scenario::from_input("peanut butter"), Scenario::LongReasoning);
        assert_eq!(Scenario::from_input("explain this"), Scenario::LongReasoning);
    }

    #[test]
    fn test_load_fixtures() {
        let provider = ReplyProvider::with_default_fixtures();
        assert!(provider.is_ok(), "Should load fixtures: {:?}", provider.err());
    }

    #[test]
    fn test_generate_simple_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_simple_events();
        assert!(!events.is_empty());
        // Should have MessageDelta
        assert!(events.iter().any(|e| matches!(e, Event::MessageDelta { .. })));
        // Should have ThinkingDelta
        assert!(events.iter().any(|e| matches!(e, Event::ThinkingDelta { .. })));
        // Should have Usage
        assert!(events.iter().any(|e| matches!(e, Event::Usage { .. })));
    }

    #[test]
    fn test_generate_tool_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_tool_events();
        assert!(!events.is_empty());
        // Should have ToolCallDelta
        assert!(events.iter().any(|e| matches!(e, Event::ToolCallDelta { .. })));
    }

    #[test]
    fn test_generate_stream_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_stream_events();
        assert!(!events.is_empty());
        // Should have multiple MessageDeltas
        let message_deltas: Vec<_> = events.iter().filter(|e| matches!(e, Event::MessageDelta { .. })).collect();
        assert!(message_deltas.len() >= 1);
    }

    #[test]
    fn test_generate_stream_tool_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_stream_tool_events();
        assert!(!events.is_empty());
        // Should have ToolCallDelta
        assert!(events.iter().any(|e| matches!(e, Event::ToolCallDelta { .. })));
    }

    #[test]
    fn test_generate_error_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_error_events();
        assert!(!events.is_empty());
        // Should have Error event
        assert!(events.iter().any(|e| matches!(e, Event::Error { .. })));
    }

    #[test]
    fn test_generate_context_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_context_events();
        assert!(!events.is_empty());
        // Should have MessageDelta
        assert!(events.iter().any(|e| matches!(e, Event::MessageDelta { .. })));
    }

    #[test]
    fn test_generate_long_reasoning_events() {
        let provider = ReplyProvider::with_default_fixtures().unwrap();
        let events = provider.generate_long_reasoning_events();
        assert!(!events.is_empty());
        // Should have many MessageDeltas (content chunks)
        let message_deltas: Vec<_> = events.iter().filter(|e| matches!(e, Event::MessageDelta { .. })).collect();
        assert!(message_deltas.len() >= 10, "Expected many chunks, got {}", message_deltas.len());
    }
}
