//! Feed tests.

use super::*;
use crate::components::message_list::MessageItem;

#[test]
fn test_add_user_message() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    assert_eq!(feed.items.len(), 1);
    assert!(matches!(feed.items[0], FeedItem::UserMessage { .. }));
}

#[test]
fn test_add_assistant_message() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    assert_eq!(feed.items.len(), 2);
    assert!(matches!(feed.items[1], FeedItem::AssistantMessage { .. }));
}

#[test]
fn test_thoughts_attached_to_assistant() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.add_thought(1.5);
    feed.add_thought(2.0);
    match &feed.items[1] {
        FeedItem::AssistantMessage { thoughts, .. } => {
            assert_eq!(thoughts.len(), 2);
            assert_eq!(thoughts[0].duration, 1.5);
            assert_eq!(thoughts[1].duration, 2.0);
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_tool_calls_attached_to_assistant() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.add_tool_call("bash".to_string(), "pwd".to_string());
    match &feed.items[1] {
        FeedItem::AssistantMessage { tool_calls, .. } => {
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(tool_calls[0].name, "bash");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_append_to_last() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.append_to_last("Hi");
    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "Hi");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_complete_turn() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.complete_turn(3.5);
    match &feed.items[1] {
        FeedItem::AssistantMessage { turn_duration, .. } => {
            assert_eq!(*turn_duration, Some(3.5));
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_has_assistant_in_progress() {
    let mut feed = Feed::new();
    assert!(!feed.has_assistant_in_progress());
    feed.add_user_message("Hello".to_string());
    assert!(!feed.has_assistant_in_progress());
    feed.add_assistant_message();
    assert!(feed.has_assistant_in_progress());
}

#[test]
fn test_clear() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.clear();
    assert!(feed.is_empty());
}

#[test]
fn test_dedup_by_id() {
    let mut feed = Feed::new();
    let id = Uuid::new_v4().to_string();
    feed.add_if_new(FeedItem::UserMessage {
        id: id.clone(),
        text: "First".to_string(),
        timestamp: None,
    });
    assert_eq!(feed.items.len(), 1);
    // Try to add same ID again - should be deduped
    feed.add_if_new(FeedItem::UserMessage {
        id: id.clone(),
        text: "Second".to_string(),
        timestamp: None,
    });
    assert_eq!(feed.items.len(), 1);
    // First item's ID is preserved
    assert_eq!(feed.items[0].id(), id);
}

#[test]
fn test_add_thought_no_op_without_assistant() {
    let mut feed = Feed::new();
    feed.add_thought(1.0); // Should not panic
    assert!(feed.is_empty());
}

#[test]
fn test_add_tool_call_no_op_without_assistant() {
    let mut feed = Feed::new();
    feed.add_tool_call("bash".to_string(), "pwd".to_string());
    assert!(feed.is_empty());
}

#[test]
fn test_remove_last_empty_assistant() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.remove_last_empty_assistant();
    assert_eq!(feed.items.len(), 1); // Only user message remains
}

#[test]
fn test_remove_last_empty_assistant_keeps_non_empty() {
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.append_to_last("Hi");
    feed.remove_last_empty_assistant();
    assert_eq!(feed.items.len(), 2); // Both remain
}

#[test]
fn test_from_message_items() {
    let messages = vec![
        MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None },
        MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None },
        MessageItem::System { text: "System notice".to_string() },
        // These should be filtered out (now inline in AssistantMessage)
        MessageItem::Thought { duration_secs: 1.0, text: String::new() },
        MessageItem::ToolCall { name: "bash".to_string(), args: "pwd".to_string(), result: None, is_error: false },
    ];
    let feed = Feed::from(messages);
    let items = feed.items();
    assert_eq!(items.len(), 3);
    assert!(matches!(&items[0], FeedItem::UserMessage { text, .. } if text == "Hello"));
    assert!(matches!(&items[1], FeedItem::AssistantMessage { text, .. } if text == "Hi"));
    assert!(matches!(&items[2], FeedItem::SystemNotice { text, .. } if text == "System notice"));
}

#[test]
fn test_assistant_message_with_inline_thoughts_and_tool_calls() {
    let messages = vec![
        MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None },
        MessageItem::Assistant { text: "I'll help you".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None },
    ];
    let mut feed = Feed::from(messages);
    // Simulate adding thoughts and tool calls inline (as done during streaming)
    feed.add_thought(1.5);
    feed.add_tool_call("bash".to_string(), "ls".to_string());

    let items = feed.items();
    match &items[1] {
        FeedItem::AssistantMessage { thoughts, tool_calls, .. } => {
            assert_eq!(thoughts.len(), 1);
            assert_eq!(thoughts[0].duration, 1.5);
            assert_eq!(tool_calls.len(), 1);
            assert_eq!(tool_calls[0].name, "bash");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}
