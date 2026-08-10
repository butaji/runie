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

/// Parse the actor-owned background-job control arguments.
pub fn parse_background_job_command(args: &str) -> Option<&str> {
    let mut parts = args.split_whitespace();
    (parts.next() == Some("cancel") && parts.next().is_some() && parts.next().is_none())
        .then(|| args.split_whitespace().nth(1).unwrap())
}

/// Parse the lifecycle-wide background cancellation control.
pub fn parse_background_job_cancel_all(args: &str) -> bool {
    args.split_whitespace().eq(["cancel", "all"])
}

/// Parse an optional single background-job ID used by the inspection view.
/// Control verbs such as `cancel` stay owned by their dedicated parser.
pub fn parse_background_job_query(args: &str) -> Option<&str> {
    let mut parts = args.split_whitespace();
    let id = parts.next()?;
    (id != "cancel" && parts.next().is_none()).then_some(id)
}

pub fn parse_background_job_status_query(args: &str) -> Option<&str> {
    let status = args.trim();
    ["running", "completed", "failed", "cancelled"]
        .contains(&status)
        .then_some(status)
}

#[allow(clippy::too_many_lines)]
fn parse_parameterized_command(value: &str) -> Option<MappableBuiltinCommand> {
    if let Some(command) = parse_compact_command(value) {
        return Some(command);
    }
    let (prefix, text) = value.split_once(' ')?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    match prefix {
        "/name" => Some(MappableBuiltinCommand::Name { name: text.into() }),
        "/model" if valid_model_reference(text) => Some(MappableBuiltinCommand::Model {
            reference: text.into(),
        }),
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
        | "/doctor" | "/rewind" | "/history" | "/find" | "/jump" | "/context" | "/effort"
        | "/always-approve" | "/auto" | "/deny" | "/plan" | "/remember" | "/goal" | "/workflow"
        | "/jobs" | "/questions" | "/sessions" | "/loop" | "/deep-research" | "/feedback"
        | "/usage" | "/memory" | "/skills" | "/hooks" | "/plugins" => {
            Some(MappableBuiltinCommand::Extended {
                name: prefix.trim_start_matches('/').to_owned(),
                args: text.to_owned(),
            })
        }
        _ => None,
    }
}

fn parse_compact_command(value: &str) -> Option<MappableBuiltinCommand> {
    let instructions = value.strip_prefix("/compact ")?;
    Some(MappableBuiltinCommand::Compact {
        instructions: (!instructions.trim().is_empty()).then(|| instructions.trim().to_owned()),
    })
}

fn valid_model_reference(value: &str) -> bool {
    value
        .split_once('/')
        .is_some_and(|(provider, model)| !provider.is_empty() && !model.is_empty())
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
            | "effort"
            | "resume"
            | "memory"
            | "workflows"
            | "jobs"
            | "questions"
            | "undo"
            | "sessions"
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
#[path = "commands_tests.rs"]
mod tests;
