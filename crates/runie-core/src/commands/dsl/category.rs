//! Command Category

use std::str::FromStr;
use strum::Display;

/// Command category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
#[strum(serialize_all = "PascalCase")]
#[derive(Default)]
pub enum CommandCategory {
    Core,
    Session,
    Model,
    Safety,
    #[default]
    System,
}

impl FromStr for CommandCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "core" => Ok(Self::Core),
            "session" => Ok(Self::Session),
            "model" => Ok(Self::Model),
            "safety" => Ok(Self::Safety),
            "tool" | "help" | "system" | "unknown" | "" => Ok(Self::System),
            _ => Err(()),
        }
    }
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
        use std::str::FromStr;
        for cat in [
            CommandCategory::Core,
            CommandCategory::Session,
            CommandCategory::Model,
            CommandCategory::Safety,
            CommandCategory::System,
        ] {
            let s = cat.to_string();
            let parsed = CommandCategory::from_str(&s);
            assert_eq!(parsed, Ok(cat), "round-trip failed for {cat:?}");
        }
    }
}
