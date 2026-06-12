use crate::event::Event;
use crate::model::AppState;
use crate::ui::elements::Element;
use crate::ui::LazyCache;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

#[test]
fn global_collapse_persists_through_rapid_events() {
    let mut state = AppState::default();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\n".into() },
        Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
    ]);
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "Global flag should be set");
    dispatch(&mut state, &[
        Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() },
        Event::AgentResponse { id: "req.0".into(), content: "Done.".into() },
        Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 },
        Event::AgentDone { id: "req.0".into() },
    ]);
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
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "A".into() },
        Event::AgentResponse { id: "req.0".into(), content: "TOOL:ls.".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "ls".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() },
        Event::AgentDone { id: "req.0".into() },
    ]);
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed);
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.1".into() },
        Event::AgentResponse { id: "req.1".into(), content: "B".into() },
        Event::AgentResponse { id: "req.1".into(), content: "TOOL:grep.".into() },
        Event::AgentThoughtDone { id: "req.1".into() },
        Event::AgentToolStart { id: "req.1".into(), name: "grep".into() },
        Event::AgentToolEnd { duration_secs: 0.3, output: "b".into() },
        Event::AgentDone { id: "req.1".into() },
    ]);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    assert_eq!(summaries.len(), 2, "Both turns' thoughts should be collapsed");
    let tool_summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolSummary { .. })).collect();
    assert_eq!(tool_summaries.len(), 2, "Both turns' tools should be collapsed");
}

#[test]
fn global_collapse_persists_through_multiple_tools_in_one_turn() {
    let mut state = AppState::default();
    state.streaming = true;
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "I'll do two things.\n".into() },
        Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "ls".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() },
    ]);
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed);
    dispatch(&mut state, &[
        Event::AgentThinking { id: "req.0".into() },
        Event::AgentResponse { id: "req.0".into(), content: "Now grep.\n".into() },
        Event::AgentResponse { id: "req.0".into(), content: "TOOL:grep:fn main:.".into() },
        Event::AgentThoughtDone { id: "req.0".into() },
        Event::AgentToolStart { id: "req.0".into(), name: "grep".into() },
        Event::AgentToolEnd { duration_secs: 0.3, output: "result".into() },
        Event::AgentDone { id: "req.0".into() },
    ]);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let tool_summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolSummary { .. })).collect();
    assert_eq!(tool_summaries.len(), 2, "BOTH tools should be collapsed with global flag");
}

#[test]
fn multiple_thoughts_all_follow_global_flag() {
    let mut state = AppState::default();
    state.streaming = true;
    for n in 0..3 {
        let id = format!("req.{}", n);
        dispatch(&mut state, &[
            Event::AgentThinking { id: id.clone() },
            Event::AgentResponse { id: id.clone(), content: format!("reasoning {}\n", n) },
            Event::AgentResponse { id: id.clone(), content: "TOOL:ls.".into() },
            Event::AgentThoughtDone { id: id.clone() },
            Event::AgentToolStart { id: id.clone(), name: "ls".into() },
            Event::AgentToolEnd { duration_secs: 0.1, output: format!("out{}", n) },
            Event::AgentDone { id: id.clone() },
        ]);
    }
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    assert_eq!(summaries.len(), 3, "All three thoughts should be collapsed");
    let markers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtMarker { .. })).collect();
    assert_eq!(markers.len(), 0, "No thoughts should be expanded");
}
