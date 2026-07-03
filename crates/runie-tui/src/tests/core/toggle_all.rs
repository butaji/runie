use runie_core::model::{ChatMessage, Role};
use runie_core::Event;
use runie_core::Part;
use runie_testing::fresh_state;

#[test]
fn collapse_all_when_some_expanded() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text {
            content: "◆ Thought 1.0s\nreasoning".into(),
        }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: "◆ Ran ls 0.5s\noutput".into(),
        }],
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "All expanded => collapse all globally"
    );
}

#[test]
fn expand_all_when_all_collapsed() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text {
            content: "◆ Thought 1.0s\nreasoning".into(),
        }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: "◆ Ran ls 0.5s\noutput".into(),
        }],
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;

    state.update(Event::ToggleExpand);
    assert!(
        !state.view.all_collapsed,
        "All collapsed => expand all globally"
    );
}

#[test]
fn running_tools_always_expanded_regardless_of_global_flag() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text {
            content: "◆ Thought 1.0s".into(),
        }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: "⠋ Running ls...".into(),
        }],
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Global flag should flip");
    // Running tool renders as ToolRunning regardless of global flag
}

#[test]
fn toggle_all_empty_state_flips_flag() {
    let mut state = fresh_state();
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle on empty state should flip global flag"
    );
}

#[test]
fn toggle_all_twice_restores_expanded() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text {
            content: "◆ Thought 1.0s".into(),
        }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle 1: collapse all");

    state.update(Event::ToggleExpand);
    assert!(!state.view.all_collapsed, "Toggle 2: expand all");

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle 3: collapse all again");
}

#[test]
fn toggle_all_with_many_items() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.session.messages.push(ChatMessage {
            role: Role::Thought,
            parts: vec![Part::Text {
                content: format!("◆ Thought {}", i),
            }],
            timestamp: i as f64,
            id: format!("t{}", i),
            ..Default::default()
        });
    }

    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "All thoughts should be collapsed globally"
    );

    state.update(Event::ToggleExpand);
    assert!(
        !state.view.all_collapsed,
        "All thoughts should be expanded globally"
    );
}

#[test]
fn new_thought_respects_global_collapse_when_true() {
    let mut state = fresh_state();
    state.view.all_collapsed = true;

    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Reasoning".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.ensure_fresh();

    let feed = runie_core::view::LazyCache::feed(&state);
    let has_summary = feed.elements.iter().any(|e| {
        matches!(
            e,
            runie_core::view::elements::Element::ThoughtSummary { .. }
        )
    });
    assert!(
        has_summary,
        "New thought should be collapsed when all_collapsed=true"
    );
}

#[test]
fn new_thought_respects_global_expand_when_false() {
    let mut state = fresh_state();
    state.view.all_collapsed = false;

    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Reasoning".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.ensure_fresh();

    let feed = runie_core::view::LazyCache::feed(&state);
    let has_marker = feed
        .elements
        .iter()
        .any(|e| matches!(e, runie_core::view::elements::Element::ThoughtMarker { .. }));
    assert!(
        has_marker,
        "New thought should be expanded when all_collapsed=false"
    );
}

#[test]
fn new_tool_respects_global_collapse_when_true() {
    let mut state = fresh_state();
    state.view.all_collapsed = true;

    state.update(Event::ToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output: "a".to_string(),
    });
    state.ensure_fresh();

    let feed = runie_core::view::LazyCache::feed(&state);
    let has_summary = feed
        .elements
        .iter()
        .any(|e| matches!(e, runie_core::view::elements::Element::ToolSummary { .. }));
    assert!(
        has_summary,
        "New tool should be collapsed when all_collapsed=true"
    );
}

#[test]
fn new_tool_respects_global_expand_when_false() {
    let mut state = fresh_state();
    state.view.all_collapsed = false;

    state.update(Event::ToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        input: None,
        duration_secs: 0.5,
        output: "a".to_string(),
    });
    state.ensure_fresh();

    let feed = runie_core::view::LazyCache::feed(&state);
    let has_done = feed
        .elements
        .iter()
        .any(|e| matches!(e, runie_core::view::elements::Element::ToolDone { .. }));
    assert!(
        has_done,
        "New tool should be expanded when all_collapsed=false"
    );
}
