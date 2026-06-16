//! Message types shared across the application.

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
    pub fn as_str(&self) -> &'static str {
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Role::User),
            "thought" => Some(Role::Thought),
            "assistant" => Some(Role::Assistant),
            "tool" => Some(Role::Tool),
            "turn_complete" => Some(Role::TurnComplete),
            "system" => Some(Role::System),
            _ => None,
        }
    }
}

impl ChatMessage {
    /// Convert to a simple provider message (drops metadata).
    pub fn to_provider_message(&self) -> crate::provider::Message {
        match self.role {
            Role::System => crate::provider::Message::System {
                content: self.content.clone(),
            },
            Role::User | Role::Thought => crate::provider::Message::User {
                content: self.content.clone(),
            },
            Role::Assistant => crate::provider::Message::Assistant {
                content: self.content.clone(),
            },
            Role::Tool | Role::TurnComplete => crate::provider::Message::ToolResult {
                content: self.content.clone(),
            },
        }
    }
}

/// Metadata for chat messages (compaction and visibility control).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
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
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub timestamp: f64,
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub metadata: MessageMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Message;

    #[test]
    fn role_as_str_matches_provider_expectations() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::Tool.as_str(), "tool");
    }

    #[test]
    fn chat_message_round_trip_json() {
        let msg = ChatMessage {
            role: Role::User,
            content: "hello".to_string(),
            timestamp: 1234567890.0,
            id: "msg-1".to_string(),
            provider: "openai".to_string(),
            metadata: MessageMetadata::default(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content, "hello");
        assert_eq!(parsed.id, "msg-1");
    }

    #[test]
    fn chat_message_to_provider_message() {
        let msg = ChatMessage {
            role: Role::User,
            content: "hello".to_string(),
            timestamp: 0.0,
            id: "1".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
        };
        let provider_msg = msg.to_provider_message();
        assert!(matches!(provider_msg, Message::User { content } if content == "hello"));
    }

    #[test]
    fn role_from_str_round_trip() {
        for role in [Role::User, Role::Assistant, Role::System, Role::Tool] {
            let s = role.as_str();
            let parsed = Role::from_str(s).unwrap();
            assert_eq!(parsed, role);
        }
    }
}
