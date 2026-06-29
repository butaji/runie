//! Embedded command YAML — compiled into the binary at build time.
//!
//! All built-in slash commands are embedded here so they are available
//! regardless of filesystem layout (TUI, headless, CLI, tests).
//!
//! Paths are relative to this file (src/commands/dsl/):
//!   ../../../../ → crates/runie-core/

use crate::declarative::types::{CommandDef as DeclDef, DeclarativeCommandYaml};
use crate::commands::dsl::spec::CommandDef;
use crate::commands::dsl::handlers::HANDLER_REGISTRY;
use crate::commands::dsl::spec::build_cmd_from_yaml;

pub const SETTINGS: &str =
    include_str!("../../../resources/commands/settings.yaml");
pub const HELP: &str = include_str!("../../../resources/commands/help.yaml");
pub const QUIT: &str = include_str!("../../../resources/commands/quit.yaml");
pub const MODEL: &str = include_str!("../../../resources/commands/model.yaml");
pub const THINKING: &str =
    include_str!("../../../resources/commands/thinking.yaml");
pub const SCOPED_MODELS: &str =
    include_str!("../../../resources/commands/scoped-models.yaml");
pub const READONLY: &str =
    include_str!("../../../resources/commands/readonly.yaml");
pub const TRUST: &str = include_str!("../../../resources/commands/trust.yaml");
pub const UNTRUST: &str =
    include_str!("../../../resources/commands/untrust.yaml");
pub const COPY: &str = include_str!("../../../resources/commands/copy.yaml");
pub const RELOAD: &str =
    include_str!("../../../resources/commands/reload.yaml");
pub const DIAGNOSTICS: &str =
    include_str!("../../../resources/commands/diagnostics.yaml");
pub const SKILLS: &str = include_str!("../../../resources/commands/skills.yaml");
pub const SKILL: &str = include_str!("../../../resources/commands/skill.yaml");
pub const PROMPT: &str =
    include_str!("../../../resources/commands/prompt.yaml");
pub const HOTKEYS: &str =
    include_str!("../../../resources/commands/hotkeys.yaml");
pub const THEME: &str = include_str!("../../../resources/commands/theme.yaml");
pub const APPROVE: &str =
    include_str!("../../../resources/commands/approve.yaml");
pub const REJECT: &str =
    include_str!("../../../resources/commands/reject.yaml");
pub const PROVIDER: &str =
    include_str!("../../../resources/commands/provider.yaml");
pub const SAVE: &str = include_str!("../../../resources/commands/save.yaml");
pub const LOAD: &str = include_str!("../../../resources/commands/load.yaml");
pub const DELETE: &str =
    include_str!("../../../resources/commands/delete.yaml");
pub const EXPORT: &str =
    include_str!("../../../resources/commands/export.yaml");
pub const IMPORT: &str =
    include_str!("../../../resources/commands/import.yaml");
pub const SESSIONS: &str =
    include_str!("../../../resources/commands/sessions.yaml");
pub const NEW: &str = include_str!("../../../resources/commands/new.yaml");
pub const RESET: &str = include_str!("../../../resources/commands/reset.yaml");
pub const HISTORY: &str =
    include_str!("../../../resources/commands/history.yaml");
pub const SESSION: &str =
    include_str!("../../../resources/commands/session.yaml");
pub const TREE: &str = include_str!("../../../resources/commands/tree.yaml");
pub const SHARE: &str = include_str!("../../../resources/commands/share.yaml");
pub const RESUME: &str =
    include_str!("../../../resources/commands/resume.yaml");
pub const COMPACT: &str =
    include_str!("../../../resources/commands/compact.yaml");
pub const FORK: &str = include_str!("../../../resources/commands/fork.yaml");
pub const NAME: &str = include_str!("../../../resources/commands/name.yaml");

const ALL: &[(&str, &str)] = &[
    ("settings", SETTINGS),
    ("help", HELP),
    ("quit", QUIT),
    ("model", MODEL),
    ("thinking", THINKING),
    ("scoped-models", SCOPED_MODELS),
    ("readonly", READONLY),
    ("trust", TRUST),
    ("untrust", UNTRUST),
    ("copy", COPY),
    ("reload", RELOAD),
    ("diagnostics", DIAGNOSTICS),
    ("skills", SKILLS),
    ("skill", SKILL),
    ("prompt", PROMPT),
    ("hotkeys", HOTKEYS),
    ("theme", THEME),
    ("approve", APPROVE),
    ("reject", REJECT),
    ("provider", PROVIDER),
    ("save", SAVE),
    ("load", LOAD),
    ("delete", DELETE),
    ("export", EXPORT),
    ("import", IMPORT),
    ("sessions", SESSIONS),
    ("new", NEW),
    ("reset", RESET),
    ("history", HISTORY),
    ("session", SESSION),
    ("tree", TREE),
    ("share", SHARE),
    ("resume", RESUME),
    ("compact", COMPACT),
    ("fork", FORK),
    ("name", NAME),
];

/// Load all embedded commands as `spec::CommandDef`.
pub fn load_embedded_commands() -> Vec<CommandDef> {
    let handler_registry = &*HANDLER_REGISTRY;
    ALL.iter()
        .filter_map(|(name, yaml)| {
            let yaml: DeclarativeCommandYaml = match serde_yaml::from_str(yaml) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!("Failed to parse {}: {}", name, e);
                    return None;
                }
            };
            let (handler_name, message) = match yaml.kind_type.as_str() {
                "handler" | "form" | "form_with_handler" => (yaml.handler.clone(), None),
                "msg" => (None, yaml.message.clone()),
                _ => (None, None),
            };
            let decl_def = DeclDef {
                name: yaml.name.clone(),
                description: yaml.description,
                category: yaml.category,
                intent: yaml.intent,
                shortcut: yaml.shortcut,
                aliases: yaml.aliases,
                has_subcommands: yaml.sub,
                file_path: std::path::PathBuf::new(),
                handler_name,
                message,
            };
            match build_cmd_from_yaml(&decl_def, handler_registry) {
                Some(cmd) => Some(cmd),
                None => {
                    eprintln!("Failed to build command {}: handler not found or other error", yaml.name);
                    None
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_command_has_handler_flow() {
        use crate::declarative::types::DeclarativeCommandYaml;
        let yaml: DeclarativeCommandYaml = serde_yaml::from_str(QUIT).unwrap();
        assert_eq!(yaml.kind_type, "handler", "quit yaml should have kind_type handler");
        assert_eq!(yaml.handler.as_deref(), Some("quit"), "quit handler should be 'quit'");
    }

    #[test]
    fn all_36_commands_loaded() {
        let cmds = load_embedded_commands();
        assert_eq!(cmds.len(), 36, "all 36 commands should be loaded");
    }
}
