//! Session tree tests — fork, clone, filter

use crate::Event;
use crate::message::{ChatMessage, Part, Role};
use crate::model::AppState;
use crate::session::tree::{SessionTree, SessionTreeFilter};

fn msg(role: Role, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        timestamp: 0.0,
        id: "test".into(),
        parts: vec![Part::Text { content: content.into() }],
        ..Default::default()
    }
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

// === Layer 1 — State/Logic ===

#[test]
fn fork_creates_branch() {
    let messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
        msg(Role::User, "how are you"),
    ];
    let mut tree = SessionTree::from_messages(&messages);
    let path = tree.fork_at(1).expect("fork should succeed");
    assert_eq!(path.len(), 2);
    // The forked node should now have an extra child
    assert_eq!(tree.root.children[0].children.len(), 2);
}

#[test]
fn clone_duplicates_position() {
    let messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
        msg(Role::User, "how are you"),
    ];
    let tree = SessionTree::from_messages(&messages);
    let cloned = tree.clone();
    assert_eq!(cloned.root.message.content(), "hello");
    assert_eq!(cloned.current_branch, tree.current_branch);
}

#[test]
fn tree_filter_excludes_tools() {
    let mut tree = SessionTree::from_messages(&[
        msg(Role::User, "hello"),
        msg(Role::Tool, "output"),
        msg(Role::Assistant, "hi"),
    ]);
    // Add a labeled node
    tree.root.children[0].label = Some("important".into());

    let all = tree.filtered_walk(SessionTreeFilter::All);
    assert_eq!(all.len(), 3);

    let no_tools = tree.filtered_walk(SessionTreeFilter::NoTools);
    assert_eq!(no_tools.len(), 2);

    let user_only = tree.filtered_walk(SessionTreeFilter::UserOnly);
    assert_eq!(user_only.len(), 1);

    let labeled = tree.filtered_walk(SessionTreeFilter::LabeledOnly);
    assert_eq!(labeled.len(), 1);
}

#[test]
fn filter_cycle_rotates() {
    assert_eq!(SessionTreeFilter::All.cycle(), SessionTreeFilter::NoTools);
    assert_eq!(
        SessionTreeFilter::NoTools.cycle(),
        SessionTreeFilter::UserOnly
    );
    assert_eq!(
        SessionTreeFilter::UserOnly.cycle(),
        SessionTreeFilter::LabeledOnly
    );
    assert_eq!(
        SessionTreeFilter::LabeledOnly.cycle(),
        SessionTreeFilter::All
    );
}

// === Layer 2 — Event Handling ===

#[test]
fn slash_fork_emits_event() {
    let mut state = AppState::default();
    state.session.messages = vec![msg(Role::User, "hello"), msg(Role::Assistant, "hi")];
    state.input.input.push_str("/fork 1");
    state.update(Event::submit()); // Opens form with pre-filled index
    state.update(crate::Event::CommandFormSubmit); // Submits the form

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content().contains("Forked"),
        "fork should emit event: {}",
        last.content()
    );
    assert!(
        state.session.session_tree.is_some(),
        "session tree should be initialized"
    );
}

#[test]
fn slash_tree_opens_dialog() {
    let mut state = AppState::default();
    palette_select(&mut state, "tree");

    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::SessionTree(_))
        ),
        "/tree should open session tree dialog"
    );
}

#[test]
fn tree_navigates_up_down() {
    let mut state = AppState::default();
    state.session.session_tree = Some(SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
        msg(Role::User, "c"),
    ]));
    state.update(crate::Event::ToggleSessionTree);

    // Up should decrement selected
    state.update(crate::Event::HistoryPrev);
    let selected = match &state.open_dialog {
        Some(crate::commands::DialogState::SessionTree(stack)) => {
            stack.current().map(|p| p.selected)
        }
        _ => None,
    };
    assert_eq!(selected, Some(2), "up at first wraps to last");

    // Down should increment selected
    state.update(crate::Event::HistoryNext);
    let selected = match &state.open_dialog {
        Some(crate::commands::DialogState::SessionTree(stack)) => {
            stack.current().map(|p| p.selected)
        }
        _ => None,
    };
    assert_eq!(selected, Some(0), "down moves to next item");
}

#[test]
fn tree_filter_cycle_event() {
    // Filter cycling is now handled by the unified panel stack; this event is
    // still dispatched and does not close the dialog.
    let mut state = AppState::default();
    state.session.session_tree = Some(SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
    ]));
    state.update(crate::Event::ToggleSessionTree);

    state.update(crate::Event::SessionTreeFilterCycle);
    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::SessionTree(_))
        ),
        "dialog should stay open"
    );
}

#[test]
fn test_session_tree_index_lookup() {
    let tree = SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
        msg(Role::User, "c"),
    ]);
    // Root is at empty path
    assert!(tree.node_index.contains_key(&Vec::<usize>::new()));
    // First child is at [0]
    assert!(tree.node_index.contains_key(&vec![0]));
    // Second child of first child is at [0, 0]
    assert!(tree.node_index.contains_key(&vec![0, 0]));
    // Nonexistent path
    assert!(!tree.node_index.contains_key(&vec![99]));
}

#[test]
fn test_session_tree_index_invalidation() {
    let messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
        msg(Role::User, "how are you"),
    ];
    let mut tree = SessionTree::from_messages(&messages);
    let old_version = tree.index_version;
    tree.fork_at(1).expect("fork should succeed");
    assert_ne!(tree.index_version, old_version);
    assert!(tree.node_index.is_empty());
}

#[test]
fn test_session_tree_navigate_with_index() {
    let mut tree = SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
        msg(Role::User, "c"),
    ]);
    // Navigate to root
    tree.navigate_to(&[]);
    assert!(tree.current_branch.is_empty());

    // Navigate to first child
    tree.navigate_to(&[0]);
    assert_eq!(tree.current_branch, vec![0]);

    // Navigate to deep child
    tree.navigate_to(&[0, 0]);
    assert_eq!(tree.current_branch, vec![0, 0]);

    // Invalid navigation should be a no-op
    tree.navigate_to(&[99]);
    assert_eq!(tree.current_branch, vec![0, 0]);
}

#[test]
fn test_fork_session_uses_index() {
    let messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
        msg(Role::User, "how are you"),
    ];
    let mut tree = SessionTree::from_messages(&messages);
    let path = tree.fork_at(1).expect("fork should succeed");
    tree.navigate_to(&path);
    assert_eq!(tree.current_branch, path);
}

#[test]
fn test_clone_session_uses_index() {
    let messages = vec![msg(Role::User, "hello"), msg(Role::Assistant, "hi")];
    let tree = SessionTree::from_messages(&messages);
    let cloned = tree.clone();
    assert_eq!(cloned.node_index, tree.node_index);
    assert_eq!(cloned.index_version, tree.index_version);
}

#[test]
fn test_filtered_walk_cache_hit() {
    let tree = SessionTree::from_messages(&[
        msg(Role::User, "hello"),
        msg(Role::Tool, "output"),
        msg(Role::Assistant, "hi"),
    ]);
    // First call populates cache
    let result1 = tree.filtered_walk(SessionTreeFilter::NoTools);
    assert_eq!(result1.len(), 2);
    // Second call should hit cache and return same result
    let result2 = tree.filtered_walk(SessionTreeFilter::NoTools);
    assert_eq!(result2.len(), 2);
    // Different filter should miss cache
    let result3 = tree.filtered_walk(SessionTreeFilter::UserOnly);
    assert_eq!(result3.len(), 1);
}

#[test]
fn test_filtered_walk_cache_invalidated_on_change() {
    let mut tree = SessionTree::from_messages(&[
        msg(Role::User, "hello"),
        msg(Role::Tool, "output"),
        msg(Role::Assistant, "hi"),
    ]);
    // Populate cache
    let _ = tree.filtered_walk(SessionTreeFilter::All);
    // Modify tree
    tree.fork_at(1);
    // Cache should be invalidated; still compute correctly
    let result = tree.filtered_walk(SessionTreeFilter::All);
    assert_eq!(result.len(), 4); // original 3 + fork placeholder
}

#[test]
fn tree_select_branch_switches_conversation() {
    let mut state = AppState::default();
    state.session.session_tree = Some(SessionTree::from_messages(&[
        ChatMessage {
            role: Role::User,
            timestamp: 0.0,
            id: "root".into(),
            parts: vec![Part::Text { content: "root".into() }],
            ..Default::default()
        },
        ChatMessage {
            role: Role::Assistant,
            timestamp: 1.0,
            id: "branch-a".into(),
            parts: vec![Part::Text { content: "branch-a".into() }],
            ..Default::default()
        },
        ChatMessage {
            role: Role::Assistant,
            timestamp: 2.0,
            id: "branch-b".into(),
            parts: vec![Part::Text { content: "branch-b".into() }],
            ..Default::default()
        },
    ]));
    state.update(crate::Event::ToggleSessionTree);
    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::SessionTree(_))
        ),
        "tree dialog should be open"
    );

    state.update(crate::Event::SessionTreeSelect {
        id: "branch-b".into(),
    });

    assert!(state.open_dialog.is_none(), "dialog should close on select");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(
        sys.iter().any(|m| m.content().contains("Switched")),
        "should confirm branch switch: {:?}",
        sys.last()
    );
    assert_eq!(
        state
            .session
            .session_tree
            .as_ref()
            .map(|t| t.current_branch.clone()),
        Some(vec![0, 0]),
        "current branch should navigate to branch-b"
    );
}
