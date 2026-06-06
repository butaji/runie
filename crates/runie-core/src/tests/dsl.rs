use crate::model::{AppState, Role};
use crate::event::Event;
use crate::ui::LazyCache;
use crate::ui::elements::Element;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_dsl_combines_consecutive_agent_chunks() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello ".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "World!".to_string() });
    let feed = LazyCache::feed(&state);
    assert_eq!(feed.elements.len(), 4);
    if let Element::AgentMessage { content, .. } = &feed.elements[2] {
        assert_eq!(content, "Hello World!");
    } else {
        panic!("Expected AgentMessage at index 2");
    }
}

#[test]
fn test_dsl_shows_thinking_when_streaming() {
    use crate::ui::format_test::format_messages;
    let mut state = fresh_state();
    state.streaming = true;
    state.request_queue.push_back(("B".to_string(), "req.1".to_string()));
    state.thinking_started_at = Some(std::time::Instant::now());
    let lines = format_messages(&state);
    let content: String = lines.iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();
    assert!(content.contains("Thinking"));
}

#[test]
fn test_multiple_thoughts_for_sequential_requests() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == Role::Thought).collect();
    assert_eq!(thoughts.len(), 2);
}

#[test]
fn feed_sorted_by_last_update_timestamp() {
    let mut state = fresh_state();
    state.messages.push(crate::model::ChatMessage { role: Role::User, content: "Q1".into(), timestamp: 0.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Assistant, content: "A1".into(), timestamp: 1.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::User, content: "Q2".into(), timestamp: 2.0, id: "t2".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Assistant, content: "A2".into(), timestamp: 3.0, id: "t2".into() });
    state.messages[1].timestamp = 5.0;
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        Element::UserMessage { .. } => "U",
        Element::AgentMessage { .. } => "A",
        Element::Spacer => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["U", "S", "U", "S", "A", "S", "A", "S"], "Most recently updated element (A1 @ t=5) must be at bottom");
}

#[test]
fn thought_before_assistant_even_when_assistant_updated_later() {
    let mut state = fresh_state();
    state.messages.push(crate::model::ChatMessage { role: Role::User, content: "Q".into(), timestamp: 0.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Assistant, content: "A".into(), timestamp: 1.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Thought, content: "Thought 1s".into(), timestamp: 2.0, id: "t1".into() });
    state.messages[1].timestamp = 5.0;
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        Element::UserMessage { .. } => "U",
        Element::ThoughtMarker { .. } => "T",
        Element::AgentMessage { .. } => "A",
        Element::Spacer => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["U", "S", "T", "S", "A", "S"], "Thought must appear before Assistant even when Assistant has later timestamp");
}

#[test]
fn tool_floats_with_turns_latest_update() {
    let mut state = fresh_state();
    state.messages.push(crate::model::ChatMessage { role: Role::User, content: "Q".into(), timestamp: 0.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Thought, content: "T".into(), timestamp: 1.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Tool, content: "Ran list_files 0.5s".into(), timestamp: 2.0, id: "t1".into() });
    state.messages.push(crate::model::ChatMessage { role: Role::Assistant, content: "A".into(), timestamp: 3.0, id: "t1".into() });
    state.messages[2].timestamp = 6.0;
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        Element::UserMessage { .. } => "U",
        Element::ThoughtMarker { .. } => "T",
        Element::ToolDone { .. } => "D",
        Element::AgentMessage { .. } => "A",
        Element::Spacer => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["U", "S", "T", "S", "A", "S", "D", "S"], "Tool (updated t=6) should float to bottom of its turn");
}
