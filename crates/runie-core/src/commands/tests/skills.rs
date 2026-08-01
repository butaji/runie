use crate::commands::CommandResult;
use crate::model::AppState;
use crate::skills::Skill;

use super::{exec_handler, run_slash};

fn rust_skill(user_invocable: bool) -> Skill {
    Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable,
        file_path: camino::Utf8PathBuf::from("rust.md"),
        scope: crate::skills::SkillScope::Local,
        enabled: true,
        plugin_name: None,
        ignore_paths: vec![],
    }
}

#[test]
fn skills_lists_loaded() {
    let mut state = AppState { skills: vec![rust_skill(false)], ..Default::default() };
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
    let mut state = AppState { skills: vec![rust_skill(true)], ..Default::default() };
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
    let mut state = AppState { skills: vec![rust_skill(true)], ..Default::default() };
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
    let mut state = AppState { skills: vec![rust_skill(true)], ..Default::default() };
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

// ── CRUD slash commands ─────────────────────────────────────────────────────

/// Serializes env-dependent tests (HOME pointing at a temp dir).
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn state_with_home(dir: &tempfile::TempDir) -> AppState {
    std::env::set_var("HOME", dir.path());
    AppState::default()
}

#[test]
fn create_skill_writes_file_and_reloads() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::TempDir::new().unwrap();
    let mut state = state_with_home(&tmp);

    let result = exec_handler(&mut state, "create-skill", "my-new-skill");
    match result {
        CommandResult::Message(msg) => {
            assert!(msg.contains("Created skill 'my-new-skill'"), "got: {msg}");
        }
        other => panic!("/create-skill should return Message, got {other:?}"),
    }

    let skill_file = tmp.path().join(".runie/skills/my-new-skill.md");
    assert!(skill_file.exists(), "skill file should be created");
    assert!(
        state.skills().iter().any(|s| s.name == "my-new-skill"),
        "state should be reloaded with the new skill"
    );
}

#[test]
fn create_skill_rejects_empty_and_duplicate() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::TempDir::new().unwrap();
    let mut state = state_with_home(&tmp);

    let empty = exec_handler(&mut state, "create-skill", "");
    assert!(
        matches!(&empty, CommandResult::Warning(_)),
        "empty name should warn, got {empty:?}"
    );

    exec_handler(&mut state, "create-skill", "dup");
    let dup = exec_handler(&mut state, "create-skill", "dup");
    assert!(
        matches!(&dup, CommandResult::Warning(msg) if msg.contains("already exists")),
        "duplicate should warn, got {dup:?}"
    );
}

#[test]
fn delete_skill_removes_file_and_reloads() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::TempDir::new().unwrap();
    let mut state = state_with_home(&tmp);

    exec_handler(&mut state, "create-skill", "to-delete");
    assert!(state.skills().iter().any(|s| s.name == "to-delete"));

    let result = exec_handler(&mut state, "delete-skill", "to-delete");
    match result {
        CommandResult::Message(msg) => {
            assert!(msg.contains("Deleted skill 'to-delete'"), "got: {msg}");
        }
        other => panic!("/delete-skill should return Message, got {other:?}"),
    }

    let skill_file = tmp.path().join(".runie/skills/to-delete.md");
    assert!(!skill_file.exists(), "skill file should be removed");
    assert!(
        !state.skills().iter().any(|s| s.name == "to-delete"),
        "state should be reloaded without the deleted skill"
    );
}

#[test]
fn delete_unknown_skill_warns() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::TempDir::new().unwrap();
    let mut state = state_with_home(&tmp);

    let result = exec_handler(&mut state, "delete-skill", "missing");
    assert!(
        matches!(&result, CommandResult::Warning(msg) if msg.contains("not found")),
        "unknown delete should warn, got {result:?}"
    );
}

#[test]
fn reload_skills_refreshes_list() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::TempDir::new().unwrap();
    let mut state = state_with_home(&tmp);

    let result = exec_handler(&mut state, "reload-skills", "");
    match result {
        CommandResult::Message(msg) => {
            assert!(
                msg.contains("none loaded"),
                "no skills should report 'none loaded', got: {msg}"
            );
        }
        other => panic!("/reload-skills should return Message, got {other:?}"),
    }

    // Create a skill on disk and reload: it must appear in state.
    let dir = tmp.path().join(".runie/skills");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("external.md"),
        "---\nname: external\n---\n# External\n",
    )
    .unwrap();

    let result = exec_handler(&mut state, "reload-skills", "");
    assert!(
        matches!(&result, CommandResult::Message(msg) if msg.contains("1")),
        "one skill should be reported, got {result:?}"
    );
    assert!(
        state.skills().iter().any(|s| s.name == "external"),
        "reload should pick up the on-disk skill"
    );
}
