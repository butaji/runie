//! Types for declarative configuration.

use std::path::PathBuf;
use std::str::FromStr;

use serde::de::{self, Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer};

use crate::commands::CommandCategory;

/// What kind of command this is — determines how it executes.
#[derive(Debug, Clone)]
pub enum CommandKindDef {
    /// Static message to display.
    Msg { message: String },
    /// Reference to a named handler function.
    Handler { name: String },
    /// Form dialog with submit event.
    Form {
        title: String,
        fields: Vec<FormFieldDef>,
        submit_event: String,
    },
    /// Form dialog with custom handler.
    FormWithHandler {
        title: String,
        fields: Vec<FormFieldDef>,
        handler: String,
    },
}

/// A form field definition.
#[derive(Debug, Clone, Deserialize)]
pub struct FormFieldDef {
    pub label: String,
    pub placeholder: String,
    pub key: String,
}

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
    pub aliases: Vec<String>,
    #[serde(default)]
    pub sub: bool,
    #[serde(default, deserialize_with = "deserialize_triggers")]
    pub triggers: Vec<Trigger>,
    /// The command kind type: "handler", "msg", "form", "form_with_handler"
    #[serde(alias = "type", default)]
    pub kind_type: String,
    /// Handler name (for handler type)
    #[serde(default)]
    pub handler: Option<String>,
    /// Static message (for msg type)
    #[serde(default)]
    pub message: Option<String>,
    /// Form title (for form types)
    #[serde(default)]
    pub title: Option<String>,
    /// Form fields (for form types)
    #[serde(default)]
    pub fields: Vec<FormFieldDef>,
    /// Form submit event (for form type)
    #[serde(default)]
    pub submit_event: Option<String>,
    /// Form handler name (for form_with_handler type)
    #[serde(default)]
    pub form_handler: Option<String>,
}

impl DeclarativeCommandYaml {
    /// Convert to `CommandKindDef` based on the `kind_type` field.
    pub fn to_kind(&self) -> CommandKindDef {
        match self.kind_type.as_str() {
            "handler" => CommandKindDef::Handler {
                name: self.handler.clone().unwrap_or_default(),
            },
            "msg" => CommandKindDef::Msg {
                message: self.message.clone().unwrap_or_default(),
            },
            "form" => CommandKindDef::Form {
                title: self.title.clone().unwrap_or_default(),
                fields: self.fields.clone(),
                submit_event: self.submit_event.clone().unwrap_or_default(),
            },
            "form_with_handler" => CommandKindDef::FormWithHandler {
                title: self.title.clone().unwrap_or_default(),
                fields: self.fields.clone(),
                handler: self.form_handler.clone().unwrap_or_default(),
            },
            _ => CommandKindDef::Msg {
                message: format!("Unknown command type: {}", self.kind_type),
            },
        }
    }
}

/// Deserialize a category string using FromStr.
/// Handles case-insensitive matching to maintain backward compatibility.
fn deserialize_category<'de, D>(deserializer: D) -> Result<CommandCategory, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    CommandCategory::from_str(&s).map_err(|_| SerdeError::custom(format!("unknown category: {s}")))
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
    pub aliases: Vec<String>,
    pub has_subcommands: bool,
    pub file_path: PathBuf,
    /// Handler name for looking up the actual handler function.
    pub handler_name: Option<String>,
    /// Static message (for Msg type commands).
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
    fn command_category_from_str_round_trip() {
        use crate::commands::CommandCategory;
        // Case-insensitive parsing (original behavior preserved)
        assert_eq!(
            CommandCategory::from_str("session"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::from_str("SESSION"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::from_str("model"),
            Ok(CommandCategory::Model)
        );
        assert_eq!(
            CommandCategory::from_str("safety"),
            Ok(CommandCategory::Safety)
        );
        // Aliases that map to System
        assert_eq!(
            CommandCategory::from_str("tool"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::from_str("help"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::from_str("system"),
            Ok(CommandCategory::System)
        );
        // Display round-trip
        assert_eq!(
            CommandCategory::from_str(&CommandCategory::Core.to_string()),
            Ok(CommandCategory::Core)
        );
        // Unknown returns error
        assert!(CommandCategory::from_str("nonexistent").is_err());
    }
}
