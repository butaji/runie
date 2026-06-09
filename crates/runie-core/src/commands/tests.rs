use super::*;

#[test]
fn registry_get_by_name() {
    let state = AppState::default();
    let cmd = state.registry.get("model");
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().name, "model");
}

#[test]
fn registry_get_by_alias() {
    let state = AppState::default();
    let cmd = state.registry.get("m");
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().name, "model");
}

#[test]
fn registry_list_returns_all() {
    let state = AppState::default();
    let defs = state.registry.list();
    assert!(
        defs.len() >= 22,
        "registry should have 22+ commands, got {}",
        defs.len()
    );
}

#[test]
fn registry_list_groups_by_category() {
    let state = AppState::default();
    let groups = state.registry.list_by_category();
    assert!(!groups.is_empty());
    let total: usize = groups.iter().map(|g| g.1.len()).sum();
    assert!(total >= 22);
}

#[test]
fn handler_model_switches() {
    let mut state = AppState::default();
    let cmd = state.registry.get("model").unwrap();
    let result = (cmd.handler)(&mut state, "gpt-4o");
    assert_eq!(state.current_model, "gpt-4o");
    assert!(matches!(result, CommandResult::Message(_)));
}

#[test]
fn handler_help_generates_list() {
    let mut state = AppState::default();
    let cmd = state.registry.get("help").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("Commands:"));
        assert!(msg.contains("/model"));
        assert!(msg.contains("/save"));
    } else {
        panic!("help should return Message, got {:?}", result);
    }
}

#[test]
fn handler_quit_sets_flag() {
    let mut state = AppState::default();
    let cmd = state.registry.get("quit").unwrap();
    let result = (cmd.handler)(&mut state, "");
    assert!(matches!(result, CommandResult::Event(Event::Quit)));
    state.update(Event::Quit);
    assert!(state.should_quit);
}

#[test]
fn unknown_command_returns_error() {
    let mut state = AppState::default();
    let result = state.handle_slash("/foo");
    assert!(matches!(result, Some(CommandResult::Message(msg)) if msg.contains("Unknown command")));
}

#[test]
fn slash_event_dispatches_to_registry() {
    let mut state = AppState::default();
    type_str(&mut state, "/model gpt-4o");
    state.update(Event::Submit);
    assert_eq!(state.current_model, "gpt-4o");
}

#[test]
fn alias_event_dispatches_correctly() {
    let mut state = AppState::default();
    type_str(&mut state, "/m gpt-4o");
    state.update(Event::Submit);
    assert_eq!(state.current_model, "gpt-4o");
}

// Palette filter tests (Layer 1)

#[test]
fn filter_empty_shows_all() {
    let state = AppState::default();
    let all = state.registry.list();
    let filtered = filter_commands(&state.registry, "");
    assert_eq!(filtered.len(), all.len());
}

#[test]
fn filter_matches_name() {
    let state = AppState::default();
    let filtered = filter_commands(&state.registry, "comp");
    assert!(
        filtered.iter().any(|c| c.name == "compact"),
        "'comp' should match 'compact'"
    );
}

#[test]
fn filter_matches_description() {
    let state = AppState::default();
    let filtered = filter_commands(&state.registry, "copy");
    assert!(
        filtered.iter().any(|c| c.name == "copy"),
        "'copy' should match 'copy' command description"
    );
}

#[test]
fn filter_case_insensitive() {
    let state = AppState::default();
    let lower = filter_commands(&state.registry, "comp");
    let upper = filter_commands(&state.registry, "COMP");
    assert_eq!(lower.len(), upper.len());
    assert!(upper.iter().any(|c| c.name == "compact"));
}

#[test]
fn select_wraps_up() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    // Up at first should wrap to last
    state.update(Event::PaletteUp);
    let count = filter_commands(&state.registry, "").len();
    if let Some(DialogState::CommandPalette { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, count - 1, "Up at first should wrap to last");
    } else {
        panic!("Palette should be open");
    }
}

#[test]
fn select_wraps_down() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let count = filter_commands(&state.registry, "").len();
    // Down at last should wrap to first
    for _ in 0..count {
        state.update(Event::PaletteDown);
    }
    if let Some(DialogState::CommandPalette { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, 0, "Down at last should wrap to first");
    } else {
        panic!("Palette should be open");
    }
}

fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

// Skills tests (Layer 1 + Layer 2)

#[test]
fn skills_lists_loaded() {
    let mut state = AppState::default();
    state.skills = vec![
        crate::skills::Skill {
            name: "rust".into(),
            description: "Rust best practices".into(),
            context: "Use clippy".into(),
            user_invocable: false,
            file_path: std::path::PathBuf::from("rust.md"),
        }
    ];
    let cmd = state.registry.get("skills").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("rust"), "Should list skill name, got: {}", msg);
        assert!(msg.contains("Rust best practices"), "Should list skill description, got: {}", msg);
    } else {
        panic!("/skills should return Message, got {:?}", result);
    }
}

#[test]
fn skills_empty_shows_no_skills() {
    let mut state = AppState::default();
    let cmd = state.registry.get("skills").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("No skills loaded"), "got: {}", msg);
    } else {
        panic!("/skills should return Message, got {:?}", result);
    }
}

#[test]
fn skill_shows_info() {
    let mut state = AppState::default();
    state.skills = vec![
        crate::skills::Skill {
            name: "rust".into(),
            description: "Rust best practices".into(),
            context: "Use clippy".into(),
            user_invocable: true,
            file_path: std::path::PathBuf::from("rust.md"),
        }
    ];
    let cmd = state.registry.get("skill").unwrap();
    let result = (cmd.handler)(&mut state, "rust");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("rust"), "Should show skill name, got: {}", msg);
        assert!(msg.contains("Use clippy"), "Should show skill context, got: {}", msg);
    } else {
        panic!("/skill rust should return Message, got {:?}", result);
    }
}

#[test]
fn skill_unknown_returns_error() {
    let mut state = AppState::default();
    let cmd = state.registry.get("skill").unwrap();
    let result = (cmd.handler)(&mut state, "unknown");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("not found"), "Should report unknown skill, got: {}", msg);
    } else {
        panic!("/skill unknown should return error Message, got {:?}", result);
    }
}

#[test]
fn palette_shows_user_invocable_skills() {
    let mut state = AppState::default();
    state.skills = vec![
        crate::skills::Skill {
            name: "rust".into(),
            description: "Rust best practices".into(),
            context: "Use clippy".into(),
            user_invocable: true,
            file_path: std::path::PathBuf::from("rust.md"),
        }
    ];
    state.update(Event::ToggleCommandPalette);
    let snap = state.snapshot();
    assert!(
        snap.palette_items.iter().any(|(n, _, c)| n == "rust" && c == "Skill"),
        "User-invocable skill should appear in palette items: {:?}",
        snap.palette_items
    );
}

#[test]
fn palette_select_skill_emits_message() {
    let mut state = AppState::default();
    state.skills = vec![
        crate::skills::Skill {
            name: "rust".into(),
            description: "Rust best practices".into(),
            context: "Use clippy".into(),
            user_invocable: true,
            file_path: std::path::PathBuf::from("rust.md"),
        }
    ];
    // Open palette and find skill position
    state.update(Event::ToggleCommandPalette);
    let snap = state.snapshot();
    let skill_pos = snap.palette_items.iter().position(|(n, _, c)| n == "rust" && c == "Skill").expect("skill should be in palette");
    // Select it
    for _ in 0..skill_pos {
        state.update(Event::PaletteDown);
    }
    state.update(Event::PaletteSelect);
    let last = state.messages.last().expect("should have a message");
    assert!(last.content.contains("rust"), "Selecting skill should emit info message: {}", last.content);
}

// Prompt tests (Layer 1 + Layer 2)

#[test]
fn prompt_switch_updates() {
    let mut state = AppState::default();
    state.prompts = vec![
        crate::prompts::PromptTemplate {
            name: "custom".into(),
            content: "Be concise.".into(),
            source: crate::prompts::PromptSource::BuiltIn,
        }
    ];
    let cmd = state.registry.get("prompt").unwrap();
    let result = (cmd.handler)(&mut state, "custom");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("custom"), "Should confirm prompt switch: {}", msg);
    } else {
        panic!("/prompt custom should return Message, got {:?}", result);
    }
    assert_eq!(state.current_prompt, "custom");
}

#[test]
fn prompt_shows_current_when_no_args() {
    let mut state = AppState::default();
    state.prompts = vec![
        crate::prompts::PromptTemplate {
            name: "default".into(),
            content: "Be helpful.".into(),
            source: crate::prompts::PromptSource::BuiltIn,
        }
    ];
    let cmd = state.registry.get("prompt").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("default"), "Should show current prompt: {}", msg);
    } else {
        panic!("/prompt should return Message, got {:?}", result);
    }
}

#[test]
fn prompt_unknown_returns_error() {
    let mut state = AppState::default();
    let cmd = state.registry.get("prompt").unwrap();
    let result = (cmd.handler)(&mut state, "unknown");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("not found"), "Should report unknown prompt: {}", msg);
    } else {
        panic!("/prompt unknown should return error Message, got {:?}", result);
    }
}

#[test]
fn session_info_shows_prompt() {
    let mut state = AppState::default();
    state.current_prompt = "custom".into();
    let cmd = state.registry.get("session").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("Prompt: custom"), "Session should show prompt: {}", msg);
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

// Session info tests (Layer 1)

#[test]
fn session_info_counts_messages() {
    let mut state = AppState::default();
    state.messages = vec![
        crate::model::ChatMessage { role: crate::model::Role::User, content: "hi".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()},
        crate::model::ChatMessage { role: crate::model::Role::Assistant, content: "hello".into(), timestamp: 0.0, id: "a1".into(), ..Default::default()},
        crate::model::ChatMessage { role: crate::model::Role::Tool, content: "tool out".into(), timestamp: 0.0, id: "t1".into(), ..Default::default()},
        crate::model::ChatMessage { role: crate::model::Role::User, content: "again".into(), timestamp: 0.0, id: "u2".into(), ..Default::default()},
    ];
    let cmd = state.registry.get("session").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("Messages: 4 total (2 user, 1 assistant, 1 tool)"), "got: {}", msg);
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn session_info_shows_tokens() {
    let mut state = AppState::default();
    state.messages = vec![
        crate::model::ChatMessage { role: crate::model::Role::User, content: "hello world".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()},
    ];
    let cmd = state.registry.get("session").unwrap();
    let result = (cmd.handler)(&mut state, "");
    if let CommandResult::Message(msg) = result {
        assert!(msg.contains("Tokens:"), "Token estimate should be present, got: {}", msg);
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn slash_session_dispatches() {
    let mut state = AppState::default();
    state.messages.push(crate::model::ChatMessage { role: crate::model::Role::User, content: "test".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()});
    type_str(&mut state, "/session");
    state.update(Event::Submit);
    let last = state.messages.last().unwrap();
    assert_eq!(last.role, crate::model::Role::System);
    assert!(last.content.contains("Messages:"));
}
