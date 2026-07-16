//! Unit tests for the `/goal` slash command.

use super::exec;
use crate::commands::CommandResult;
use crate::commands::{DialogKind, DialogState};
use crate::model::GoalStatus;
use crate::tests::fresh_state;

#[test]
fn goal_create_sets_goal_state() {
    let mut state = fresh_state();
    assert!(state.goal_state().is_none());

    exec(&mut state, "/goal -- Build a great CLI tool");

    let goal = state.goal_state().expect("goal should be created");
    assert_eq!(goal.objective, "Build a great CLI tool");
    assert_eq!(goal.status, GoalStatus::Active);
}

#[test]
fn goal_create_emits_event() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Write tests");
    // The handler emits Event::GoalCreate via CommandResult::Event
    // We verify the state was updated (side effect of the event)
    let goal = state.goal_state().expect("goal should be created");
    assert_eq!(goal.objective, "Write tests");
}

#[test]
fn goal_create_without_dash_parses_objective() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Implement feature X");

    let goal = state.goal_state().expect("goal should be created");
    assert_eq!(goal.objective, "Implement feature X");
}

#[test]
fn goal_status_no_goal_returns_message() {
    let mut state = fresh_state();
    state.input_mut().input = "/goal status".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    // No open dialog — CommandResult::Message returned
    assert!(state.open_dialog().is_none());
    let sys_msgs: Vec<_> = state.session.messages.iter().filter(|m| {
        m.role == crate::model::Role::System
    }).collect();
    assert!(!sys_msgs.is_empty(), "should have a system message");
}

#[test]
fn goal_status_with_goal_opens_panel() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Test the goal feature");

    // Clear messages from creation
    state.session.messages.retain(|_| false);

    exec(&mut state, "/goal status");

    assert!(
        matches!(
            state.open_dialog(),
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                panels: _,
            })
        ),
        "/goal status should open a panel"
    );
}

#[test]
fn goal_pause_pauses_active_goal() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Active goal");

    state.input_mut().input = "/goal pause".into();
    state.input_mut().cursor_pos = 10;
    state.update(crate::Event::Submit);

    let goal = state.goal_state().expect("goal should exist");
    assert_eq!(goal.status, GoalStatus::Paused);
}

#[test]
fn goal_pause_warns_when_no_goal() {
    let mut state = fresh_state();
    state.input_mut().input = "/goal pause".into();
    state.input_mut().cursor_pos = 10;
    state.update(crate::Event::Submit);

    // CommandResult::Warning — no dialog opened, goal state unchanged
    assert!(state.open_dialog().is_none());
    assert!(state.goal_state().is_none());
}

#[test]
fn goal_pause_warns_when_already_paused() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- My goal");
    // Manually set to paused
    state.goal_state_mut().as_mut().unwrap().status = GoalStatus::Paused;

    state.input_mut().input = "/goal pause".into();
    state.input_mut().cursor_pos = 10;
    state.update(crate::Event::Submit);

    // Still paused, no state change
    assert_eq!(state.goal_state().unwrap().status, GoalStatus::Paused);
}

#[test]
fn goal_resume_resumes_paused_goal() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Resumable goal");
    // Manually pause it
    state.goal_state_mut().as_mut().unwrap().status = GoalStatus::Paused;

    state.input_mut().input = "/goal resume".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    assert_eq!(state.goal_state().unwrap().status, GoalStatus::Active);
}

#[test]
fn goal_resume_warns_when_no_goal() {
    let mut state = fresh_state();
    state.input_mut().input = "/goal resume".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    assert!(state.open_dialog().is_none());
    assert!(state.goal_state().is_none());
}

#[test]
fn goal_resume_warns_when_active() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Already active");
    // Already active
    state.input_mut().input = "/goal resume".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    assert!(state.open_dialog().is_none());
    assert_eq!(state.goal_state().unwrap().status, GoalStatus::Active);
}

#[test]
fn goal_cancel_clears_goal() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- To be cancelled");
    assert!(state.goal_state().is_some());

    state.input_mut().input = "/goal cancel".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    assert!(state.goal_state().is_none(), "goal should be cancelled");
}

#[test]
fn goal_cancel_warns_when_no_goal() {
    let mut state = fresh_state();
    state.input_mut().input = "/goal cancel".into();
    state.input_mut().cursor_pos = 11;
    state.update(crate::Event::Submit);

    assert!(state.open_dialog().is_none());
    assert!(state.goal_state().is_none());
}

#[test]
fn goal_replace_updates_objective() {
    let mut state = fresh_state();
    exec(&mut state, "/goal -- Original objective");

    state.input_mut().input = "/goal --replace -- New objective".into();
    state.input_mut().cursor_pos = 26;
    state.update(crate::Event::Submit);

    let goal = state.goal_state().expect("goal should exist");
    assert_eq!(goal.objective, "New objective");
    assert_eq!(goal.status, GoalStatus::Active);
}

#[test]
fn goal_empty_objective_warns() {
    let mut state = fresh_state();
    state.input_mut().input = "/goal --".into();
    state.input_mut().cursor_pos = 7;
    state.update(crate::Event::Submit);

    // Should warn about empty objective
    assert!(state.open_dialog().is_none());
    assert!(state.goal_state().is_none(), "empty goal should not be created");
}
