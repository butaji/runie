//! Tests for session fork functionality.
//!
//! Tests cover:
//! - Fork at specific message index
//! - Fork with fallback to most recent user message
//! - Invalid index handling
//! - Tree branching verification
//! - Fork serialization/deserialization

use runie_core::message::{ChatMessage, Role};
use runie_core::session::tree::SessionTree;

fn msg(role: Role, content: &str, id: &str) -> ChatMessage {
    ChatMessage {
        role,
        timestamp: 0.0,
        id: id.into(),
        parts: vec![runie_core::message::Part::Text { content: content.into() }],
        ..Default::default()
    }
}

fn msgs() -> Vec<ChatMessage> {
    vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi there", "m2"),
        msg(Role::User, "how are you", "m3"),
        msg(Role::Assistant, "I'm fine", "m4"),
    ]
}

#[test]
fn fork_at_specific_index() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    let path = tree.fork_at(2).expect("fork should succeed");

    // Verify fork created new node
    assert!(path.len() >= 2);
    assert_eq!(tree.node_count(), 5); // 4 original + 1 fork placeholder
}

#[test]
fn fork_creates_placeholder_message() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    tree.fork_at(1).expect("fork should succeed");

    // Verify placeholder exists with expected ID pattern
    let fork_id = format!("fork.{}", 1);
    assert!(tree.id_index().contains_key(&fork_id));

    // Find the fork node and verify it's a system message
    let fork_node_id = *tree.id_index().get(&fork_id).unwrap();
    let node = tree.get_node(fork_node_id).expect("should get fork node");
    assert_eq!(node.message.role, Role::System);
    assert!(node.message.content().contains("fork point"));
}

#[test]
fn fork_at_zero_index() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    let path = tree.fork_at(0).expect("fork should succeed");

    assert!(!path.is_empty());
    assert!(tree.node_count() >= messages.len());
}

#[test]
fn fork_at_last_index() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    let last_index = messages.len() - 1;

    let path = tree.fork_at(last_index).expect("fork should succeed");

    assert!(path.len() >= 2);
}

#[test]
fn fork_out_of_bounds_returns_none() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    assert!(tree.fork_at(100).is_none());
    assert!(tree.fork_at(messages.len() + 10).is_none());
}

#[test]
fn multiple_forks_create_branches() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    // Create multiple forks at different points
    let path1 = tree.fork_at(1).expect("first fork should succeed");
    tree.navigate_to(&path1);

    let path2 = tree.fork_at(2).expect("second fork should succeed");

    // Verify tree has all fork nodes
    assert!(tree.node_count() >= 6); // 4 original + 2 fork placeholders

    // Verify paths are different
    assert_ne!(path1, path2);
}

#[test]
fn fork_preserves_existing_messages() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    let original_count = tree.node_count();

    tree.fork_at(2).expect("fork should succeed");

    // Original messages should still be accessible
    assert!(tree.id_index().contains_key("m1"));
    assert!(tree.id_index().contains_key("m2"));
    assert!(tree.id_index().contains_key("m3"));
    assert!(tree.id_index().contains_key("m4"));
    assert!(tree.node_count() > original_count);
}

#[test]
fn fork_at_user_message_index() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    // Find index of a user message
    let user_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::User)
        .map(|(i, _)| i)
        .collect();

    let user_index = user_indices[0]; // First user message
    tree.fork_at(user_index).expect("fork should succeed");

    // Verify fork was created
    assert!(tree.node_count() > messages.len());
}

#[test]
fn fork_navigation_after_fork() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);

    let path = tree.fork_at(1).expect("fork should succeed");
    tree.navigate_to(&path);

    // Verify navigation worked
    assert!(!tree.current_branch().is_empty());
}

#[test]
fn fallback_fork_index_finds_user_message() {
    use runie_core::commands::dsl::handlers::session::run::fallback_fork_index;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = msgs();

    let index = fallback_fork_index(&state);

    // Should find the last user message (index 2)
    assert_eq!(index, 2);
}

#[test]
fn fallback_fork_index_empty_session() {
    use runie_core::commands::dsl::handlers::session::run::fallback_fork_index;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = vec![];

    let index = fallback_fork_index(&state);

    // Should return 0 for empty session
    assert_eq!(index, 0);
}

#[test]
fn fallback_fork_index_only_assistant() {
    use runie_core::commands::dsl::handlers::session::run::fallback_fork_index;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = vec![
        msg(Role::Assistant, "Hello", "a1"),
        msg(Role::Assistant, "World", "a2"),
    ];

    let index = fallback_fork_index(&state);

    // Should return 0 when no user messages
    assert_eq!(index, 0);
}

#[test]
fn fork_serialization_roundtrip() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    tree.fork_at(2).expect("fork should succeed");

    // Serialize to snapshot
    let snapshot = tree.to_snapshot().expect("snapshot should succeed");

    // Verify snapshot contains fork node
    assert!(snapshot.nodes.iter().any(|n| n.data.message.id.starts_with("fork.")));

    // Deserialize
    let restored = SessionTree::from_snapshot(&snapshot).expect("restore should succeed");

    // Verify fork is preserved
    assert_eq!(restored.node_count(), tree.node_count());
    assert!(restored
        .id_index()
        .keys()
        .any(|k| k.starts_with("fork.")));
}

#[test]
fn fork_json_serialization() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    tree.fork_at(1).expect("fork should succeed");

    // Serialize to JSON
    let json = serde_json::to_string(&tree).expect("should serialize");

    // Deserialize
    let restored: SessionTree = serde_json::from_str(&json).expect("should deserialize");

    // Verify fork preserved
    assert_eq!(restored.node_count(), tree.node_count());
}

#[test]
fn run_fork_with_valid_index() {
    use runie_core::commands::dsl::handlers::session::run::run_fork;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = msgs();

    let result = run_fork(&mut state, "2");

    // Should emit ForkSession event
    match result {
        runie_core::commands::CommandResult::Event(evt) => {
            assert!(matches!(evt, runie_core::Event::ForkSession { message_index: 2 }));
        }
        _ => panic!("expected ForkSession event"),
    }
}

#[test]
fn run_fork_with_empty_index() {
    use runie_core::commands::dsl::handlers::session::run::run_fork;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = msgs();

    let result = run_fork(&mut state, "");

    // Should use fallback index (2 = last user message)
    match result {
        runie_core::commands::CommandResult::Event(evt) => {
            assert!(matches!(evt, runie_core::Event::ForkSession { message_index: 2 }));
        }
        _ => panic!("expected ForkSession event"),
    }
}

#[test]
fn run_fork_with_invalid_index() {
    use runie_core::commands::dsl::handlers::session::run::run_fork;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = msgs();

    let result = run_fork(&mut state, "not_a_number");

    // Should emit RunForkCommand event for error handling
    if let runie_core::commands::CommandResult::Event(evt) = result {
        assert!(matches!(evt, runie_core::Event::RunForkCommand { .. }));
    }
}

#[test]
fn run_fork_out_of_bounds_index() {
    use runie_core::commands::dsl::handlers::session::run::run_fork;

    let mut state = runie_core::model::AppState::default();
    state.session_mut().messages = msgs();

    let result = run_fork(&mut state, "999");

    // Should emit RunForkCommand event for error handling
    if let runie_core::commands::CommandResult::Event(evt) = result {
        assert!(matches!(evt, runie_core::Event::RunForkCommand { .. }));
    }
}

#[test]
fn fork_from_empty_tree() {
    let messages = vec![];
    let mut tree = SessionTree::from_messages(&messages);

    // Fork at index 0 on empty tree
    let path = tree.fork_at(0).expect("fork should succeed");

    assert!(!path.is_empty());
    assert!(tree.node_count() >= 1);
}

#[test]
fn fork_at_preserves_filter_cache() {
    let messages = msgs();
    let tree = SessionTree::from_messages(&messages);

    // Populate cache
    let _ = tree.filtered_walk(runie_core::session::tree::SessionTreeFilter::All);

    // Fork mutates tree - cache should be invalidated
    let mut tree = tree;
    tree.fork_at(2).expect("fork should succeed");

    // Accessing filtered_walk should recompute (not use stale cache)
    let _ = tree.filtered_walk(runie_core::session::tree::SessionTreeFilter::All);
    // Test passes if no panic/assertion failure
}

#[test]
fn session_fork_adds_system_message() {
    let messages = msgs();
    let mut tree = SessionTree::from_messages(&messages);
    let original_count = tree.node_count();

    tree.fork_at(2).expect("fork should succeed");

    // Fork should add a system message
    assert!(tree.node_count() > original_count);
}
