use crate::commands::CommandResult;
use crate::model::{AppState, ChatMessage, Role};

use super::{exec_handler, run_slash};

fn four_messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: Role::User,
            content: "hi".into(),
            timestamp: 0.0,
            id: "u1".into(),
            ..Default::default()
        },
        ChatMessage {
            role: Role::Assistant,
            content: "hello".into(),
            timestamp: 0.0,
            id: "a1".into(),
            ..Default::default()
        },
        ChatMessage {
            role: Role::Tool,
            content: "tool out".into(),
            timestamp: 0.0,
            id: "t1".into(),
            ..Default::default()
        },
        ChatMessage {
            role: Role::User,
            content: "again".into(),
            timestamp: 0.0,
            id: "u2".into(),
            ..Default::default()
        },
    ]
}

#[test]
fn session_info_counts_messages() {
    let mut state = AppState::default();
    state.session.messages = four_messages();
    let result = exec_handler(&mut state, "session", "");
    if let CommandResult::Message(msg) = result {
        assert!(
            msg.contains("Messages: 4 (2 user, 1 assistant, 1 tool)"),
            "got: {}",
            msg
        );
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn session_info_shows_tokens() {
    let mut state = AppState::default();
    state.session.messages = vec![ChatMessage {
        role: Role::User,
        content: "hello world".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    }];
    let result = exec_handler(&mut state, "session", "");
    if let CommandResult::Message(msg) = result {
        assert!(
            msg.contains("Tokens:"),
            "Token estimate should be present, got: {}",
            msg
        );
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn slash_session_dispatches() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "test".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    run_slash(&mut state, "/session");
    let last = state.session.messages.last().unwrap();
    assert_eq!(last.role, Role::System);
    assert!(last.content.contains("Messages:"));
}
