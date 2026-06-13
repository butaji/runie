use crate::dsl::AppStateDsl;
use crate::model::{AppState, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

fn run_turn_with_tool(state: &mut AppState, id: &str, tool_name: &str) {
    state
        .agent(id)
        .think()
        .respond(format!("I'll list files.\nTOOL:{}:.", tool_name))
        .thought_done()
        .tool(tool_name, "file1\nfile2")
        .respond("Done.")
        .complete(1.0)
        .done();
}

#[test]
fn rapid_double_submit_both_messages_present() {
    let mut state = fresh_state();
    state.type_text("list").submit();
    state.type_text("list").submit();
    assert_eq!(
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::User)
            .count(),
        2
    );
}

#[test]
fn rapid_double_submit_different_ids() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.type_text("b").submit();
    let ids: Vec<String> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::User)
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn rapid_submit_turn_active_clears_after_done() {
    let mut state = fresh_state();
    state.type_text("x").submit();
    state
        .agent("req.0")
        .think()
        .respond("first")
        .complete(1.0)
        .done();
    assert!(!state.agent.turn_active);
    assert!(state.agent.current_action.is_none());
}

#[test]
fn second_submit_while_first_active_queues_correctly() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.agent("req.0").think();
    state.type_text("b").submit();
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.agent.message_queue[0].content, "b");
}

#[test]
fn rapid_submit_both_turns_complete() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state
        .agent("req.0")
        .think()
        .respond("first")
        .complete(1.0)
        .done();
    state
        .agent("req.1")
        .think()
        .respond("second")
        .complete(1.0)
        .done();
    assert_eq!(
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::TurnComplete)
            .count(),
        2
    );
}

#[test]
fn turn_complete_not_overwritten_by_second_turn() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.agent("req.0").think().respond("first").complete(1.0);
    state.type_text("b").submit();
    state
        .agent("req.1")
        .think()
        .respond("second")
        .complete(2.0)
        .done();
    assert_eq!(
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::TurnComplete)
            .count(),
        2
    );
}

#[test]
fn deliver_queued_moves_steering_to_request_queue() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.agent("req.0").think();
    state.type_text("b").submit();
    assert_eq!(state.agent.message_queue.len(), 1);
    state.agent("req.0").respond("done").complete(1.0).done();
    assert!(state.agent.message_queue.is_empty());
    let queued: Vec<String> = state
        .agent
        .request_queue
        .iter()
        .map(|(c, _)| c.clone())
        .collect();
    assert!(queued.contains(&"b".to_string()));
}

#[test]
fn two_full_turns_with_tools_state_correct() {
    let mut state = fresh_state();
    state.type_text("list").submit();
    run_turn_with_tool(&mut state, "req.0", "list_dir");
    run_turn_with_tool(&mut state, "req.1", "list_dir");
    assert!(!state.agent.turn_active);
    assert!(state.agent.current_action.is_none());
    assert_eq!(state.agent.inflight, 0);
    assert_eq!(
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::TurnComplete)
            .count(),
        2
    );
}

#[test]
fn steering_message_delivered_after_done() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    state.agent("req.0").think();
    state.type_text("b").submit();
    assert_eq!(state.agent.message_queue.len(), 1);
    state.agent("req.0").respond("done").complete(1.0).done();
    assert!(state.agent.message_queue.is_empty());
}

#[test]
fn end_tool_updates_content() {
    let mut state = fresh_state();
    state.agent("req.0").tool("list_dir", "");
    let tool = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Tool)
        .unwrap();
    assert!(
        tool.content.contains("✓"),
        "Tool should be done: {}",
        tool.content
    );
}

#[test]
fn two_turns_with_tools_no_stuck_timer() {
    let mut state = fresh_state();
    state.type_text("a").submit();
    run_turn_with_tool(&mut state, "req.0", "list_dir");
    assert!(state.agent.tool_started_at.is_none());
    run_turn_with_tool(&mut state, "req.1", "list_dir");
    assert!(state.agent.tool_started_at.is_none());
    for tool in state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
    {
        assert!(
            !tool.content.contains("⠋ Running"),
            "Tool should be done: {}",
            tool.content
        );
    }
}
