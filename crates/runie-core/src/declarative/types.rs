//! Types for declarative configuration.

use std::path::PathBuf;

/// A trigger that activates a skill or command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// Slash command trigger (e.g., `/check-work`).
    Command(String),
    /// File pattern trigger (e.g., `*.xlsx`).
    FilePattern(String),
    /// Keyboard shortcut trigger (e.g., `Ctrl+b`).
    Shortcut(String),
}

impl Trigger {
    /// Parse a trigger from a string.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.starts_with('/') {
            Some(Trigger::Command(s.to_owned()))
        } else if s.starts_with("Ctrl+") || s.starts_with("Alt+") || s.starts_with("Shift+") {
            Some(Trigger::Shortcut(s.to_owned()))
        } else if s.contains('*') || s.contains('.') {
            Some(Trigger::FilePattern(s.to_owned()))
        } else {
            None
        }
    }
}

/// A loaded skill definition from a markdown file.
#[derive(Debug, Clone)]
pub struct SkillDef {
    /// Unique skill name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Additional context or instructions.
    pub context: Option<String>,
    /// Triggers that activate this skill.
    pub triggers: Vec<Trigger>,
    /// File path where this skill was defined.
    pub file_path: PathBuf,
    /// Whether users can invoke this skill directly.
    pub user_invocable: bool,
}

/// A loaded command definition from a YAML file.
#[derive(Debug, Clone)]
pub struct CommandDef {
    /// Unique command name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Category for organization.
    pub category: CommandCategory,
    /// Event variant name to emit when command executes.
    pub intent: String,
    /// Keyboard shortcut (if any).
    pub shortcut: Option<String>,
    /// Whether this command has sub-commands.
    pub has_subcommands: bool,
    /// File path where this command was defined.
    pub file_path: PathBuf,
}

/// Command category for organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Session,
    Model,
    Tool,
    System,
    Help,
    Unknown,
}

impl CommandCategory {
    /// Parse from string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "session" => CommandCategory::Session,
            "model" => CommandCategory::Model,
            "tool" => CommandCategory::Tool,
            "system" => CommandCategory::System,
            "help" => CommandCategory::Help,
            _ => CommandCategory::Unknown,
        }
    }
}

impl Default for CommandCategory {
    fn default() -> Self {
        CommandCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_parse_command() {
        let t = Trigger::parse("/check-work").unwrap();
        assert_eq!(t, Trigger::Command("/check-work".to_owned()));
    }

    #[test]
    fn trigger_parse_shortcut() {
        let t = Trigger::parse("Ctrl+b").unwrap();
        assert_eq!(t, Trigger::Shortcut("Ctrl+b".to_owned()));
    }

    #[test]
    fn trigger_parse_file_pattern() {
        let t = Trigger::parse("*.xlsx").unwrap();
        assert_eq!(t, Trigger::FilePattern("*.xlsx".to_owned()));
    }

    #[test]
    fn command_category_parse() {
        assert_eq!(CommandCategory::parse("session"), CommandCategory::Session);
        assert_eq!(CommandCategory::parse("Session"), CommandCategory::Session);
        assert_eq!(CommandCategory::parse("unknown"), CommandCategory::Unknown);
    }
}
