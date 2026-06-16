use crate::model::{AppState, ChatMessage, Role};
use crate::tokens::estimate_tokens;
use crate::message::MessageMetadata;

fn msg(role: Role, content: impl Into<String>, timestamp: f64, id: impl Into<String>) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        timestamp,
        id: id.into(),
        ..Default::default()
    }
}

fn pinned_msg(role: Role, content: impl Into<String>, timestamp: f64, id: impl Into<String>) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        timestamp,
        id: id.into(),
        metadata: MessageMetadata { pinned: true, ..Default::default() },
        ..Default::default()
    }
}

#[test]
fn token_estimation_consistent() {
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("abcd"), 1);
    assert_eq!(estimate_tokens("abcdefgh"), 2);
}

#[test]
fn empty_session_no_compaction_needed() {
    let state = AppState::default();
    assert_eq!(state.total_tokens(), 0);
}

#[test]
fn session_tokens_from_messages() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "Hello world test".to_string(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "Response here".to_string(),
        timestamp: 0.0,
        id: "a1".into(),
        ..Default::default()
    });
    let tokens = state.total_tokens();
    assert!(tokens > 0);
    assert_eq!(
        tokens,
        estimate_tokens("Hello world test") + estimate_tokens("Response here")
    );
}

#[test]
fn compaction_creates_summary_entry() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Question {}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("Answer {} with lots of text to make tokens", i),
            timestamp: i as f64 + 0.5,
            id: format!("a{}", i),
            ..Default::default()
        });
    }
    let before = state.session.messages.len();
    let summary = state.compact(50);
    assert!(summary.contains("compact") || summary.contains("Compact"));
    let after = state.session.messages.len();
    assert!(after < before, "Expected fewer messages after compaction, got {} vs {}", after, before);
}

#[test]
fn compaction_keeps_recent_messages() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Q{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.compact(20);
    let last = state.session.messages.last().unwrap();
    assert_eq!(last.content, "Q9");
    assert_eq!(last.role, Role::User);
}

#[test]
fn compaction_does_not_cut_mid_turn() {
    let mut state = AppState::default();
    state.session.messages.extend([
        msg(Role::User, "Start", 0.0, "u1"),
        msg(Role::Assistant, "A", 1.0, "a1"),
        msg(Role::Tool, "tool result", 2.0, "t1"),
        msg(Role::User, "Recent", 3.0, "u2"),
    ]);
    state.compact(10);
    let has_tool = state.session.messages.iter().any(|m| m.role == Role::Tool);
    let has_assistant = state.session.messages.iter().any(|m| m.role == Role::Assistant && m.id == "a1");
    assert!(!has_tool || has_assistant, "Should not leave orphaned tool result");
}

#[test]
fn pinned_messages_not_compacted() {
    let mut state = AppState::default();
    // Add pinned messages
    state.session.messages.push(pinned_msg(Role::User, "Important pinned question", 0.0, "p1"));
    state.session.messages.push(pinned_msg(Role::Assistant, "Important pinned answer", 1.0, "p2"));
    // Add regular messages
    for i in 0..10 {
        state.session.messages.push(msg(Role::User, format!("Q{}", i), i as f64 + 2.0, format!("u{}", i)));
    }
    state.compact(20);
    // Pinned messages should still be present
    let pinned: Vec<_> = state.session.messages.iter()
        .filter(|m| m.metadata.pinned)
        .collect();
    assert_eq!(pinned.len(), 2, "Pinned messages should not be removed");
}

#[test]
fn compacted_message_has_compacted_flag() {
    let mut state = AppState::default();
    // Add enough content to trigger compaction with a reasonable threshold
    for i in 0..10 {
        state.session.messages.push(msg(Role::User, format!("Question {} with some extra text to increase tokens", i), i as f64, format!("u{}", i)));
    }
    state.compact(100); // Keep ~100 tokens worth
    let first = state.session.messages.first().unwrap();
    // After compaction, first message should be the summary with compacted flag
    assert!(first.metadata.compacted, "Summary should have compacted flag set. First: {:?}", first);
    assert!(first.content.contains("Compacted"), "Summary should mention compaction");
}
