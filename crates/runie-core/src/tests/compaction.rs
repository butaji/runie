use crate::model::{AppState, ChatMessage, Role};
use crate::tokens::estimate_tokens;

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
    state.messages.push(ChatMessage { role: Role::User, content: "Hello world test".to_string(), timestamp: 0.0, id: "u1".into() });
    state.messages.push(ChatMessage { role: Role::Assistant, content: "Response here".to_string(), timestamp: 0.0, id: "a1".into() });
    let tokens = state.total_tokens();
    assert!(tokens > 0);
    assert_eq!(tokens, estimate_tokens("Hello world test") + estimate_tokens("Response here"));
}

#[test]
fn compaction_creates_summary_entry() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Question {}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
        state.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("Answer {} with lots of text to make tokens", i),
            timestamp: i as f64 + 0.5,
            id: format!("a{}", i),
        });
    }
    let before = state.messages.len();
    let summary = state.compact(50);
    assert!(summary.contains("compact") || summary.contains("Compact"));
    let after = state.messages.len();
    assert!(after < before, "Expected fewer messages after compaction, got {} vs {}", after, before);
}

#[test]
fn compaction_keeps_recent_messages() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Q{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.compact(20);
    let last = state.messages.last().unwrap();
    assert_eq!(last.content, "Q9");
    assert_eq!(last.role, Role::User);
}

#[test]
fn compaction_does_not_cut_mid_turn() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage { role: Role::User, content: "Start".to_string(), timestamp: 0.0, id: "u1".into() });
    state.messages.push(ChatMessage { role: Role::Assistant, content: "A".to_string(), timestamp: 1.0, id: "a1".into() });
    state.messages.push(ChatMessage { role: Role::Tool, content: "tool result".to_string(), timestamp: 2.0, id: "t1".into() });
    state.messages.push(ChatMessage { role: Role::User, content: "Recent".to_string(), timestamp: 3.0, id: "u2".into() });
    state.compact(10);
    let has_tool = state.messages.iter().any(|m| m.role == Role::Tool);
    let has_assistant = state.messages.iter().any(|m| m.role == Role::Assistant && m.id == "a1");
    assert!(!has_tool || has_assistant, "Should not leave orphaned tool result");
}
