//! Embedded command YAML — compiled into the binary at build time.
//!
//! All built-in slash commands are embedded here so they are available
//! regardless of filesystem layout (TUI, headless, CLI, tests).
//!
//! Uses `include_dir!` to automatically load all YAML files from
//! resources/commands/ at compile time, avoiding a hand-maintained list.

use crate::commands::dsl::handlers::HANDLER_REGISTRY;
use crate::commands::dsl::yaml::build_cmd_from_yaml;
use crate::commands::Command;
use include_dir::Dir;

/// Embed resources/commands/ at compile time via include_dir.
/// This avoids a hand-maintained list: adding a new .yaml file is enough.
static COMMANDS_DIR: Dir<'static> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/commands");

/// Load all embedded commands as `Command`.
pub fn load_embedded_commands() -> Vec<Command> {
    let handler_registry = &*HANDLER_REGISTRY;
    COMMANDS_DIR
        .files()
        .filter_map(|f| load_single_command_file(f, handler_registry))
        .collect()
}

/// Process a single command file from the embedded directory.
fn load_single_command_file(
    f: &include_dir::File<'_>,
    handler_registry: &crate::commands::dsl::handlers::HandlerRegistry,
) -> Option<Command> {
    let path = f.path();
    let extension = path.extension()?.to_str()?;
    if extension != "yaml" && extension != "yml" {
        return None;
    }

    // Safe: YAML files in resources/commands/ are always UTF-8.
    let yaml_contents = std::str::from_utf8(f.contents()).ok()?;
    let yaml: crate::declarative::types::DeclarativeCommandYaml = match serde_yaml::from_str(yaml_contents) {
        Ok(y) => y,
        Err(e) => {
            tracing::warn!(
                "Failed to parse {:?}: {}",
                path.file_name().and_then(|n| n.to_str()),
                e
            );
            return None;
        }
    };

    match build_cmd_from_yaml(&yaml, handler_registry) {
        Some(cmd) => Some(cmd),
        None => {
            tracing::warn!(
                "Failed to build command {}: handler not found or other error",
                yaml.name
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_command_has_handler_flow() {
        use crate::declarative::types::{CommandKind, DeclarativeCommandYaml};

        // Find the quit.yaml file
        let quit_yaml = COMMANDS_DIR
            .files()
            .find(|f| f.path().file_stem().map(|s| s == "quit").unwrap_or(false))
            .expect("quit.yaml should be embedded");

        let yaml_contents = std::str::from_utf8(quit_yaml.contents()).unwrap();
        let yaml: DeclarativeCommandYaml = serde_yaml::from_str(yaml_contents).unwrap();
        assert!(
            matches!(&yaml.kind, CommandKind::Handler { handler } if handler == "quit"),
            "quit yaml should deserialize to Handler(\"quit\")"
        );
    }

    #[test]
    fn all_yaml_files_load() {
        let cmds = load_embedded_commands();
        let yaml_count = COMMANDS_DIR
            .files()
            .filter(|f| {
                f.path()
                    .extension()
                    .map(|e| e == "yaml" || e == "yml")
                    .unwrap_or(false)
            })
            .count();

        // All YAML files should load successfully
        assert_eq!(
            cmds.len(),
            yaml_count,
            "all {} YAML files should be loaded",
            yaml_count
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn command_names_are_valid() {
        let cmds = load_embedded_commands();

        // All commands should have non-empty names
        for cmd in &cmds {
            assert!(!cmd.name.is_empty(), "command name should not be empty");
        }

        // Verify expected commands are present (using names from YAML content)
        let cmd_names: std::collections::HashSet<_> = cmds.iter().map(|c| c.name.clone()).collect();

        // These are the command names as defined in the YAML files
        let expected_commands = vec![
            "settings",
            "help",
            "quit",
            "model",
            "thinking",
            "scoped-models",
            "readonly",
            "trust",
            "untrust",
            "copy",
            "reload",
            "diagnostics",
            "skills",
            "skill",
            "prompt",
            "hotkeys",
            "theme",
            "approve",
            "reject",
            "provider",
            "save",
            "load",
            "delete",
            "export",
            "import",
            "sessions",
            "new",
            "reset",
            "history",
            "session_info",
            "tree",
            "share",
            "resume",
            "compact",
            "fork",
            "name",
        ];

        for expected in expected_commands {
            assert!(
                cmd_names.contains(expected),
                "command '{}' should be loaded",
                expected
            );
        }
    }
}
