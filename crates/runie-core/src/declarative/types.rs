//! Types for declarative configuration.

use std::path::PathBuf;

use serde::de::{self, Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer};

use crate::commands::CommandCategory;

/// A command definition loaded from a YAML file.
/// This is the typed deserialization target; convert to `CommandDef` via `From`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeCommandYaml {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, deserialize_with = "deserialize_category")]
    pub category: CommandCategory,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub subcommands: bool,
    #[serde(default, deserialize_with = "deserialize_triggers")]
    pub triggers: Vec<Trigger>,
}

fn deserialize_category<'de, D>(deserializer: D) -> Result<CommandCategory, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_category(&s)
        .ok_or_else(|| SerdeError::custom(format!("unknown category: {s}")))
}

/// Deserialize a YAML list of trigger strings into `Vec<Trigger>`.
fn deserialize_triggers<'de, D>(deserializer: D) -> Result<Vec<Trigger>, D::Error>
where
    D: Deserializer<'de>,
{
    struct TriggerVisitor;

    impl<'de> Visitor<'de> for TriggerVisitor {
        type Value = Vec<Trigger>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a list of trigger strings")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            let mut triggers = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                if let Some(t) = Trigger::parse(&s) {
                    triggers.push(t);
                }
            }
            Ok(triggers)
        }
    }

    deserializer.deserialize_seq(TriggerVisitor)
}

/// Parse a declarative category string to the shared `CommandCategory`.
pub fn parse_category(s: &str) -> Option<CommandCategory> {
    match s.to_lowercase().as_str() {
        "core" => Some(CommandCategory::Core),
        "session" => Some(CommandCategory::Session),
        "model" => Some(CommandCategory::Model),
        "safety" => Some(CommandCategory::Safety),
        "tool" | "help" | "system" | "unknown" | "" => Some(CommandCategory::System),
        _ => None,
    }
}

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
        assert_eq!(parse_category("session"), Some(CommandCategory::Session));
        assert_eq!(parse_category("Session"), Some(CommandCategory::Session));
        assert_eq!(parse_category("model"), Some(CommandCategory::Model));
        assert_eq!(parse_category("tool"), Some(CommandCategory::System));
        assert_eq!(parse_category("help"), Some(CommandCategory::System));
        assert_eq!(parse_category("unknown"), Some(CommandCategory::System));
        assert_eq!(parse_category("nonexistent"), None);
    }
}
