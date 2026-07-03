//! Safety command tests — read-only mode and trust system
use super::slash::exec;
use crate::model::AppState;
use crate::tests::fresh_state;

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

#[test]
fn toggle_flips_read_only() {
    let mut state = fresh_state();
    assert!(!state.config.read_only, "default is read-write");
    state.update(crate::Event::ToggleReadOnly);
    assert!(state.config.read_only, "toggled to read-only");
    state.update(crate::Event::ToggleReadOnly);
    assert!(!state.config.read_only, "toggled back to read-write");
}

#[test]
fn slash_readonly_toggles() {
    let mut state = fresh_state();
    assert!(!state.config.read_only);
    palette_select(&mut state, "readonly");
    assert!(state.config.read_only, "/readonly toggles read_only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("Read-only mode enabled"),
        "confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning)
    );
}

#[test]
fn slash_ro_alias_toggles() {
    let mut state = fresh_state();
    exec(&mut state, "/ro");
    assert!(state.config.read_only, "/ro alias toggles read_only");
}

#[test]
fn slash_trust_sets_trusted() {
    let mut state = fresh_state();
    state.config.read_only = true;
    palette_select(&mut state, "trust");
    assert!(!state.config.read_only, "/trust disables read-only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("trusted"),
        "trust confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn slash_untrust_sets_untrusted() {
    let mut state = fresh_state();
    palette_select(&mut state, "untrust");
    assert!(state.config.read_only, "/untrust enables read-only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("untrusted"),
        "untrust confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning)
    );
}

#[test]
fn slash_approve_applies_pending_edits() {
    let mut state = fresh_state();
    let file = std::env::temp_dir().join(format!("runie_approve_test_{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&file);
    state
        .session
        .pending_edits
        .push(crate::edit_preview::EditPreview::new(
            camino::Utf8PathBuf::from_path_buf(file.clone()).unwrap(),
            "old".into(),
            "new content".into(),
        ));

    exec(&mut state, "/approve");

    assert!(
        state.session.pending_edits.is_empty(),
        "pending edits cleared"
    );
    let written = std::fs::read_to_string(&file).unwrap_or_default();
    assert_eq!(written, "new content", "edit applied");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn slash_reject_clears_pending_edits() {
    let mut state = fresh_state();
    let file = std::env::temp_dir().join(format!("runie_reject_test_{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&file);
    state
        .session
        .pending_edits
        .push(crate::edit_preview::EditPreview::new(
            camino::Utf8PathBuf::from_path_buf(file.clone()).unwrap(),
            "old".into(),
            "new content".into(),
        ));

    exec(&mut state, "/reject");

    assert!(
        state.session.pending_edits.is_empty(),
        "pending edits cleared"
    );
    assert!(!file.exists(), "file should not be written");
}

#[test]
fn slash_approve_without_edits_warns() {
    let mut state = fresh_state();
    exec(&mut state, "/approve");

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content().contains("No pending edits"),
        "expected warning: {}",
        last.content()
    );
}
