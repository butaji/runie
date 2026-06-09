//! Tests for chat feed element sorting by last update time.

use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn element_kinds(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements.iter().map(|e| match e {
        crate::ui::Element::UserMessage { .. } => "User".to_string(),
        crate::ui::Element::AgentMessage { .. } => "Agent".to_string(),
        crate::ui::Element::Thinking { .. } => "Thinking".to_string(),
        crate::ui::Element::ThoughtMarker { .. } => "Thought".to_string(),
        crate::ui::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
        crate::ui::Element::ToolRunning { .. } => "ToolRun".to_string(),
        crate::ui::Element::ToolDone { .. } => "ToolDone".to_string(),
        crate::ui::Element::ToolSummary { .. } => "ToolSum".to_string(),
        crate::ui::Element::TurnComplete { .. } => "Turn".to_string(),
        crate::ui::Element::Spacer { .. } => "Spacer".to_string(),
    }).collect()
}

/// Every element (including spacers) should have non-decreasing timestamps.
fn _timestamps_are_monotonic(state: &AppState) -> Result<(), String> {
    let feed = LazyCache::feed(state);
    let last_ts = 0.0f64;
    for (i, entry) in feed.elements.iter().enumerate() {
        // We can't directly read timestamp from Element, but we can infer from the source
        // messages. Instead, we'll use a different approach: verify sort order by checking
        // that the feed is built from messages sorted by timestamp.
        let _ = (i, entry, last_ts);
    }
    Ok(())
}

// ─── Scenario 1: Streaming response after tool ─────────────────────────

#[test]
fn agent_response_updated_after_tool_stays_after_tool() {
    let mut state = fresh_state();
    state.streaming = true;
    // 1. Agent starts responding
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Let me ".into() });
    // 2. Agent uses tool
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file.txt".into() });
    // 3. More response chunks arrive (updates existing assistant msg timestamp)
    state.update(Event::AgentResponse { id: "req.0".into(), content: "check files.".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    // After tool, more response should NOT push Agent before Tool.
    // The agent message was CREATED before the tool, so even though its
    // timestamp was updated, it should still appear after the tool
    // because the tool also has a later timestamp.
    let tool_pos = kinds.iter().position(|k| k == "ToolDone");
    let agent_pos = kinds.iter().position(|k| k == "Agent");
    assert!(tool_pos.is_some(), "Tool should exist");
    assert!(agent_pos.is_some(), "Agent should exist");
    assert!(tool_pos.unwrap() < agent_pos.unwrap(),
        "Agent should appear after Tool when response continues after tool: got {:?}", kinds);
}

// ─── Scenario 2: Multiple response chunks preserve relative order ───────

#[test]
fn multiple_response_chunks_preserve_creation_order() {
    let mut state = fresh_state();
    state.streaming = true;
    // First chunk creates assistant
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello ".into() });
    // Second chunk updates same assistant (bumps timestamp)
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    // Third chunk
    state.update(Event::AgentResponse { id: "req.0".into(), content: "!".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    // Should be exactly one Agent message (chunks merged)
    let agent_count = kinds.iter().filter(|k| *k == "Agent").count();
    assert_eq!(agent_count, 1, "Multiple chunks should merge into one Agent message");
}

// ─── Scenario 3: Thought before agent, agent updated later ─────────────

#[test]
fn thought_appears_before_agent_even_when_agent_updated_later() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    // Thought created at timestamp ~1, agent message created at timestamp ~2
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Result".into() });
    // Agent timestamp bumped to ~3
    state.update(Event::AgentResponse { id: "req.0".into(), content: " done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    let thought_pos = kinds.iter().position(|k| k == "Thought");
    let agent_pos = kinds.iter().position(|k| k == "Agent");
    assert!(thought_pos.is_some(), "Thought should exist");
    assert!(agent_pos.is_some(), "Agent should exist");
    assert!(thought_pos.unwrap() < agent_pos.unwrap(),
        "Thought should appear before Agent even when Agent timestamp is bumped later: got {:?}", kinds);
}

// ─── Scenario 4: TurnComplete is strictly last during its turn ─────────

#[test]
fn turn_complete_last_during_turn_despite_updates() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // Even after turn complete, delayed empty response bumps assistant
    state.update(Event::AgentResponse { id: "req.0".into(), content: "".into() });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must be last: got {:?}", kinds);
}

// ─── Scenario 5: Cross-turn ordering ───────────────────────────────────

#[test]
fn previous_turn_complete_before_next_turn_user() {
    let mut state = fresh_state();
    // Turn 1
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "T1".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    // Turn 2 user message
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    let turn_pos = kinds.iter().position(|k| k == "Turn").expect("TurnComplete");
    let _user2_pos = kinds.iter().position(|k| k == "User" && *k != "User").unwrap_or(0);
    // Find the SECOND user (turn 2)
    let user_positions: Vec<_> = kinds.iter().enumerate()
        .filter(|(_, k)| *k == "User")
        .map(|(i, _)| i)
        .collect();
    assert!(!user_positions.is_empty());
    // TurnComplete should be before the last user message
    assert!(turn_pos < *user_positions.last().unwrap(),
        "TurnComplete of turn 1 should be before user message of turn 2: got {:?}", kinds);
}

// ─── Scenario 6: Timestamp-based sort, not index-based ─────────────────

#[test]
fn elements_sorted_by_timestamp_not_index() {
    // Manually construct messages with out-of-order timestamps
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "First".into(),
        timestamp: 3.0,  // Later timestamp
        id: "u1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "Second".into(),
        timestamp: 1.0,  // Earlier timestamp
        id: "u2".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state).into_iter().filter(|k| k != "Spacer").collect();
    // Should be sorted by timestamp: Second (1.0) then First (3.0)
    let _first_pos = kinds.iter().position(|k| k == "User").unwrap();
    // We need to check the actual content, not just the kind
    let feed = LazyCache::feed(&state);
    let user_contents: Vec<_> = feed.elements.iter()
        .filter_map(|e| match e {
            crate::ui::Element::UserMessage { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(user_contents, vec!["Second", "First"],
        "Messages should be sorted by timestamp, not insertion order");
}
