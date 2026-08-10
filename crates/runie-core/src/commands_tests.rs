use super::*;

#[test]
fn registry_matches_pi_builtin_count_and_order() {
    assert_eq!(PI_BUILTIN_SLASH_COMMANDS.len(), 22);
    assert_eq!(PI_BUILTIN_SLASH_COMMANDS.first().unwrap().name, "settings");
    assert_eq!(PI_BUILTIN_SLASH_COMMANDS.last().unwrap().name, "quit");
    assert!(matches!(
        PI_BUILTIN_SLASH_COMMANDS[1].argument_hint,
        Some("<provider/model>")
    ));
}

#[test]
fn registry_filter_is_pure_and_case_insensitive() {
    let commands = matching_pi_builtin_slash_commands("  MODEL ");
    assert_eq!(
        commands
            .into_iter()
            .map(|command| command.name)
            .collect::<Vec<_>>(),
        vec!["model", "scoped-models"]
    );
    assert_eq!(
        matching_pi_builtin_slash_commands("authentication")
            .into_iter()
            .map(|command| command.name)
            .collect::<Vec<_>>(),
        vec!["login", "logout"]
    );
}

#[test]
fn mappable_parser_rejects_unimplemented_commands_without_swallowing_text() {
    assert_mappable_basics();
    assert_eq!(parse_mappable_builtin_command("/quit now"), None);
    assert_eq!(
        parse_mappable_builtin_command("/scoped-models"),
        Some(MappableBuiltinCommand::ScopedModels)
    );
    assert_eq!(
        parse_mappable_builtin_command("/model openai/gpt-5"),
        Some(MappableBuiltinCommand::Model {
            reference: "openai/gpt-5".into()
        })
    );
    assert_eq!(parse_mappable_builtin_command("/model"), None);
    assert_eq!(
        parse_mappable_builtin_command("/jobs"),
        Some(MappableBuiltinCommand::Extended {
            name: "jobs".into(),
            args: String::new()
        })
    );
}

fn assert_mappable_basics() {
    assert_eq!(
        parse_mappable_builtin_command(" /new "),
        Some(MappableBuiltinCommand::NewSession)
    );
    assert_eq!(
        parse_mappable_builtin_command("/changelog"),
        Some(MappableBuiltinCommand::Changelog)
    );
    assert_eq!(
        parse_mappable_builtin_command("/compact"),
        Some(MappableBuiltinCommand::Compact { instructions: None })
    );
    assert_eq!(
        parse_mappable_builtin_command("/compact preserve the latest user intent"),
        Some(MappableBuiltinCommand::Compact {
            instructions: Some("preserve the latest user intent".into())
        })
    );
    assert_eq!(
        parse_mappable_builtin_command("/copy"),
        Some(MappableBuiltinCommand::Copy)
    );
    assert_eq!(
        parse_mappable_builtin_command("/undo"),
        Some(MappableBuiltinCommand::Extended {
            name: "undo".into(),
            args: String::new()
        })
    );
    assert_eq!(
        parse_mappable_builtin_command("/name release parity"),
        Some(MappableBuiltinCommand::Name {
            name: "release parity".into()
        })
    );
}

#[test]
fn classifier_exposes_known_unsupported_capabilities() {
    assert_classifier_mappable_paths();
    assert_classifier_dispositions();
}

fn assert_classifier_mappable_paths() {
    assert_eq!(
        classify_builtin_command("/export session.jsonl"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Export {
            path: "session.jsonl".into()
        })
    );
    assert_eq!(
        classify_builtin_command("/import session.jsonl"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Import {
            path: "session.jsonl".into()
        })
    );
    assert_eq!(
        classify_builtin_command("/clone copy.jsonl"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Clone {
            path: "copy.jsonl".into()
        })
    );
    assert_eq!(
        classify_builtin_command("/resume saved.jsonl"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Resume {
            path: "saved.jsonl".into()
        })
    );
}

fn assert_classifier_dispositions() {
    assert!(matches!(
        classify_builtin_command("/login openai"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Extended { .. })
    ));
    assert!(matches!(
        classify_builtin_command("/name release"),
        BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Name { .. })
    ));
    assert_eq!(
        classify_builtin_command("/not-a-pi-command"),
        BuiltinCommandDisposition::NotBuiltin
    );
    assert_eq!(
        classify_builtin_command("ordinary prompt"),
        BuiltinCommandDisposition::NotBuiltin
    );
}

#[test]
fn formerly_unsupported_pi_commands_are_now_typed_and_mappable() {
    for input in [
        "/settings",
        "/share",
        "/trust",
        "/login anthropic",
        "/logout",
        "/reload",
    ] {
        assert!(matches!(
            classify_builtin_command(input),
            BuiltinCommandDisposition::Mappable(MappableBuiltinCommand::Extended {
                name: _,
                args: _
            })
        ));
    }
}

#[test]
fn every_registry_name_has_a_typed_malformed_input_classification() {
    let cases = [
        ("settings", UnsupportedBuiltinCommand::Settings),
        ("model", UnsupportedBuiltinCommand::Model),
        ("scoped-models", UnsupportedBuiltinCommand::ScopedModels),
        ("export", UnsupportedBuiltinCommand::Export),
        ("import", UnsupportedBuiltinCommand::Import),
        ("share", UnsupportedBuiltinCommand::Share),
        ("copy", UnsupportedBuiltinCommand::Copy),
        ("name", UnsupportedBuiltinCommand::Name),
        ("session", UnsupportedBuiltinCommand::Session),
        ("changelog", UnsupportedBuiltinCommand::Changelog),
        ("hotkeys", UnsupportedBuiltinCommand::Hotkeys),
        ("fork", UnsupportedBuiltinCommand::Fork),
        ("clone", UnsupportedBuiltinCommand::Clone),
        ("tree", UnsupportedBuiltinCommand::Tree),
        ("trust", UnsupportedBuiltinCommand::Trust),
        ("login", UnsupportedBuiltinCommand::Login),
        ("logout", UnsupportedBuiltinCommand::Logout),
        ("new", UnsupportedBuiltinCommand::New),
        ("compact", UnsupportedBuiltinCommand::Compact),
        ("resume", UnsupportedBuiltinCommand::Resume),
        ("reload", UnsupportedBuiltinCommand::Reload),
        ("quit", UnsupportedBuiltinCommand::Quit),
    ];
    assert_eq!(cases.len(), PI_BUILTIN_SLASH_COMMANDS.len());
    for (name, expected) in cases {
        let disposition = classify_builtin_command(&format!("/{name} invalid"));
        assert_eq!(expected.name(), name);
        assert!(!matches!(
            disposition,
            BuiltinCommandDisposition::NotBuiltin
        ));
    }
}
