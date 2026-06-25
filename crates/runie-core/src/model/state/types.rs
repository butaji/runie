//! Core application state types used by `AppState`.

/// A file entry from the FFF indexer for the file picker.
#[derive(Clone, Debug)]
pub(crate) struct FffFileEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_dir: bool,
    pub(crate) score: f64,
    pub(crate) git_status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DeliveryMode {
    /// Each message triggers a separate LLM call
    #[default]
    OneAtATime,
    /// All queued messages delivered together in one LLM call
    All,
}

/// Thinking level for reasoning-intensive tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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

    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Low => "\nThink briefly before responding.",
            Self::Medium => "\nThink step by step before responding.",
            Self::High => "\nThink deeply and thoroughly. Consider edge cases and alternatives.",
        }
    }

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

impl std::str::FromStr for ThinkingLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("Unknown thinking level: {s}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

/// Active permission approval prompt shown as a blocking modal dialog.
#[derive(Clone, Debug)]
pub struct PermissionRequestState {
    pub request_id: String,
    pub tool: String,
    pub input: serde_json::Value,
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
