//! Pi's built-in slash-command contract.
//!
//! This is a declarative capability registry, not an execution path.  An
//! owning actor may later interpret a command, but the command vocabulary and
//! presentation metadata have one source of truth here.

use serde::Serialize;

/// A command exposed by Pi's interactive command registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub argument_hint: Option<&'static str>,
}

macro_rules! pi_slash_commands {
    ($($name:literal => $description:literal $(, $hint:literal)?);+ $(;)?) => {
pub const PI_BUILTIN_SLASH_COMMANDS: &[SlashCommand] = &[
            $(SlashCommand {
                name: $name,
                description: $description,
                argument_hint: pi_slash_commands!(@hint $($hint)?),
            }),+
];

    };
    (@hint $hint:literal) => { Some($hint) };
    (@hint) => { None };
}

// Source: pi/packages/coding-agent/src/core/slash-commands.ts
pi_slash_commands! {
    "settings" => "Open settings menu";
    "model" => "Select model (opens selector UI)", "<provider/model>";
    "scoped-models" => "Enable/disable models for Ctrl+P cycling";
    "export" => "Export session (HTML default, or specify path: .html/.jsonl)";
    "import" => "Import and resume a session from a JSONL file";
    "share" => "Share session as a secret GitHub gist";
    "copy" => "Copy last agent message to clipboard";
    "name" => "Set session display name";
    "session" => "Show session info and stats";
    "changelog" => "Show changelog entries";
    "hotkeys" => "Show all keyboard shortcuts";
    "fork" => "Create a new fork from a previous user message";
    "clone" => "Duplicate the current session at the current position";
    "tree" => "Navigate session tree (switch branches)";
    "trust" => "Save project trust decision for future sessions";
    "login" => "Configure provider authentication", "<provider>";
    "logout" => "Remove provider authentication";
    "new" => "Start a new session";
    "compact" => "Manually compact the session context";
    "resume" => "Resume a different session";
    "reload" => "Reload keybindings, extensions, skills, prompts, themes, and context files";
    "quit" => "Quit Pi"
}

/// Return the source-defined registry, optionally filtered by user input.
/// Matching is case-insensitive and checks both command names and descriptions,
/// which mirrors the interactive palette's search contract without owning UI
/// state.
pub fn matching_pi_builtin_slash_commands(query: &str) -> Vec<SlashCommand> {
    let query = query.trim().to_ascii_lowercase();
    PI_BUILTIN_SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|command| {
            query.is_empty()
                || command.name.contains(&query)
                || command.description.to_ascii_lowercase().contains(&query)
        })
        .collect()
}

/// The subset whose effects already have an owning Runie actor boundary.
/// Keeping this separate from the complete registry prevents unsupported Pi
/// commands from being reported as successful no-ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappableBuiltinCommand {
    Changelog,
    NewSession,
    Hotkeys,
    Copy,
    Quit,
    Model {
        reference: String,
    },
    ScopedModels,
    SessionInfo,
    Name {
        name: String,
    },
    Compact {
        instructions: Option<String>,
    },
    Fork {
        target_id: String,
    },
    Tree {
        target_id: String,
    },
    Export {
        path: String,
    },
    Import {
        path: String,
    },
    Clone {
        path: String,
    },
    Resume {
        path: String,
    },
    /// A Runie/Grok command whose execution is delegated through the normal
    /// prompt actor until a dedicated owning actor exists. Keeping the
    /// invocation typed ensures palette and pasted-input paths agree.
    Extended {
        name: String,
        args: String,
    },
}

/// A known Pi command whose provider/UI capability has no Runie owner yet.
/// Keeping this closed prevents unsupported commands from becoming an
/// accidental successful no-op when the registry grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedBuiltinCommand {
    Settings,
    Model,
    ScopedModels,
    Export,
    Import,
    Share,
    Copy,
    Name,
    Session,
    Changelog,
    Hotkeys,
    Trust,
    Login,
    Logout,
    New,
    Compact,
    Reload,
    Fork,
    Clone,
    Tree,
    Resume,
    Quit,
}

impl UnsupportedBuiltinCommand {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Model => "model",
            Self::ScopedModels => "scoped-models",
            Self::Export => "export",
            Self::Import => "import",
            Self::Share => "share",
            Self::Copy => "copy",
            Self::Name => "name",
            Self::Session => "session",
            Self::Changelog => "changelog",
            Self::Hotkeys => "hotkeys",
            Self::Trust => "trust",
            Self::Login => "login",
            Self::Logout => "logout",
            Self::New => "new",
            Self::Compact => "compact",
            Self::Reload => "reload",
            Self::Fork => "fork",
            Self::Clone => "clone",
            Self::Tree => "tree",
            Self::Resume => "resume",
            Self::Quit => "quit",
        }
    }
}

/// Classification at the slash-command boundary. Unsupported Pi commands are
/// observable capabilities rather than silent successful no-ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinCommandDisposition {
    Mappable(MappableBuiltinCommand),
    Unsupported(UnsupportedBuiltinCommand),
    NotBuiltin,
}

/// Parse an exact Pi built-in command that Runie can currently route through
/// an existing actor/application boundary. Non-mappable commands return
/// `None` and remain ordinary prompt text until their capability is built.
pub fn parse_mappable_builtin_command(input: &str) -> Option<MappableBuiltinCommand> {
    match input.trim() {
        "/changelog" => Some(MappableBuiltinCommand::Changelog),
        "/new" => Some(MappableBuiltinCommand::NewSession),
        "/hotkeys" => Some(MappableBuiltinCommand::Hotkeys),
        "/copy" => Some(MappableBuiltinCommand::Copy),
        "/quit" => Some(MappableBuiltinCommand::Quit),
        "/scoped-models" => Some(MappableBuiltinCommand::ScopedModels),
        "/session" => Some(MappableBuiltinCommand::SessionInfo),
        "/compact" => Some(MappableBuiltinCommand::Compact { instructions: None }),
        value => parse_extended_no_arg(value).or_else(|| parse_parameterized_command(value)),
    }
}

#[allow(clippy::too_many_lines)]
fn parse_parameterized_command(value: &str) -> Option<MappableBuiltinCommand> {
    if let Some(instructions) = value.strip_prefix("/compact ") {
        return Some(MappableBuiltinCommand::Compact {
            instructions: (!instructions.trim().is_empty()).then(|| instructions.trim().to_owned()),
        });
    }
    let (prefix, text) = value.split_once(' ')?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    match prefix {
        "/name" => Some(MappableBuiltinCommand::Name { name: text.into() }),
        "/model"
            if text
                .split_once('/')
                .is_some_and(|(p, m)| !p.is_empty() && !m.is_empty()) =>
        {
            Some(MappableBuiltinCommand::Model {
                reference: text.into(),
            })
        }
        "/fork" => Some(MappableBuiltinCommand::Fork {
            target_id: text.into(),
        }),
        "/tree" => Some(MappableBuiltinCommand::Tree {
            target_id: text.into(),
        }),
        "/export" => jsonl_command(text, export_command),
        "/import" => jsonl_command(text, import_command),
        "/clone" => jsonl_command(text, clone_command),
        "/resume" => jsonl_command(text, resume_command),
        "/settings" | "/share" | "/trust" | "/login" | "/logout" | "/reload" | "/help"
        | "/doctor" | "/rewind" | "/history" | "/find" | "/jump" | "/effort"
        | "/always-approve" | "/auto" | "/plan" | "/remember" | "/goal" | "/workflow" | "/loop"
        | "/deep-research" | "/feedback" | "/usage" | "/memory" | "/skills" | "/hooks"
        | "/plugins" => Some(MappableBuiltinCommand::Extended {
            name: prefix.trim_start_matches('/').to_owned(),
            args: text.to_owned(),
        }),
        _ => None,
    }
}

fn jsonl_command(
    path: &str,
    make: fn(String) -> MappableBuiltinCommand,
) -> Option<MappableBuiltinCommand> {
    path.ends_with(".jsonl").then(|| make(path.to_owned()))
}

#[allow(clippy::too_many_lines)]
fn parse_extended_no_arg(value: &str) -> Option<MappableBuiltinCommand> {
    let name = value.strip_prefix('/')?;
    matches!(
        name,
        "settings"
            | "share"
            | "trust"
            | "login"
            | "logout"
            | "reload"
            | "context"
            | "recap"
            | "view-plan"
            | "skills"
            | "hooks"
            | "plugins"
            | "mcps"
            | "memory"
            | "workflows"
    )
    .then(|| MappableBuiltinCommand::Extended {
        name: name.to_owned(),
        args: String::new(),
    })
}

fn export_command(path: String) -> MappableBuiltinCommand {
    MappableBuiltinCommand::Export { path }
}
fn import_command(path: String) -> MappableBuiltinCommand {
    MappableBuiltinCommand::Import { path }
}
fn clone_command(path: String) -> MappableBuiltinCommand {
    MappableBuiltinCommand::Clone { path }
}
fn resume_command(path: String) -> MappableBuiltinCommand {
    MappableBuiltinCommand::Resume { path }
}

/// Classify a submitted line against Pi's complete built-in registry while
/// retaining the existing mappable parser for execution.
pub fn classify_builtin_command(input: &str) -> BuiltinCommandDisposition {
    if let Some(command) = parse_mappable_builtin_command(input) {
        return BuiltinCommandDisposition::Mappable(command);
    }
    let Some(name) = input.trim().strip_prefix('/').and_then(|value| {
        value
            .split_whitespace()
            .next()
            .filter(|name| !name.is_empty())
    }) else {
        return BuiltinCommandDisposition::NotBuiltin;
    };
    match name {
        "settings" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Settings),
        "model" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Model),
        "scoped-models" => {
            BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::ScopedModels)
        }
        "export" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Export),
        "import" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Import),
        "share" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Share),
        "copy" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Copy),
        "name" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Name),
        "session" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Session),
        "changelog" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Changelog),
        "hotkeys" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Hotkeys),
        "trust" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Trust),
        "login" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Login),
        "logout" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Logout),
        "new" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::New),
        "compact" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Compact),
        "reload" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Reload),
        "fork" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Fork),
        "clone" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Clone),
        "tree" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Tree),
        "resume" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Resume),
        "quit" => BuiltinCommandDisposition::Unsupported(UnsupportedBuiltinCommand::Quit),
        _ => BuiltinCommandDisposition::NotBuiltin,
    }
}

#[cfg(test)]
mod tests {
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
}
