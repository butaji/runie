use crate::dsl::AppStateDsl;
use crate::model::AppState;
use crate::tests::fresh_state;
use crate::view::LazyCache;

fn element_kinds(state: &AppState) -> Vec<String> {
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
        .collect()
}

#[test]
fn completed_tool_with_running_in_name_renders_as_tool_done() {
    let mut state = fresh_state();
    state.agent("req.0").tool("listRunningProcs", "pid 123");
    state.ensure_fresh();
    let k = element_kinds(&state);
    assert!(
        k.iter().any(|x| x == "ToolDone"),
        "Should be ToolDone. Got: {:?}",
        k
    );
    assert!(
        !k.iter().any(|x| x == "ToolRun"),
        "Should NOT be ToolRun. Got: {:?}",
        k
    );
}

#[test]
fn completed_tool_running_check_does_not_show_timer() {
    let mut state = fresh_state();
    state.agent("req.0").tool("isRunning", "yes");
    state.ensure_fresh();
    for elem in &LazyCache::feed(&state).elements {
        if let crate::view::Element::ToolRunning { name, .. } = elem {
            panic!("Should not have ToolRunning for completed tool '{}'", name);
        }
    }
}

#[test]
fn finish_turn_does_not_clear_next_turns_thinking() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.agent("req.0").think().respond("T1");
    state.agent("req.1").think();
    state.agent("req.0").done();
    assert!(
        state.agent.thinking_started_at.is_some(),
        "must NOT clear next turn's thinking"
    );
}

#[test]
fn next_turn_thinking_shows_after_previous_turn_complete() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state
        .agent("req.0")
        .think()
        .thought_done()
        .tool("ls", "a")
        .respond("First")
        .complete(1.0)
        .done();
    state.agent("req.1").think();
    state.ensure_fresh();
    let k: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|x| x != "Spacer")
        .collect();
    let turn_pos = k.iter().position(|x| x == "Turn");
    let thinking_pos = k.iter().position(|x| x == "Thinking");
    assert!(turn_pos.is_some() && thinking_pos.is_some());
    assert!(turn_pos.unwrap() < thinking_pos.unwrap(), "Got: {:?}", k);
}

#[test]
fn thinking_indicator_gone_after_thought_done() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state
        .agent("req.0")
        .think()
        .thought_done()
        .respond("Done")
        .complete(1.0)
        .done();
    state.ensure_fresh();
    let k: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|x| x != "Spacer")
        .collect();
    assert!(!k.iter().any(|x| x == "Thinking"), "Got: {:?}", k);
}

#[test]
fn only_one_turn_complete_after_done() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state
        .agent("req.0")
        .think()
        .thought_done()
        .tool("ls", "a")
        .respond("Hello")
        .complete(1.0)
        .done();
    state.ensure_fresh();
    let k: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|x| x != "Spacer")
        .collect();
    assert_eq!(k.iter().filter(|x| *x == "Turn").count(), 1, "Got: {:?}", k);
}

#[test]
fn turn_complete_timestamp_monotonically_increases() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state
        .agent("req.0")
        .think()
        .thought_done()
        .tool("ls", "a")
        .respond("A")
        .complete(1.0);
    let ts1 = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();
    state.agent("req.0").respond("B");
    let ts2 = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();
    assert!(
        ts2 >= ts1,
        "TurnComplete timestamp must not regress: {} -> {}",
        ts1,
        ts2
    );
}
