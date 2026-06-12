use crate::model::AppState;
use crate::dsl::AppStateDsl;
use crate::model::Role;

#[test]
fn dsl_type_text_and_submit() {
    let mut state = AppState::default();
    state.type_text("hello").submit();
    assert_eq!(state.input.input, "");
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content, "hello");
}

#[test]
fn dsl_agent_turn_full() {
    let mut state = AppState::default();
    state.type_text("a").submit();
    state.agent("req.0")
        .think()
        .respond("hello")
        .thought_done()
        .tool("ls", "file1")
        .respond("done")
        .complete(1.0)
        .done();
    assert!(!state.agent.turn_active);
    assert_eq!(state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).count(), 1);
}

#[test]
fn dsl_multiple_turns() {
    let mut state = AppState::default();
    state.type_text("a").submit();
    state.agent("req.0").think().respond("first").complete(1.0).done();
    state.type_text("b").submit();
    state.agent("req.1").think().respond("second").complete(1.0).done();

    let turns: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).collect();
    assert_eq!(turns.len(), 2);
}
