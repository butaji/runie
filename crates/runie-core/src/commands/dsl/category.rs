//! Command Category

use std::str::FromStr;
use strum::{Display, EnumString};

/// Command category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display, EnumString)]
#[strum(serialize_all = "PascalCase")]
#[strum(ascii_case_insensitive)]
#[derive(Default)]
pub enum CommandCategory {
    Core,
    Session,
    Model,
    Safety,
    #[default]
    System,
}

impl CommandCategory {
    /// Label for display (pascal case).
    pub fn label(&self) -> &'static str {
        // `Display` derive with `serialize_all = "PascalCase"` ensures this matches.
        match self {
            Self::Core => "Core",
            Self::Session => "Session",
            Self::Model => "Model",
            Self::Safety => "Safety",
            Self::System => "System",
        }
    }

    /// Parse a category string (case-insensitive), including legacy aliases.
    ///
    /// Standard values: `Core`, `Session`, `Model`, `Safety`, `System` (any case).
    /// Aliases: `tool`, `help`, `unknown`, `""` → `System`.
    #[allow(clippy::result_unit_err)]
    pub fn parse_case_insensitive(s: &str) -> Result<Self, ()> {
        // Try strum first (case-insensitive standard variants).
        if let Ok(cat) = Self::from_str(s) {
            return Ok(cat);
        }
        // Legacy aliases that map to System.
        match s.to_lowercase().as_str() {
            "tool" | "help" | "unknown" | "" => Ok(Self::System),
            _ => Err(()),
        }
    }

    /// String representation (pascal case).
    pub fn as_str(&self) -> &'static str {
        self.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_labels() {
        assert_eq!(CommandCategory::Core.label(), "Core");
        assert_eq!(CommandCategory::Session.label(), "Session");
        assert_eq!(CommandCategory::Model.label(), "Model");
        assert_eq!(CommandCategory::Safety.label(), "Safety");
        assert_eq!(CommandCategory::System.label(), "System");
    }

    #[test]
    fn test_category_order() {
        assert!(CommandCategory::Core < CommandCategory::Session);
        assert!(CommandCategory::Session < CommandCategory::Model);
        assert!(CommandCategory::Model < CommandCategory::Safety);
        assert!(CommandCategory::Safety < CommandCategory::System);
    }

    #[test]
    fn test_category_round_trip() {
        for cat in [
            CommandCategory::Core,
            CommandCategory::Session,
            CommandCategory::Model,
            CommandCategory::Safety,
            CommandCategory::System,
        ] {
            let s = cat.to_string();
            let parsed = CommandCategory::parse_case_insensitive(&s);
            assert_eq!(parsed, Ok(cat), "round-trip failed for {cat:?}");
        }
    }

    #[test]
    fn test_category_aliases() {
        // Legacy aliases for backward compatibility.
        assert_eq!(
            CommandCategory::parse_case_insensitive("tool"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("help"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive("unknown"),
            Ok(CommandCategory::System)
        );
        assert_eq!(
            CommandCategory::parse_case_insensitive(""),
            Ok(CommandCategory::System)
        );
        assert!(CommandCategory::parse_case_insensitive("nonexistent").is_err());
    }
}
