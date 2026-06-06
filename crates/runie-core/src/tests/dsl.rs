use crate::model::AppState;
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
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello ".to_string()
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "World!".to_string()
    });

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
    use crate::ui::format_messages;

    let mut state = fresh_state();
    state.streaming = true;
    state.request_queue.push(("B".to_string(), "req.1".to_string()));
    state.thinking_started_at = Some(std::time::Instant::now());

    let lines = format_messages(&state);
    let content: String = lines.iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
        .collect();

    assert!(content.contains("Though"));
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

    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
    assert_eq!(thoughts.len(), 2);
}
