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
    NewSession,
    Hotkeys,
    Quit,
    Model { reference: String },
    ScopedModels,
    SessionInfo,
    Name { name: String },
    Compact { instructions: Option<String> },
    Fork { target_id: String },
    Tree { target_id: String },
    Export { path: String },
    Import { path: String },
    Clone { path: String },
    Resume { path: String },
}

/// Classification at the slash-command boundary. Unsupported Pi commands are
/// observable capabilities rather than silent successful no-ops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinCommandDisposition {
    Mappable(MappableBuiltinCommand),
    Unsupported { name: String },
    NotBuiltin,
}

/// Parse an exact Pi built-in command that Runie can currently route through
/// an existing actor/application boundary. Non-mappable commands return
/// `None` and remain ordinary prompt text until their capability is built.
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the Pi command parser keeps the complete typed vocabulary in one pure boundary"
)]
pub fn parse_mappable_builtin_command(input: &str) -> Option<MappableBuiltinCommand> {
    match input.trim() {
        "/new" => Some(MappableBuiltinCommand::NewSession),
        "/hotkeys" => Some(MappableBuiltinCommand::Hotkeys),
        "/quit" => Some(MappableBuiltinCommand::Quit),
        "/scoped-models" => Some(MappableBuiltinCommand::ScopedModels),
        "/session" => Some(MappableBuiltinCommand::SessionInfo),
        "/compact" => Some(MappableBuiltinCommand::Compact { instructions: None }),
        value if value.starts_with("/compact ") => {
            let instructions = value[9..].trim();
            Some(MappableBuiltinCommand::Compact {
                instructions: (!instructions.is_empty()).then(|| instructions.to_owned()),
            })
        }
        value if value.starts_with("/name ") => {
            let name = value[6..].trim();
            (!name.is_empty()).then(|| MappableBuiltinCommand::Name {
                name: name.to_owned(),
            })
        }
        value if value.starts_with("/model ") => {
            let reference = value[7..].trim();
            reference
                .split_once('/')
                .filter(|(provider, model)| !provider.is_empty() && !model.is_empty())
                .map(|_| MappableBuiltinCommand::Model {
                    reference: reference.to_owned(),
                })
        }
        value if value.starts_with("/fork ") => {
            let target_id = value[6..].trim();
            (!target_id.is_empty()).then(|| MappableBuiltinCommand::Fork {
                target_id: target_id.to_owned(),
            })
        }
        value if value.starts_with("/tree ") => {
            let target_id = value[6..].trim();
            (!target_id.is_empty()).then(|| MappableBuiltinCommand::Tree {
                target_id: target_id.to_owned(),
            })
        }
        value if value.starts_with("/export ") => {
            let path = value[8..].trim();
            (path.ends_with(".jsonl") && !path.is_empty()).then(|| MappableBuiltinCommand::Export {
                path: path.to_owned(),
            })
        }
        value if value.starts_with("/import ") => {
            let path = value[8..].trim();
            (path.ends_with(".jsonl") && !path.is_empty()).then(|| MappableBuiltinCommand::Import {
                path: path.to_owned(),
            })
        }
        value if value.starts_with("/clone ") => {
            let path = value[7..].trim();
            (path.ends_with(".jsonl") && !path.is_empty()).then(|| MappableBuiltinCommand::Clone {
                path: path.to_owned(),
            })
        }
        value if value.starts_with("/resume ") => {
            let path = value[8..].trim();
            (path.ends_with(".jsonl") && !path.is_empty()).then(|| MappableBuiltinCommand::Resume {
                path: path.to_owned(),
            })
        }
        _ => None,
    }
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
    if PI_BUILTIN_SLASH_COMMANDS
        .iter()
        .any(|command| command.name == name)
    {
        BuiltinCommandDisposition::Unsupported {
            name: name.to_owned(),
        }
    } else {
        BuiltinCommandDisposition::NotBuiltin
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
        assert_eq!(
            PI_BUILTIN_SLASH_COMMANDS[1].argument_hint,
            Some("<provider/model>")
        );
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
        assert_eq!(
            parse_mappable_builtin_command(" /new "),
            Some(MappableBuiltinCommand::NewSession)
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
            parse_mappable_builtin_command("/name release parity"),
            Some(MappableBuiltinCommand::Name {
                name: "release parity".into()
            })
        );
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

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the classifier regression keeps all supported and unsupported command examples together"
    )]
    fn classifier_exposes_known_unsupported_capabilities() {
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
        assert_eq!(
            classify_builtin_command("/login openai"),
            BuiltinCommandDisposition::Unsupported {
                name: "login".into()
            }
        );
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
}
