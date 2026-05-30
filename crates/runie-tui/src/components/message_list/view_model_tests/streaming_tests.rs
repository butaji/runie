//! Tests for streaming behavior in MessageList

use super::helper_helpers::*;
use crate::components::message_list::feed::Feed;

#[test]
fn test_assistant_streaming_text_append() {
    // Test that append_to_last correctly appends text for streaming
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.append_to_last("Hel");
    feed.append_to_last("lo");
    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "Hello", "append_to_last should accumulate text");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_assistant_streaming_full_replacement() {
    // Test that update_last_assistant_text correctly replaces text
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.update_last_assistant_text("Partial");
    feed.update_last_assistant_text("Complete response");
    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "Complete response", "update_last_assistant_text should replace text");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_assistant_streaming_empty_then_content() {
    // Test rendering when assistant starts empty then gets content
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message(); // starts empty
    feed.append_to_last("Hi there!"); // streaming appends content

    let item = &feed.items[1];
    let (row_text, _, _) = render_feed_item(item, true);
    assert!(row_text.contains("Hi there!"), "Expected streaming text 'Hi there!' in row, got: '{}'", row_text.trim());
}

#[test]
fn test_assistant_streaming_multiple_appends() {
    // Test multiple streaming appends accumulate correctly
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.append_to_last("Hello ");
    feed.append_to_last("world!");
    feed.append_to_last(" How are you?");

    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "Hello world! How are you?", "Multiple appends should accumulate");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_feed_append_to_last_only_affects_assistant() {
    // Verify append_to_last doesn't affect UserMessage
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.append_to_last(" World"); // should NOT modify user message

    match &feed.items[0] {
        FeedItem::UserMessage { text, .. } => {
            assert_eq!(text, "Hello", "append_to_last should not modify UserMessage");
        }
        _ => panic!("Expected UserMessage"),
    }
}

#[test]
fn test_assistant_thinking_overwrites_then_content() {
    // Simulate: empty assistant -> thinking content -> actual content
    // This is the likely bug scenario
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message(); // starts empty
    feed.update_last_assistant_text("[thinking: ...]"); // thinking arrives first
    feed.update_last_assistant_text("Actual response"); // actual content arrives

    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "Actual response", "Final text should be actual content, not thinking");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_assistant_thinking_ignored_when_actual_content_arrives() {
    // When actual content arrives after thinking, final text should be actual
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();

    // Simulate streaming: first thinking, then actual
    feed.append_to_last("[thinking: ...]");
    feed.append_to_last("Actual response");

    match &feed.items[1] {
        FeedItem::AssistantMessage { text, .. } => {
            assert_eq!(text, "[thinking: ...]Actual response", "Append should accumulate");
        }
        _ => panic!("Expected AssistantMessage"),
    }
}

#[test]
fn test_render_assistant_after_thinking_update() {
    // Test rendering after thinking text was updated to actual content
    let mut feed = Feed::new();
    feed.add_user_message("Hello".to_string());
    feed.add_assistant_message();
    feed.update_last_assistant_text("Final response");

    let item = &feed.items[1];
    let (row_text, _, _) = render_feed_item(item, false);
    assert!(row_text.contains("Final response"), "Should show actual response, got: '{}'", row_text.trim());
    assert!(!row_text.contains("·"), "Should NOT show placeholder dot, got: '{}'", row_text.trim());
}
