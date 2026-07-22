//! Role and origin enums for chat messages.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumString, IntoStaticStr};

/// Role of a chat message participant.
///
/// Mirrors the OpenAI/Anthropic role taxonomy:
/// `user` (human input), `assistant` (model output), `tool` (tool result),
/// `thought` (model reasoning), `system` (configuration), `turn_complete` (marker).
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Role {
    #[default]
    User,
    Thought,
    Assistant,
    Tool,
    TurnComplete,
    System,
}

impl Role {
    /// String representation (snake_case).
    pub fn as_str(&self) -> &'static str {
        // Matches #[strum(serialize_all = "snake_case")] on the enum.
        match self {
            Role::User => "user",
            Role::Thought => "thought",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::TurnComplete => "turn_complete",
            Role::System => "system",
        }
    }

    /// Convert from API string representation.
    pub fn parse(s: &str) -> Option<Self> {
        Self::from_str(s).ok()
    }
}

/// Origin of a message, used to distinguish user messages from injected content.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum MessageOrigin {
    /// Direct user input.
    #[default]
    User,
    /// Tool result injected into the conversation.
    Tool,
    /// System message or prompt injection.
    System,
    /// Compaction summary (replaces older messages).
    Compaction,
    /// Steering or guidance message.
    Steering,
    /// Follow-up from user after turn completion.
    FollowUp,
    /// Session context injection (e.g., @file, @search).
    Context,
}
