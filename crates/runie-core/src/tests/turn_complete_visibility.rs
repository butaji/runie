use crate::model::{AppState, Role};
use crate::tests::fresh_state;
use crate::view::LazyCache;
use crate::Event;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

fn element_kinds_no_spacer(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .map(|e| match e {
            crate::view::Element::UserMessage { .. } => "User".to_string(),
            crate::view::Element::AgentMessage { .. } => "Agent".to_string(),
            crate::view::Element::Thinking { .. } => "Thinking".to_string(),
            crate::view::Element::ThoughtMarker { .. } => "Thought".to_string(),
            crate::view::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
            crate::view::Element::ToolRunning { .. } => "ToolRun".to_string(),
            crate::view::Element::ToolDone { .. } => "ToolDone".to_string(),
            crate::view::Element::ToolSummary { .. } => "ToolSum".to_string(),
            crate::view::Element::ContextGroup { .. } => "Context".to_string(),
            crate::view::Element::TurnComplete { .. } => "Turn".to_string(),
            crate::view::Element::Spacer { .. } => "Spacer".to_string(),
        })
        .filter(|k| k != "Spacer")
        .collect()
}

fn feed_has_turn_complete(state: &AppState) -> bool {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .any(|e| matches!(e, crate::view::Element::TurnComplete { .. }))
}

#[test]
fn single_thought_hides_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.5,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

#[test]
fn tool_plus_thought_shows_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.3,
        output: "file1".into(),
    
        input: None,});
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Found it".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn tool_only_hides_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "start".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.3,
        output: "file1".into(),
    
        input: None,});
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

#[test]
fn two_thoughts_shows_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Answer".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn two_tools_shows_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.1,
        output: "a".into(),
    
        input: None,});
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "cat".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.2,
        output: "b".into(),
    
        input: None,});
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 3.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn mixed_thought_tool_shows_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),
    
        input: None,});
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.5,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn zero_actions_hides_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.1,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

fn first_turn_events() -> Vec<Event> {
    vec![
        crate::Event::Thinking { id: "req.0".into() },
        crate::Event::ThoughtDone { id: "req.0".into() },
        crate::Event::ToolStart {
            id: "req.0".into(),
            name: "ls".into(),
            input: serde_json::Value::Null,
        },
        crate::Event::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.5,
            output: "a".into(),
        
        input: None,},
        crate::Event::Response {
            id: "req.0".into(),
            content: "First".into(),
        
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),},
        crate::Event::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        crate::Event::Done { id: "req.0".into() },
    ]
}

fn second_turn_events() -> Vec<Event> {
    vec![
        crate::Event::Thinking { id: "req.1".into() },
        crate::Event::ToolStart {
            id: "req.1".into(),
            name: "cat".into(),
            input: serde_json::Value::Null,
        },
        crate::Event::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.3,
            output: "b".into(),
        
        input: None,},
        crate::Event::Response {
            id: "req.1".into(),
            content: "Second".into(),
        
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),},
        crate::Event::TurnComplete {
            id: "req.1".into(),
            duration_secs: 0.8,
        },
        crate::Event::Done { id: "req.1".into() },
    ]
}

#[test]
fn second_turn_independent_action_count() {
    let mut state = fresh_state();
    state.set_streaming(true);
    dispatch(&mut state, &first_turn_events());
    dispatch(&mut state, &second_turn_events());
    state.ensure_fresh();
    let kinds = element_kinds_no_spacer(&state);
    let turn_count = kinds.iter().filter(|k| *k == "Turn").count();
    assert_eq!(
        turn_count, 1,
        "Only turn 1's TurnComplete should be visible; got {:?}",
        kinds
    );
}

#[test]
fn three_mixed_actions_shows_turn_complete() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.1,
        output: "a".into(),
    
        input: None,});
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn turn_complete_still_in_session_when_hidden() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),
    
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),});
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.5,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
    state.ensure_fresh();
    let turn_msgs = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::TurnComplete)
        .count();
    assert_eq!(turn_msgs, 1);
    assert!(!feed_has_turn_complete(&state));
}
