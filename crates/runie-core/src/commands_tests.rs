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
    assert_eq!(
        parse_mappable_builtin_command("/sessions"),
        Some(MappableBuiltinCommand::Extended {
            name: "sessions".into(),
            args: String::new()
        })
    );
    assert_eq!(
        parse_mappable_builtin_command("/questions deploy"),
        Some(MappableBuiltinCommand::Extended {
            name: "questions".into(),
            args: "deploy".into()
        })
    );
}

#[test]
fn parser_maps_git_conflict_report_as_an_extended_command() {
    assert_eq!(
        parse_mappable_builtin_command("/git conflicts"),
        Some(MappableBuiltinCommand::Extended {
            name: "git".into(),
            args: "conflicts".into()
        })
    );
}

#[test]
fn parser_maps_effort_without_an_argument_for_picker_reopen() {
    assert_eq!(
        parse_mappable_builtin_command("/effort"),
        Some(MappableBuiltinCommand::Extended {
            name: "effort".into(),
            args: String::new()
        })
    );
}

#[test]
fn parser_maps_clear_and_reset_to_context_commands() {
    for name in ["clear", "reset"] {
        assert_eq!(
            parse_mappable_builtin_command(&format!("/{name}")),
            Some(MappableBuiltinCommand::Extended {
                name: name.into(),
                args: String::new(),
            })
        );
    }
}

#[test]
fn parser_keeps_question_history_actions_as_extended_commands() {
    assert_eq!(
        parse_mappable_builtin_command("/questions clear"),
        Some(MappableBuiltinCommand::Extended {
            name: "questions".into(),
            args: "clear".into(),
        })
    );
}

#[test]
fn parser_keeps_context_compaction_as_a_data_command() {
    assert_eq!(
        parse_mappable_builtin_command("/context compact keep the latest turn"),
        Some(MappableBuiltinCommand::Extended {
            name: "context".into(),
            args: "compact keep the latest turn".into(),
        })
    );
}

#[test]
fn parser_maps_session_text_as_a_query_argument() {
    assert_eq!(
        parse_mappable_builtin_command("/sessions deploy"),
        Some(MappableBuiltinCommand::Extended {
            name: "sessions".into(),
            args: "deploy".into()
        })
    );
}

#[test]
fn parser_maps_resume_without_an_argument_to_the_session_picker() {
    assert_eq!(
        parse_mappable_builtin_command("/resume"),
        Some(MappableBuiltinCommand::Extended {
            name: "resume".into(),
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
#[allow(
    clippy::cognitive_complexity,
    reason = "This is one declarative parser acceptance matrix"
)]
fn background_job_command_accepts_only_cancel_and_one_id() {
    assert_eq!(parse_background_job_command("cancel 7"), Some("7"));
    assert_eq!(parse_background_job_command("cancel 7 extra"), None);
    assert_eq!(parse_background_job_command("list 7"), None);
    assert_eq!(parse_background_job_query("7"), Some("7"));
    assert_eq!(parse_background_job_query("cancel 7"), None);
    assert_eq!(parse_background_job_query("7 extra"), None);
    assert!(parse_background_job_cancel_all("cancel all"));
    assert!(!parse_background_job_cancel_all("cancel all extra"));
    assert!(parse_background_scheduler_query("scheduler"));
    assert!(!parse_background_scheduler_query("scheduler extra"));
    assert!(parse_background_scheduler_active_query("scheduler active"));
    assert!(!parse_background_scheduler_active_query("scheduler"));
    assert_eq!(
        parse_background_job_status_query("running"),
        Some("running")
    );
    assert_eq!(parse_background_job_status_query("failed"), Some("failed"));
    assert_eq!(parse_background_job_status_query("queued"), None);
    assert_eq!(parse_mcp_transport_query("http"), Some("http"));
    assert_eq!(parse_mcp_transport_query("websocket"), None);
    assert_eq!(parse_mcp_status_query("ready"), Some("ready"));
    assert_eq!(parse_mcp_status_query("closed"), Some("closed"));
    assert_eq!(parse_mcp_status_query("unknown"), None);
    assert_eq!(parse_session_picker_query("pick"), Some("".into()));
    assert_eq!(
        parse_session_picker_query("pick deploy"),
        Some("deploy".into())
    );
    assert_eq!(parse_session_picker_query("browse deploy"), None);
}

#[test]
fn background_job_output_query_requires_one_id() {
    assert_eq!(parse_background_job_output_query("output 7"), Some("7"));
    assert_eq!(parse_background_job_output_query("output"), None);
    assert_eq!(parse_background_job_output_query("output 7 extra"), None);
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
