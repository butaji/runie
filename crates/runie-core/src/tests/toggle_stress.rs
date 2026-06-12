use crate::model::{AppState, Role};
use crate::event::Event;
use crate::ui::LazyCache;
use crate::ui::elements::Element;

#[test]
fn global_collapse_persists_through_rapid_events() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll list files.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "Global flag should be set");

    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Done.".to_string() });
    state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".to_string() });

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let thought_elems: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    assert_eq!(thought_elems.len(), 1, "Thought should be collapsed after rapid events");
    let tool_elems: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolSummary { .. })).collect();
    assert_eq!(tool_elems.len(), 1, "Tool should also be collapsed");
}

#[test]
fn global_collapse_persists_when_second_turn_starts() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });

    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "Global flag should be set");

    // Second turn starts immediately
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    let markers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtMarker { .. })).collect();
    assert_eq!(summaries.len(), 2, "BOTH thoughts should be collapsed with global flag");
    assert_eq!(markers.len(), 0, "No thoughts should be expanded");
}

#[test]
fn global_collapse_persists_through_multiple_tools_in_one_turn() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll do two things.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "ls".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".to_string() });

    // Toggle — collapse all
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed);

    // Second tool iteration
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Now grep.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:grep:fn main:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "grep".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "result".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let tool_summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolSummary { .. })).collect();
    assert_eq!(tool_summaries.len(), 2, "BOTH tools should be collapsed with global flag");
}

#[test]
fn multiple_thoughts_all_follow_global_flag() {
    let mut state = AppState::default();
    state.streaming = true;

    // First thinking phase
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "First reasoning.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:ls:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "ls".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.1, output: "a".to_string() });

    // Second thinking phase (same turn, next iteration)
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Second reasoning.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    let thoughts: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::Thought).collect();
    assert_eq!(thoughts.len(), 2, "Should have two thoughts");

    // Toggle — collapse ALL thoughts
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed);

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    let markers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtMarker { .. })).collect();

    assert_eq!(summaries.len(), 2, "BOTH thoughts should be collapsed with global flag");
    assert_eq!(markers.len(), 0, "No thoughts should be expanded");
}
