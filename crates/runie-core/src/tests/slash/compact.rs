//! /compact slash command tests.

use super::exec;
use crate::event::Event;
use crate::message::Part;
use crate::model::{ChatMessage, Role};
use crate::tests::fresh_state;

fn add_messages(state: &mut crate::model::AppState, count: usize) {
    for i in 0..count {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("Question {} with lots of text to make tokens", i),
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
}

#[test]
fn compact_50_reduces_message_count() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);
    let before = state.session.messages.len();

    exec(&mut state, "/compact 50");
    state.update(Event::submit());

    let after = state.session.messages.len();
    assert!(
        after < before,
        "Expected fewer messages after compact, got {} vs {}",
        after,
        before
    );
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        sys.iter()
            .any(|m| m.content().contains("compact") || m.content().contains("Compact")),
        "summary should mention compaction: {:?}",
        sys.last()
    );
}

#[test]
fn compact_with_focus_includes_focus_in_summary() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);

    exec(&mut state, "/compact 50 core");
    state.update(Event::submit());

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content().contains("focus: core"),
        "summary should include focus: {}",
        last.content()
    );
}
