//! Command Category

/// Command category for organization
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandCategory {
    Session,
    Model,
    Tool,
    System,
    Help,
}

impl CommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Model => "Model",
            Self::Tool => "Tool",
            Self::System => "System",
            Self::Help => "Help",
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
        assert_eq!(CommandCategory::Session.label(), "Session");
        assert_eq!(CommandCategory::Model.label(), "Model");
        assert_eq!(CommandCategory::Help.label(), "Help");
    }
}
