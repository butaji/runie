use crate::commands::CommandResult;
use crate::model::AppState;
use crate::skills::Skill;
use crate::Event;

use super::{exec_handler, run_slash};

fn rust_skill(user_invocable: bool) -> Skill {
    Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable,
        file_path: std::path::PathBuf::from("rust.md"),
    }
}

#[test]
fn skills_lists_loaded() {
    let mut state = AppState {
        skills: vec![rust_skill(false)],
        ..Default::default()
    };
    let result = exec_handler(&mut state, "skills", "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("rust"), "Should list skill name, got: {}", msg);
        assert!(
            msg.contains("Rust best practices"),
            "Should list skill description, got: {}",
            msg
        );
    } else {
        panic!("/skills should return Message, got {:?}", result);
    }
}

#[test]
fn skills_empty_shows_warning() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "skills", "");
    if let CommandResult::Warning(msg) = result {
        assert!(msg.contains("No skills loaded"), "got: {}", msg);
    } else {
        panic!(
            "/skills with no skills should return Warning, got {:?}",
            result
        );
    }
}

#[test]
fn slash_skills_empty_emits_warning_transient() {
    let mut state = AppState::default();
    run_slash(&mut state, "/skills");
    assert_eq!(
        state.transient_message,
        Some("No skills loaded.".into()),
        "Empty /skills should produce a transient warning"
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning),
        "Empty /skills should have warning level"
    );
    assert!(
        state.session.messages.is_empty(),
        "Empty /skills must not publish to the feed"
    );
}

#[test]
fn skill_shows_info() {
    let mut state = AppState {
        skills: vec![rust_skill(true)],
        ..Default::default()
    };
    let result = exec_handler(&mut state, "skill", "rust");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("rust"), "Should show skill name, got: {}", msg);
        assert!(
            msg.contains("Use clippy"),
            "Should show skill context, got: {}",
            msg
        );
    } else {
        panic!("/skill rust should return Message, got {:?}", result);
    }
}

#[test]
fn skill_unknown_returns_error() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "skill", "unknown");
    if let CommandResult::Message(msg) = result {
        assert!(
            msg.contains("not found"),
            "Should report unknown skill, got: {}",
            msg
        );
    } else {
        panic!(
            "/skill unknown should return error Message, got {:?}",
            result
        );
    }
}

#[test]
fn palette_shows_user_invocable_skills() {
    let mut state = AppState {
        skills: vec![rust_skill(true)],
        ..Default::default()
    };
    state.update(crate::Event::ToggleCommandPalette);
    let snap = state.snapshot();
    assert!(
        snap.palette_items
            .iter()
            .any(|(n, _, c)| n == "rust" && c == "Skill"),
        "User-invocable skill should appear in palette items: {:?}",
        snap.palette_items
    );
}

#[test]
fn palette_select_skill_emits_message() {
    let mut state = AppState {
        skills: vec![rust_skill(true)],
        ..Default::default()
    };
    state.update(crate::Event::ToggleCommandPalette);
    let snap = state.snapshot();
    let skill_pos = snap
        .palette_items
        .iter()
        .position(|(n, _, c)| n == "rust" && c == "Skill")
        .expect("skill should be in palette");
    for _ in 0..skill_pos {
        state.update(crate::Event::PaletteDown);
    }
    state.update(crate::Event::PaletteSelect);
    let last = state
        .session
        .messages
        .last()
        .expect("should have a message");
    assert!(
        last.content().contains("rust"),
        "Selecting skill should emit info message: {}",
        last.content()
    );
}
