//! Session tree tests — fork, clone, filter
use crate::event::Event;
use crate::message::{ChatMessage, Role};
use crate::model::AppState;
use crate::session_tree::{SessionTree, SessionTreeFilter};

fn msg(role: Role, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        timestamp: 0.0,
        id: "test".into(),
        ..Default::default()
    }
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
    assert_eq!(cloned.root.message.content, "hello");
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
    assert_eq!(SessionTreeFilter::NoTools.cycle(), SessionTreeFilter::UserOnly);
    assert_eq!(SessionTreeFilter::UserOnly.cycle(), SessionTreeFilter::LabeledOnly);
    assert_eq!(SessionTreeFilter::LabeledOnly.cycle(), SessionTreeFilter::All);
}

// === Layer 2 — Event Handling ===

#[test]
fn slash_fork_emits_event() {
    let mut state = AppState::default();
    state.messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
    ];
    state.input.push_str("/fork 1");
    state.update(Event::Submit);

    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("Forked"), "fork should emit event: {}", last.content);
    assert!(state.session_tree.is_some(), "session tree should be initialized");
}

#[test]
fn slash_clone_emits_event() {
    let mut state = AppState::default();
    state.messages = vec![
        msg(Role::User, "hello"),
        msg(Role::Assistant, "hi"),
    ];
    state.input.push_str("/clone");
    state.update(Event::Submit);

    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("cloned"), "clone should emit event: {}", last.content);
    assert!(state.session_tree.is_some(), "session tree should be initialized");
}

#[test]
fn slash_tree_opens_dialog() {
    let mut state = AppState::default();
    state.input.push_str("/tree");
    state.update(Event::Submit);

    assert!(
        matches!(state.open_dialog, Some(crate::commands::DialogState::SessionTree { .. })),
        "/tree should open session tree dialog"
    );
}

#[test]
fn tree_navigates_up_down() {
    let mut state = AppState::default();
    state.session_tree = Some(SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
        msg(Role::User, "c"),
    ]));
    state.open_dialog = Some(crate::commands::DialogState::SessionTree {
        filter: SessionTreeFilter::All,
        selected: 1,
    });

    // Up should decrement selected
    state.update(Event::HistoryPrev);
    if let Some(crate::commands::DialogState::SessionTree { selected, .. }) = state.open_dialog {
        assert_eq!(selected, 0, "up moves to previous item");
    } else {
        panic!("dialog should stay open");
    }

    // Down should increment selected
    state.update(Event::HistoryNext);
    if let Some(crate::commands::DialogState::SessionTree { selected, .. }) = state.open_dialog {
        assert_eq!(selected, 1, "down moves to next item");
    } else {
        panic!("dialog should stay open");
    }
}

#[test]
fn tree_filter_cycle_event() {
    let mut state = AppState::default();
    state.session_tree = Some(SessionTree::from_messages(&[
        msg(Role::User, "a"),
        msg(Role::Assistant, "b"),
    ]));
    state.open_dialog = Some(crate::commands::DialogState::SessionTree {
        filter: SessionTreeFilter::All,
        selected: 0,
    });

    state.update(Event::SessionTreeFilterCycle);
    if let Some(crate::commands::DialogState::SessionTree { filter, .. }) = state.open_dialog {
        assert_eq!(filter, SessionTreeFilter::NoTools, "filter should cycle");
    } else {
        panic!("dialog should stay open");
    }
}
