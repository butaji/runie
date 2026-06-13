use super::*;
use crate::model::AppState;
use crate::Event;

/// Type a slash command directly into the input and submit it.
/// Bypasses the `/` → command-palette shortcut so tests can exercise
/// the slash-command dispatcher itself.
fn run_slash(state: &mut AppState, text: &str) {
    state.input.input = text.to_string();
    state.input.cursor_pos = text.len();
    state.update(Event::Submit);
}

/// Execute the handler function inside a command's flow, ignoring any
/// `.sub()` wrapper that would otherwise push the current dialog onto
/// the back stack.
fn exec_handler(state: &mut AppState, name: &str, args: &str) -> CommandResult {
    let cmd = state.registry.get(name).unwrap();
    match &cmd.flow {
        CommandFlow::Handler(f) => f(state, args),
        CommandFlow::Sub(inner) => match inner.as_ref() {
            CommandFlow::Handler(f) => f(state, args),
            _ => panic!("command {name} is not a handler"),
        },
        _ => panic!("command {name} is not a handler"),
    }
}

fn palette_stack(state: &AppState) -> Option<&crate::dialog::PanelStack> {
    match &state.open_dialog {
        Some(DialogState::CommandPalette(stack)) => Some(&stack),
        _ => None,
    }
}

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
fn registry_get_providers_alias() {
    let state = AppState::default();
    let providers = state.registry.get("providers");
    let provider = state.registry.get("provider");
    assert!(providers.is_some());
    assert_eq!(providers.unwrap().name, "providers");
    assert_eq!(provider.unwrap().name, "providers");
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
    let result = exec_handler(&mut state, "model", "gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(matches!(result, CommandResult::Message(_)));
}

#[test]
fn handler_help_generates_list() {
    let mut state = AppState::default();
    let result = exec_handler(&mut state, "help", "");
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
    let result = exec_handler(&mut state, "quit", "");
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
    run_slash(&mut state, "/model gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn alias_event_dispatches_correctly() {
    let mut state = AppState::default();
    run_slash(&mut state, "/m gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn provider_alias_dispatches_to_same_command() {
    let mut state = AppState::default();
    run_slash(&mut state, "/provider");
    assert!(
        state.open_dialog.is_some(),
        "/provider should open providers dialog"
    );

    let mut state = AppState::default();
    run_slash(&mut state, "/providers");
    assert!(
        state.open_dialog.is_some(),
        "/providers should open providers dialog"
    );
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
    state.update(Event::PaletteUp);
    let count = filter_commands(&state.registry, "").len();
    let stack = palette_stack(&state).expect("Palette should be open");
    assert_eq!(
        stack.current().unwrap().selected,
        count - 1,
        "Up at first should wrap to last"
    );
}

#[test]
fn select_wraps_down() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let count = filter_commands(&state.registry, "").len();
    for _ in 0..count {
        state.update(Event::PaletteDown);
    }
    let stack = palette_stack(&state).expect("Palette should be open");
    assert_eq!(
        stack.current().unwrap().selected,
        0,
        "Down at last should wrap to first"
    );
}

// Skills tests (Layer 1 + Layer 2)

#[test]
fn skills_lists_loaded() {
    let mut state = AppState::default();
    state.skills = vec![crate::skills::Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable: false,
        file_path: std::path::PathBuf::from("rust.md"),
    }];
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
    state.transient_message = None;
    state.transient_level = None;
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
    let mut state = AppState::default();
    state.skills = vec![crate::skills::Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable: true,
        file_path: std::path::PathBuf::from("rust.md"),
    }];
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
    let mut state = AppState::default();
    state.skills = vec![crate::skills::Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable: true,
        file_path: std::path::PathBuf::from("rust.md"),
    }];
    state.update(Event::ToggleCommandPalette);
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
    let mut state = AppState::default();
    state.skills = vec![crate::skills::Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy".into(),
        user_invocable: true,
        file_path: std::path::PathBuf::from("rust.md"),
    }];
    state.update(Event::ToggleCommandPalette);
    let snap = state.snapshot();
    let skill_pos = snap
        .palette_items
        .iter()
        .position(|(n, _, c)| n == "rust" && c == "Skill")
        .expect("skill should be in palette");
    for _ in 0..skill_pos {
        state.update(Event::PaletteDown);
    }
    state.update(Event::PaletteSelect);
    let last = state
        .session
        .messages
        .last()
        .expect("should have a message");
    assert!(
        last.content.contains("rust"),
        "Selecting skill should emit info message: {}",
        last.content
    );
}

// Prompt tests (Layer 1 + Layer 2)
// /prompt is a form command, so the real switch happens via the
// RunPromptCommand event after the form is submitted.

#[test]
fn prompt_switch_updates() {
    let mut state = AppState::default();
    state.prompts = vec![crate::prompts::PromptTemplate {
        name: "custom".into(),
        content: "Be concise.".into(),
        source: crate::prompts::PromptSource::BuiltIn,
    }];
    state.update(Event::RunPromptCommand {
        name: "custom".into(),
    });
    assert_eq!(state.input.current_prompt, "custom");
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content.contains("custom"));
}

#[test]
fn prompt_shows_current_when_no_args() {
    let mut state = AppState::default();
    state.prompts = vec![crate::prompts::PromptTemplate {
        name: "default".into(),
        content: "Be helpful.".into(),
        source: crate::prompts::PromptSource::BuiltIn,
    }];
    state.update(Event::RunPromptCommand { name: "".into() });
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content.contains("default"), "got: {}", last.content);
}

#[test]
fn prompt_unknown_returns_error() {
    let mut state = AppState::default();
    state.update(Event::RunPromptCommand {
        name: "unknown".into(),
    });
    let last = state.session.messages.last().expect("should have message");
    assert!(last.content.contains("not found"), "got: {}", last.content);
}

// Session info tests (Layer 1)

#[test]
fn session_info_counts_messages() {
    let mut state = AppState::default();
    state.session.messages = vec![
        crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: "hi".into(),
            timestamp: 0.0,
            id: "u1".into(),
            ..Default::default()
        },
        crate::model::ChatMessage {
            role: crate::model::Role::Assistant,
            content: "hello".into(),
            timestamp: 0.0,
            id: "a1".into(),
            ..Default::default()
        },
        crate::model::ChatMessage {
            role: crate::model::Role::Tool,
            content: "tool out".into(),
            timestamp: 0.0,
            id: "t1".into(),
            ..Default::default()
        },
        crate::model::ChatMessage {
            role: crate::model::Role::User,
            content: "again".into(),
            timestamp: 0.0,
            id: "u2".into(),
            ..Default::default()
        },
    ];
    let result = exec_handler(&mut state, "session", "");
    if let CommandResult::Message(msg) = result {
        assert!(
            msg.contains("Messages: 4 (2 user, 1 assistant, 1 tool)"),
            "got: {}",
            msg
        );
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn session_info_shows_tokens() {
    let mut state = AppState::default();
    state.session.messages = vec![crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "hello world".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    }];
    let result = exec_handler(&mut state, "session", "");
    if let CommandResult::Message(msg) = result {
        assert!(
            msg.contains("Tokens:"),
            "Token estimate should be present, got: {}",
            msg
        );
    } else {
        panic!("session should return Message, got {:?}", result);
    }
}

#[test]
fn slash_session_dispatches() {
    let mut state = AppState::default();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "test".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    run_slash(&mut state, "/session");
    let last = state.session.messages.last().unwrap();
    assert_eq!(last.role, crate::model::Role::System);
    assert!(last.content.contains("Messages:"));
}
