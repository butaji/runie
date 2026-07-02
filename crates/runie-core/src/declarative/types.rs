//! Types for declarative configuration.

use std::path::PathBuf;

use serde::de::{self, Error as SerdeError, Visitor};
use serde::{Deserialize, Deserializer};
use validator::Validate;

use crate::commands::CommandCategory;

/// Tagged enum for deserializing the `type` field from YAML.
///
/// Uses `#[serde(tag = "type")]` so that the `type` field discriminates
/// between variants. This replaces the old pattern of `kind_type: String`
/// plus many `Option` fields with `unwrap_or_default()`.
///
/// Example YAML:
/// ```yaml
/// type: form_with_handler
/// handler: save
/// title: "Save Session"
/// fields:
///   - label: Name
///     placeholder: session-name
///     key: name
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum CommandKind {
    /// Handler by name lookup.
    #[serde(alias = "handler")]
    Handler { handler: String },
    /// Static message.
    Msg { message: String },
    /// Form dialog with a named handler.
    #[serde(rename = "form_with_handler")]
    FormWithHandler {
        title: String,
        fields: Vec<FormFieldDef>,
        handler: String,
    },
}

impl CommandKind {
    /// Returns the handler name if this is a Handler or FormWithHandler variant.
    pub fn handler_name(&self) -> Option<&str> {
        match self {
            CommandKind::Handler { handler } => Some(handler),
            CommandKind::FormWithHandler { handler, .. } => Some(handler),
            CommandKind::Msg { .. } => None,
        }
    }

    /// Returns the static message if this is a Msg variant.
    pub fn message(&self) -> Option<&str> {
        match self {
            CommandKind::Msg { message } => Some(message),
            _ => None,
        }
    }
}

/// A form field definition.
///
/// Validation ensures label, placeholder, and key are non-empty strings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct FormFieldDef {
    #[validate(length(min = 1, message = "label cannot be empty"))]
    pub label: String,
    #[validate(length(min = 1, message = "placeholder cannot be empty"))]
    pub placeholder: String,
    #[validate(length(min = 1, message = "key cannot be empty"))]
    pub key: String,
}

/// A command definition loaded from a YAML file.
/// This is the typed deserialization target; convert to `Command` via `build_cmd_from_yaml`.
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
    /// The command kind discriminated by the `type` field.
    #[serde(flatten)]
    pub kind: CommandKind,
}

/// Deserialize a category string.
/// Handles case-insensitive matching and legacy aliases to maintain backward compatibility.
fn deserialize_category<'de, D>(deserializer: D) -> Result<CommandCategory, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    CommandCategory::parse_case_insensitive(&s)
        .map_err(|_| SerdeError::custom(format!("unknown category: {s}")))
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
/// NOTE: This is kept for backward compatibility but the canonical representation
/// is now `Command` from `commands::dsl::command`.
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub name: String,
    pub description: String,
    pub category: CommandCategory,
    pub intent: String,
    pub shortcut: Option<String>,
    pub aliases: Vec<String>,
    pub has_subcommands: bool,
    pub file_path: PathBuf,
    pub yaml_kind: CommandKind,
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
    fn command_category_from_str_round_trip() {
        use crate::commands::CommandCategory;
        // Case-insensitive parsing (original behavior preserved)
        assert_eq!(
            CommandCategory::parse_case_insensitive("session"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("SESSION"),
            Ok(CommandCategory::Session)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("model"),
            Ok(CommandCategory::Model)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("safety"),
            Ok(CommandCategory::Safety)
        );
        // Aliases that map to System
        assert_eq!(
            CommandCategory::parse_case_insensitive("tool"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("help"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("system"),
            Ok(CommandCategory::System)
        );
        // Display round-trip
        assert_eq!(
            CommandCategory::parse_case_insensitive(&CommandCategory::Core.to_string()),
            Ok(CommandCategory::Core)
        );
        // Unknown returns error
        assert!(CommandCategory::parse_case_insensitive("nonexistent").is_err());
    }

    #[test]
    fn form_field_def_valid() {
        let field = FormFieldDef {
            label: "Name".into(),
            placeholder: "session-name".into(),
            key: "name".into(),
        };
        assert!(field.validate().is_ok());
    }

    #[test]
    fn form_field_def_empty_label() {
        let field = FormFieldDef {
            label: "".into(),
            placeholder: "session-name".into(),
            key: "name".into(),
        };
        assert!(field.validate().is_err());
    }

    #[test]
    fn form_field_def_empty_placeholder() {
        let field = FormFieldDef {
            label: "Name".into(),
            placeholder: "".into(),
            key: "name".into(),
        };
        assert!(field.validate().is_err());
    }

    #[test]
    fn form_field_def_empty_key() {
        let field = FormFieldDef {
            label: "Name".into(),
            placeholder: "session-name".into(),
            key: "".into(),
        };
        assert!(field.validate().is_err());
    }
}
