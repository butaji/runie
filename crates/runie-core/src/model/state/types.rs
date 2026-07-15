//! Core application state types used by `AppState`.

/// Status of an active goal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
}

impl GoalStatus {
    /// Human-readable label for this status.
    pub fn label(&self) -> &'static str {
        match self {
            GoalStatus::Active => "Active",
            GoalStatus::Paused => "Paused",
            GoalStatus::Completed => "Completed",
            GoalStatus::Cancelled => "Cancelled",
        }
    }
}

/// Active goal state — tracks the current objective and completion criterion.
#[derive(Clone, Debug)]
pub struct GoalState {
    pub objective: String,
    pub completion_criterion: Option<String>,
    pub status: GoalStatus,
    pub created_at: std::time::Instant,
    pub turns_spent: usize,
}

impl GoalState {
    /// Create a new active goal.
    pub fn new(objective: String, completion_criterion: Option<String>) -> Self {
        Self {
            objective,
            completion_criterion,
            status: GoalStatus::Active,
            created_at: std::time::Instant::now(),
            turns_spent: 0,
        }
    }
}

/// A file entry from the FFF indexer for the file picker.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FffFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub score: f64,
    pub git_status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DeliveryMode {
    /// Each message triggers a separate LLM call
    #[default]
    OneAtATime,
    /// All queued messages delivered together in one LLM call
    All,
}

/// Thinking level for reasoning-intensive tasks.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    strum::EnumString,
)]
#[strum(ascii_case_insensitive)]
pub enum ThinkingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ThinkingLevel {
    /// All thinking levels in cycle order (low → high).
    /// Single source of truth for UI selectors.
    pub const ALL: &'static [ThinkingLevel] = &[
        ThinkingLevel::Off,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ];

    /// All thinking levels in cycle order. See [`ALL`](Self::ALL).
    pub fn all() -> &'static [ThinkingLevel] {
        Self::ALL
    }

    /// Cycle to the next thinking level.
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    /// Prompt suffix for this thinking level.
    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Low => "\nThink briefly before responding.",
            Self::Medium => "\nThink step by step before responding.",
            Self::High => "\nThink deeply and thoroughly. Consider edge cases and alternatives.",
        }
    }

    /// Lowercase string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Returns the I Ching hexagram for this thinking level.
    /// Maps to 3-bit representation: 000=earth, 111=heaven.
    pub fn hexagram(&self) -> &'static str {
        match self {
            Self::Off => "☷",    // 000 - earth (no thinking)
            Self::Low => "☵",    // 010 - water (minimal thinking)
            Self::Medium => "☳", // 100 - thunder (moderate thinking)
            Self::High => "☰",   // 111 - heaven (deep thinking)
        }
    }
}

/// Queued message for steering/follow-up.
/// Fields are public to allow test setup in dependent crates.
#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

/// Active permission approval prompt shown as a blocking modal dialog.
/// Fields are public to allow test setup in dependent crates.
#[derive(Clone, Debug)]
pub struct PermissionRequestState {
    pub request_id: String,
    pub tool: String,
    pub input: serde_json::Value,
}

/// A single question in an AskUserQuestion dialog.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Question {
    /// Unique identifier for this question
    pub id: String,
    /// The question text shown to the user
    pub question: String,
    /// Available options for this question
    pub options: Vec<QuestionOption>,
}

/// A single option within a question.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QuestionOption {
    /// Option identifier (e.g., "_1", "_S")
    pub id: String,
    /// Display label for this option
    pub label: String,
}

/// Active AskUserQuestion dialog state.
/// Tracks which question the user is on and their answers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuestionState {
    pub request_id: String,
    pub questions: Vec<Question>,
    pub current_index: usize,
    pub answers: Vec<Answer>,
}

impl QuestionState {
    /// Create a new question state.
    pub fn new(request_id: String, questions: Vec<Question>) -> Self {
        Self {
            request_id,
            questions,
            current_index: 0,
            answers: Vec::new(),
        }
    }

    /// Get the current question, if any.
    pub fn current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_index)
    }

    /// Record an answer for the current question and advance.
    pub fn answer(&mut self, option_id: String) {
        if self.current_index < self.questions.len() {
            self.answers.push(Answer {
                question_id: self.questions[self.current_index].id.clone(),
                option_id,
            });
            self.current_index += 1;
        }
    }

    /// Skip the current question without answering.
    pub fn skip(&mut self) {
        if self.current_index < self.questions.len() {
            self.answers.push(Answer {
                question_id: self.questions[self.current_index].id.clone(),
                option_id: "_S".to_string(),
            });
            self.current_index += 1;
        }
    }

    /// Check if all questions have been answered or skipped.
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.questions.len()
    }
}

/// An answer to a question.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub question_id: String,
    pub option_id: String,
}

/// Identifies which component is currently receiving keyboard input.
/// Used to determine how Esc should behave (e.g., close dialog vs enter vim-nav).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InputReceiver {
    /// The main chat input field is active
    #[default]
    ChatInput,
    /// A dialog/overlay is open and receiving input
    Dialog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_level_iterates() {
        // Test that cycle() goes through all levels and wraps correctly
        assert_eq!(ThinkingLevel::Off.cycle(), ThinkingLevel::Low);
        assert_eq!(ThinkingLevel::Low.cycle(), ThinkingLevel::Medium);
        assert_eq!(ThinkingLevel::Medium.cycle(), ThinkingLevel::High);
        assert_eq!(ThinkingLevel::High.cycle(), ThinkingLevel::Off);
    }

    #[test]
    fn thinking_level_all_contains_all() {
        for level in ThinkingLevel::ALL {
            assert!(ThinkingLevel::all().contains(level));
        }
        assert_eq!(ThinkingLevel::ALL.len(), 4);
    }
}
