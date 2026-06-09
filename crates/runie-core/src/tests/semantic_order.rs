use crate::model::AppState;
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

/// The bug: when agent response arrives before tool (mock provider),
/// large tool output pushes the agent response above the viewport.
/// After finish_turn, the final agent response must be AFTER tools.
#[test]
fn final_agent_after_tools_when_turn_completes() {
    let mut state = fresh_state();
    state.streaming = true;

    // Simulate mock provider event order: response BEFORE tool
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    // Find positions of Agent and Tool in element feed
    let feed = crate::ui::LazyCache::feed(&state);
    let agent_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::AgentMessage { content, .. } if content == "Done!"));
    let tool_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::ToolDone { .. }));

    assert!(agent_pos.is_some(), "Agent message must exist");
    assert!(tool_pos.is_some(), "Tool message must exist");
    assert!(
        agent_pos.unwrap() > tool_pos.unwrap(),
        "Final agent response must appear AFTER tool output. Got agent at {:?}, tool at {:?}",
        agent_pos, tool_pos
    );
}

#[test]
fn final_agent_visible_when_tool_overflows() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    state.scroll = 0;

    // Viewport of 5 lines — tool is 21 lines + spacer = 22, agent is 1 + spacer = 2
    // Total ~30 lines. With agent AFTER tool, bottom 5 lines should include agent.
    let region = state.visible_scroll(5);
    let has_agent = region.elements.iter().any(|e| matches!(e, crate::ui::Element::AgentMessage { content, .. } if content == "Done!"));
    assert!(has_agent, "Final agent 'Done!' must be visible at bottom");
}

#[test]
fn agent_before_tool_preserved_during_turn() {
    let mut state = fresh_state();
    state.streaming = true;

    // During the turn, before done, agent IS before tool (timestamp order)
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let agent_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::AgentMessage { .. }));
    let tool_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::ToolDone { .. }));

    // Before finish_turn, agent can be before tool (streaming order)
    assert!(agent_pos.is_some() && tool_pos.is_some());
}

#[test]
fn no_reorder_when_no_tools() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        crate::ui::Element::AgentMessage { .. } => "A",
        crate::ui::Element::TurnComplete { .. } => "T",
        crate::ui::Element::Spacer { .. } => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["A", "S", "T", "S"], "No tools: agent before turn complete");
}

#[test]
fn thought_stays_before_tool_after_reorder() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let thought_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }));
    let tool_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::ToolDone { .. }));
    let agent_pos = feed.elements.iter().position(|e| matches!(e, crate::ui::Element::AgentMessage { content, .. } if content == "Done!"));

    assert!(thought_pos.unwrap() < tool_pos.unwrap(), "Thought must be before tool");
    assert!(tool_pos.unwrap() < agent_pos.unwrap(), "Agent must be after tool");
}
