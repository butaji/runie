//! Metadata for chat messages.

use serde::{Deserialize, Serialize};

use super::role::MessageOrigin;

/// Metadata for chat messages (compaction and visibility control).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    /// Message is pinned and won't be compacted.
    #[serde(default)]
    pub pinned: bool,
    /// Message is hidden from user display but still sent to the model.
    #[serde(default)]
    pub hidden_from_user: bool,
    /// Message is omitted from persistence (ephemeral).
    #[serde(default)]
    pub ephemeral: bool,
    /// This message is a compaction summary (replaces older messages).
    #[serde(default)]
    pub compacted: bool,
    /// Origin of the message (used for turn scheduling).
    #[serde(default)]
    pub origin: MessageOrigin,
}
