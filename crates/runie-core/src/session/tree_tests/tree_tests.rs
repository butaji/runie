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

// ─── Serialization Tests ────────────────────────────────────────────────────

#[test]
fn to_snapshot_and_back_preserves_tree() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "there", "m3"),
    ];
    let tree = SessionTree::from_messages(&messages);

    // Serialize
    let snapshot = tree.to_snapshot().unwrap();

    // Deserialize
    let restored = SessionTree::from_snapshot(&snapshot).expect("should restore");

    // Verify structure is preserved
    assert_eq!(restored.node_count(), tree.node_count());
    assert_eq!(restored.current_branch().len(), tree.current_branch().len());

    // Verify message IDs are preserved
    assert!(restored.id_index().contains_key("m1"));
    assert!(restored.id_index().contains_key("m2"));
    assert!(restored.id_index().contains_key("m3"));
}

#[test]
fn clone_preserves_tree_structure() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "there", "m3"),
    ];
    let tree = SessionTree::from_messages(&messages);

    // Clone
    let cloned = tree.clone();

    // Verify structure is preserved
    assert_eq!(cloned.node_count(), tree.node_count());
    assert_eq!(cloned.current_branch().len(), tree.current_branch().len());

    // Verify message IDs are preserved
    assert!(cloned.id_index().contains_key("m1"));
    assert!(cloned.id_index().contains_key("m2"));
    assert!(cloned.id_index().contains_key("m3"));
}

#[test]
fn to_snapshot_preserves_fork() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "there", "m3"),
    ];
    let mut tree = SessionTree::from_messages(&messages);
    let _path = tree.fork_at(1).expect("fork should succeed");

    // Serialize
    let snapshot = tree.to_snapshot().unwrap();

    // Verify snapshot has the fork
    assert!(snapshot.nodes.len() >= 4); // 3 original + 1 fork

    // Deserialize
    let restored = SessionTree::from_snapshot(&snapshot).expect("should restore");

    // Verify fork is preserved
    assert_eq!(restored.node_count(), tree.node_count());
}

#[test]
fn to_snapshot_preserves_current_branch() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
        msg(Role::User, "there", "m3"),
    ];
    let mut tree = SessionTree::from_messages(&messages);

    // Navigate to a different position
    let root = tree.root_id().unwrap();
    let first_child = tree.first_child(root).unwrap();
    tree.navigate_to(&[root, first_child]);

    // Serialize
    let snapshot = tree.to_snapshot().unwrap();

    // Verify snapshot has the branch
    assert_eq!(snapshot.current_branch.len(), 2); // root + first_child

    // Deserialize
    let restored = SessionTree::from_snapshot(&snapshot).expect("should restore");

    // Verify branch is preserved
    assert_eq!(restored.current_branch().len(), tree.current_branch().len());
}

#[test]
fn serde_roundtrip_preserves_tree() {
    let messages = vec![
        msg(Role::User, "hello", "m1"),
        msg(Role::Assistant, "hi", "m2"),
    ];
    let tree = SessionTree::from_messages(&messages);

    // Serialize to JSON
    let json = serde_json::to_string(&tree).expect("should serialize");

    // Deserialize from JSON
    let restored: SessionTree =
        serde_json::from_str(&json).expect("should deserialize");

    // Verify structure is preserved
    assert_eq!(restored.node_count(), tree.node_count());
    assert_eq!(restored.current_branch().len(), tree.current_branch().len());

    // Verify message IDs are preserved
    assert!(restored.id_index().contains_key("m1"));
    assert!(restored.id_index().contains_key("m2"));
}
