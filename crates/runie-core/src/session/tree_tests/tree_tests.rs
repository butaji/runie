//! Tests for SessionTree.

use crate::message::{ChatMessage, Role};
use crate::session::tree::{SessionTree, SessionTreeFilter, TreeNodeData};
use indextree::Arena;

fn msg(role: Role, content: &str, id: &str) -> ChatMessage {
    ChatMessage {
        role,
        timestamp: 0.0,
        id: id.into(),
        parts: vec![crate::message::Part::Text {
            content: content.into(),
        }],
        ..Default::default()
    }
}

#[test]
fn from_messages_creates_linear_tree() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "there", "m3"),
    ];
    let tree = SessionTree::from_messages(&messages);

    assert_eq!(tree.node_count(), 3);
    assert_eq!(tree.current_branch().len(), 2);
    assert!(tree.id_index().contains_key("m1"));
    assert!(tree.id_index().contains_key("m2"));
    assert!(tree.id_index().contains_key("m3"));
}

#[test]
fn fork_creates_branch() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "how are you", "m3"),
    ];
    let mut tree = SessionTree::from_messages(&messages);
    let path = tree.fork_at(1).expect("fork should succeed");

    assert!(path.len() >= 2);
    assert_eq!(tree.node_count(), 4); // 3 original + 1 fork placeholder
}

#[test]
fn navigate_to_with_valid_path() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
    ];
    let mut tree = SessionTree::from_messages(&messages);

    // Navigate to root
    let root = tree.root_id().unwrap();
    tree.navigate_to(&[root]);
    // After navigating to [root], current_branch is [root] (len 1)
    assert_eq!(tree.current_branch().len(), 1);

    // Navigate to first child
    let first_child = tree.first_child(root).unwrap();
    tree.navigate_to(&[root, first_child]);
    assert_eq!(tree.current_branch().len(), 2);

    // Invalid navigation should be a no-op
    // Use a large separate arena so indices don't collide with tree's arena
    let mut large_arena = Arena::new();
    // Create many nodes to push the index high
    for _ in 0..10000 {
        large_arena.new_node(TreeNodeData::default());
    }
    let phantom = large_arena.new_node(TreeNodeData::default());
    tree.navigate_to(&[phantom]);
    assert_eq!(tree.current_branch().len(), 2);
}

#[test]
fn filtered_walk_cache() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Tool, "output", "m2"),
        msg(Role::Assistant, "hi", "m3"),
    ];
    let tree = SessionTree::from_messages(&messages);

    // First call populates cache
    let all = tree.filtered_walk(SessionTreeFilter::All);
    assert_eq!(all.len(), 3);

    // Second call should return same result from cache
    let all2 = tree.filtered_walk(SessionTreeFilter::All);
    assert_eq!(all2.len(), 3);

    // Different filter should miss cache
    let no_tools = tree.filtered_walk(SessionTreeFilter::NoTools);
    assert_eq!(no_tools.len(), 2);
}

#[test]
fn find_path_by_id() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
    ];
    let tree = SessionTree::from_messages(&messages);

    let path = tree.find_path_by_id("m2").expect("should find m2");
    assert!(!path.is_empty());
    let last_id = *path.last().unwrap();
    assert_eq!(tree.get_node(last_id).unwrap().message.id, "m2");
}

#[test]
fn filter_excludes_tools() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Tool, "output", "m2"),
        msg(Role::Assistant, "hi", "m3"),
    ];
    let tree = SessionTree::from_messages(&messages);

    let all = tree.filtered_walk(SessionTreeFilter::All);
    let no_tools = tree.filtered_walk(SessionTreeFilter::NoTools);
    let user_only = tree.filtered_walk(SessionTreeFilter::UserOnly);

    assert_eq!(all.len(), 3);
    assert_eq!(no_tools.len(), 2);
    assert_eq!(user_only.len(), 1);
}
