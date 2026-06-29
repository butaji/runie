//! Command Category

use std::str::FromStr;
use strum::Display;

/// Command category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum CommandCategory {
    Core,
    Session,
    Model,
    Safety,
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

impl Default for CommandCategory {
    fn default() -> Self {
        Self::System
    }
}

impl CommandCategory {
    /// Label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Core => "Core",
            Self::Session => "Session",
            Self::Model => "Model",
            Self::Safety => "Safety",
        }
    }

    pub fn as_str(&self) -> &str {
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
}
