//! Types for declarative configuration.

use std::path::PathBuf;

use crate::commands::CommandCategory;

/// A trigger that activates a skill or command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    Command(String),
    FilePattern(String),
    Shortcut(String),
}

impl Trigger {
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
    pub name: String,
    pub description: String,
    pub context: Option<String>,
    pub triggers: Vec<Trigger>,
    pub file_path: PathBuf,
    pub user_invocable: bool,
}

/// A loaded command definition from a YAML file.
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub name: String,
    pub description: String,
    /// Category — uses the shared `CommandCategory` from commands DSL.
    pub category: CommandCategory,
    pub intent: String,
    pub shortcut: Option<String>,
    pub has_subcommands: bool,
    pub file_path: PathBuf,
}

/// Parse a declarative category string to the shared `CommandCategory`.
/// Tool, Help, and Unknown map to System since they don't have DSL equivalents.
pub fn parse_category(s: &str) -> CommandCategory {
    match s.to_lowercase().as_str() {
        "session" => CommandCategory::Session,
        "model" => CommandCategory::Model,
        "tool" | "help" | "system" | "unknown" | "" => CommandCategory::System,
        _ => CommandCategory::System,
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
        use crate::commands::CommandCategory;
        assert_eq!(parse_category("session"), CommandCategory::Session);
        assert_eq!(parse_category("Session"), CommandCategory::Session);
        assert_eq!(parse_category("model"), CommandCategory::Model);
        assert_eq!(parse_category("tool"), CommandCategory::System);
        assert_eq!(parse_category("help"), CommandCategory::System);
        assert_eq!(parse_category("unknown"), CommandCategory::System);
    }
}
