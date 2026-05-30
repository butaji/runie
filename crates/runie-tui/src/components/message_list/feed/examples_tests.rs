//! Tests for Feed builder API and examples.

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::super::{Feed, FeedItem};
    use super::builder::FeedBuilder;

    #[test]
    fn test_simple_builder() {
        let feed = FeedBuilder::new()
            .user_message("Hello!")
            .assistant()
            .thinking_for(Duration::from_millis(200))
            .say("Hi there!")
            .turn_completed_in(Duration::from_secs(1))
            .done()
            .build();

        assert_eq!(feed.items.len(), 2);
        assert!(matches!(
            &feed.items[0],
            FeedItem::UserMessage { text, .. } if text == "Hello!"
        ));
        assert!(matches!(
            &feed.items[1],
            FeedItem::AssistantMessage { text, .. } if text == "Hi there!"
        ));
    }

    #[test]
    fn test_builder_with_thoughts() {
        let feed = FeedBuilder::new()
            .user_message("What's 2+2?")
            .assistant()
            .thinking_for(Duration::from_millis(500))
            .say("It's 4!")
            .done()
            .build();

        assert_eq!(feed.items.len(), 2);
        match &feed.items[1] {
            FeedItem::AssistantMessage {
                thoughts, text, ..
            } => {
                assert_eq!(thoughts.len(), 1);
                assert_eq!(thoughts[0].duration, 0.5);
                assert_eq!(text, "It's 4!");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_builder_with_tool_call() {
        let feed = FeedBuilder::new()
            .user_message("List files")
            .assistant()
            .thinking_for(Duration::from_millis(300))
            .tool_call("bash", serde_json::json!({"command": "ls -la"}))
            .say("Here they are!")
            .done()
            .build();

        match &feed.items[1] {
            FeedItem::AssistantMessage {
                tool_calls, text, ..
            } => {
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].name, "bash");
                assert_eq!(text, "Here they are!");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_multi_turn_builder() {
        let feed = FeedBuilder::new()
            .user_message("First question")
            .assistant()
            .say("First response")
            .done()
            .user_message("Second question")
            .assistant()
            .say("Second response")
            .done()
            .build();

        assert_eq!(feed.items.len(), 4);
        assert!(matches!(
            &feed.items[0],
            FeedItem::UserMessage { text, .. } if text == "First question"
        ));
        assert!(matches!(
            &feed.items[1],
            FeedItem::AssistantMessage { text, .. } if text == "First response"
        ));
        assert!(matches!(
            &feed.items[2],
            FeedItem::UserMessage { text, .. } if text == "Second question"
        ));
        assert!(matches!(
            &feed.items[3],
            FeedItem::AssistantMessage { text, .. } if text == "Second response"
        ));
    }

    #[test]
    fn test_code_block() {
        let feed = FeedBuilder::new()
            .user_message("Show me code")
            .assistant()
            .thinking_for(Duration::from_millis(100))
            .code_block("fn main() {}")
            .done()
            .build();

        match &feed.items[1] {
            FeedItem::AssistantMessage { text, .. } => {
                assert!(text.contains("```"));
                assert!(text.contains("fn main() {}"));
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_chainable_trait() {
        use super::builder::FeedChainable;

        let feed = FeedBuilder::new()
            .user_message("Hello")
            .build();

        assert_eq!(feed.items.len(), 1);
    }
}
