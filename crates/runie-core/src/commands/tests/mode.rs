//! /mode command tests — pattern display, listing, switching, and validation.

use crate::commands::{CommandResult, DialogType};
use crate::model::AppState;

use super::exec_handler;

fn handle(state: &mut AppState, args: &str) -> CommandResult {
    crate::commands::dsl::handlers::mode::handle_mode(state, args)
}

#[test]
fn mode_command_is_registered() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "mode", "");
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModeSelector)),
        "expected mode selector dialog via registry, got {:?}",
        result
    );
}

#[test]
fn mode_empty_opens_interactive_selector() {
    let mut state = AppState::default();
    let result = handle(&mut state, "");
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModeSelector)),
        "expected mode selector dialog, got {:?}",
        result
    );
}

#[test]
fn mode_list_shows_all_patterns_with_descriptions_and_config() {
    let mut state = AppState::default();
    let result = handle(&mut state, "list");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("single"), "missing single: {}", msg);
        assert!(msg.contains("swarm"), "missing swarm: {}", msg);
        assert!(msg.contains("improve"), "missing improve: {}", msg);
        assert!(
            msg.contains("Direct execution"),
            "missing single description: {}",
            msg
        );
        assert!(
            msg.contains("Coordinated multi-agent work"),
            "missing swarm description: {}",
            msg
        );
        assert!(
            msg.contains("Iterative improvement with review"),
            "missing improve description: {}",
            msg
        );
        assert!(msg.contains("workers: 3"), "missing workers: {}", msg);
        assert!(msg.contains("max_rounds: 5"), "missing max_rounds: {}", msg);
    } else {
        panic!("expected Message, got {:?}", result);
    }
}

#[test]
fn mode_switch_to_pattern_emits_set_mode_event() {
    let mut state = AppState::default();
    for pattern in ["single", "swarm", "improve"] {
        let result = handle(&mut state, pattern);
        assert!(
            matches!(result, CommandResult::Event(crate::Event::SetMode { ref active, workers: None })
                if active == pattern),
            "expected SetMode event for {}, got {:?}",
            pattern,
            result
        );
    }
}

#[test]
fn mode_unknown_pattern_warns() {
    let mut state = AppState::default();
    let result = handle(&mut state, "bogus");
    assert!(
        matches!(result, CommandResult::Warning(ref msg)
            if msg.contains("bogus") && msg.contains("single") && msg.contains("swarm") && msg.contains("improve")),
        "expected warning listing valid patterns, got {:?}",
        result
    );
}

#[test]
fn mode_workers_override_emits_event() {
    let mut state = AppState::default();
    let result = handle(&mut state, "swarm workers 5");
    assert!(
        matches!(result, CommandResult::Event(crate::Event::SetMode { ref active, workers: Some(5) })
            if active == "swarm"),
        "expected SetMode swarm with workers 5, got {:?}",
        result
    );
    // workers override also works for non-swarm patterns.
    let result = handle(&mut state, "single workers 2");
    assert!(
        matches!(result, CommandResult::Event(crate::Event::SetMode { ref active, workers: Some(2) })
            if active == "single"),
        "expected SetMode single with workers 2, got {:?}",
        result
    );
}

#[test]
fn mode_workers_non_numeric_warns() {
    let mut state = AppState::default();
    let result = handle(&mut state, "swarm workers x");
    assert!(
        matches!(result, CommandResult::Warning(_)),
        "expected warning for non-numeric workers, got {:?}",
        result
    );
}

#[test]
fn mode_workers_zero_warns() {
    let mut state = AppState::default();
    let result = handle(&mut state, "swarm workers 0");
    assert!(
        matches!(result, CommandResult::Warning(_)),
        "expected warning for zero workers, got {:?}",
        result
    );
}

#[test]
fn mode_swarm_variant_sets_session_state() {
    let mut state = AppState::default();
    for variant in ["parallel", "delegation", "dag"] {
        let result = handle(&mut state, &format!("swarm {}", variant));
        assert!(
            matches!(result, CommandResult::Event(crate::Event::SetMode { ref active, workers: None })
                if active == "swarm"),
            "expected SetMode swarm event for variant {}, got {:?}",
            variant,
            result
        );
        assert_eq!(
            state.config().swarm_variant.as_deref(),
            Some(variant),
            "swarm_variant should be set to {}",
            variant
        );
    }
}

#[test]
fn mode_swarm_variant_with_task_text_still_switches() {
    let mut state = AppState::default();
    // Phase 1: trailing task text is accepted but not dispatched.
    let result = handle(&mut state, "swarm parallel \"process these 10 files\"");
    assert!(
        matches!(result, CommandResult::Event(crate::Event::SetMode { ref active, workers: None })
            if active == "swarm"),
        "expected SetMode swarm event, got {:?}",
        result
    );
    assert_eq!(state.config().swarm_variant.as_deref(), Some("parallel"));
}

#[test]
fn mode_swarm_bogus_variant_warns() {
    let mut state = AppState::default();
    let result = handle(&mut state, "swarm bogus-variant");
    assert!(
        matches!(result, CommandResult::Warning(_)),
        "expected warning for unknown swarm variant, got {:?}",
        result
    );
    assert_eq!(
        state.config().swarm_variant,
        None,
        "swarm_variant must stay unset after a rejected variant"
    );
}

#[test]
fn mode_switch_to_single_clears_swarm_variant() {
    let mut state = AppState::default();
    let result = handle(&mut state, "swarm parallel");
    state.apply_command_result(result);
    assert_eq!(state.config().swarm_variant.as_deref(), Some("parallel"));

    let result = handle(&mut state, "single");
    state.apply_command_result(result);
    assert_eq!(
        state.config().swarm_variant,
        None,
        "switching to single must clear swarm_variant"
    );
    assert_eq!(state.config().mode.active, "single");
}

#[test]
fn set_mode_event_updates_state_via_model_config_event() {
    let mut state = AppState::default();
    crate::update::agent::model_config_event(
        &mut state,
        crate::Event::SetMode { active: "swarm".into(), workers: None },
    );
    assert_eq!(state.config().mode.active, "swarm");
    // workers untouched when None
    assert_eq!(state.config().mode.workers, 3);

    crate::update::agent::model_config_event(
        &mut state,
        crate::Event::SetMode { active: "improve".into(), workers: Some(7) },
    );
    assert_eq!(state.config().mode.active, "improve");
    assert_eq!(state.config().mode.workers, 7);
    assert_eq!(
        state.config().swarm_variant,
        None,
        "non-swarm pattern must clear swarm_variant"
    );
}
