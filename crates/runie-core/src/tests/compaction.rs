use crate::message::{MessageMetadata, Part};
use crate::model::{AppState, ChatMessage, Role};
use crate::tokens::estimate_tokens;

fn msg(
    role: Role,
    content: impl Into<String>,
    timestamp: f64,
    id: impl Into<String>,
) -> ChatMessage {
    ChatMessage {
        role,
        timestamp,
        id: id.into(),
        parts: vec![Part::Text {
            content: content.into(),
        }],
        ..Default::default()
    }
}

fn pinned_msg(
    role: Role,
    content: impl Into<String>,
    timestamp: f64,
    id: impl Into<String>,
) -> ChatMessage {
    ChatMessage {
        role,
        timestamp,
        id: id.into(),
        metadata: MessageMetadata {
            pinned: true,
            ..Default::default()
        },
        parts: vec![Part::Text {
            content: content.into(),
        }],
        ..Default::default()
    }
}

#[test]
fn token_estimation_consistent() {
    // Empty string
    assert_eq!(estimate_tokens(""), 0);
    // Note: tiktoken uses subword tokenization, so these are approximate
    // The key invariant is that estimate_tokens never panics and is consistent
    let count_abcd = estimate_tokens("abcd");
    assert!(count_abcd >= 1, "abcd should be at least 1 token");
    let count_abcdefgh = estimate_tokens("abcdefgh");
    assert!(
        count_abcdefgh >= count_abcd,
        "longer strings should have >= tokens"
    );
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
        timestamp: 0.0,
        id: "u1".into(),
        parts: vec![Part::Text {
            content: "Hello world test".to_string(),
        }],
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 0.0,
        id: "a1".into(),
        parts: vec![Part::Text {
            content: "Response here".to_string(),
        }],
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
            parts: vec![Part::Text {
                content: format!("Question {}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: format!("Answer {} with lots of text to make tokens", i),
            }],
            timestamp: i as f64 + 0.5,
            id: format!("a{}", i),
            ..Default::default()
        });
    }
    let before = state.session.messages.len();
    let summary = state.compact(50);
    assert!(summary.contains("compact") || summary.contains("Compact"));
    let after = state.session.messages.len();
    assert!(
        after < before,
        "Expected fewer messages after compaction, got {} vs {}",
        after,
        before
    );
}

#[test]
fn compaction_keeps_recent_messages() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("Q{}", i),
            }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.compact(20);
    let last = state.session.messages.last().unwrap();
    assert_eq!(last.content(), "Q9");
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
    let has_assistant = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::Assistant && m.id == "a1");
    assert!(
        !has_tool || has_assistant,
        "Should not leave orphaned tool result"
    );
}

#[test]
fn pinned_messages_not_compacted() {
    let mut state = AppState::default();
    // Add pinned messages
    state.session.messages.push(pinned_msg(
        Role::User,
        "Important pinned question",
        0.0,
        "p1",
    ));
    state.session.messages.push(pinned_msg(
        Role::Assistant,
        "Important pinned answer",
        1.0,
        "p2",
    ));
    // Add regular messages with more content to exceed threshold
    for i in 0..10 {
        state.session.messages.push(msg(
            Role::User,
            format!(
                "This is question number {} with some extra content to add tokens",
                i
            ),
            i as f64 + 2.0,
            format!("u{}", i),
        ));
    }
    // Use a very low threshold to ensure compaction happens
    let result = state.compact(10);
    eprintln!("Compaction result: {}", result);
    eprintln!(
        "Total messages after compaction: {}",
        state.session.messages.len()
    );
    // Pinned messages should still be present
    let pinned: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.metadata.pinned)
        .collect();
    eprintln!("Pinned count: {}", pinned.len());
    for m in &pinned {
        eprintln!("  Pinned: {}", m.content());
    }
    assert_eq!(pinned.len(), 2, "Pinned messages should not be removed");
}

#[test]
fn compacted_message_has_compacted_flag() {
    let mut state = AppState::default();
    // Add enough content to trigger compaction
    // Use shorter messages that are more predictable for tiktoken
    for i in 0..10 {
        state.session.messages.push(msg(
            Role::User,
            format!("This is question number {}", i),
            i as f64,
            format!("u{}", i),
        ));
    }
    // Use a very low threshold to ensure compaction happens
    let result = state.compact(5); // Keep only ~5 tokens
    assert!(
        result.contains("Compacted") || result.contains("removed"),
        "Should trigger compaction. Got: {}",
        result
    );
    let first = state.session.messages.first().unwrap();
    // After compaction, first message should be the summary with compacted flag
    assert!(
        first.metadata.compacted,
        "Summary should have compacted flag set. First: {:?}",
        first
    );
    assert!(
        first.content().contains("Compacted"),
        "Summary should mention compaction"
    );
}
