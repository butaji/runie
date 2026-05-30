//! Tests for MessageList update behavior

use crate::components::message_list::types::{MessageItem, MessageList};

#[test]
fn test_update_last_assistant() {
    let mut list = MessageList::default();
    list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    list.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
    list.update_last_assistant("Hi there");
    assert_eq!(list.messages.last(), Some(&MessageItem::Assistant { text: "Hi there".to_string(), model: Some("gpt-4".to_string()), timestamp: None }));
}

#[test]
fn test_add_or_update_assistant_updates_existing() {
    let mut list = MessageList::default();
    list.messages.push(MessageItem::Assistant { text: "Partial".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
    list.add_or_update_assistant("Complete response", Some("gpt-4".to_string()));
    assert_eq!(list.messages.len(), 1);
    assert_eq!(list.messages[0], MessageItem::Assistant { text: "Complete response".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
}

#[test]
fn test_add_or_update_assistant_adds_new() {
    let mut list = MessageList::default();
    list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    list.add_or_update_assistant("Response", Some("gpt-4".to_string()));
    assert_eq!(list.messages.len(), 2);
    assert_eq!(list.messages[1], MessageItem::Assistant { text: "Response".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
}

#[test]
fn test_has_assistant_in_progress() {
    let mut list = MessageList::default();
    list.messages.push(MessageItem::Assistant { text: "Thinking...".to_string(), model: None, timestamp: None });
    assert!(list.has_assistant_in_progress());
    list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    assert!(!list.has_assistant_in_progress());
}

#[test]
fn test_update_last_assistant_no_op_when_no_assistant() {
    let mut list = MessageList::default();
    list.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    list.update_last_assistant("This should not change anything");
    assert_eq!(list.messages[0], MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
}
