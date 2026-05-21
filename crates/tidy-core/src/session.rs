use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::Message;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub messages: Vec<MessageNode>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl Session {
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn add_message(&mut self, parent_id: Option<String>, message: Message) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let node = MessageNode {
            id: id.clone(),
            parent_id,
            message,
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        };
        self.messages.push(node);
        self.updated_at = Utc::now();
        id
    }

    pub fn get_message(&self, id: &str) -> Option<&MessageNode> {
        self.messages.iter().find(|m| m.id == id)
    }

    pub fn get_thread(&self, message_id: &str) -> Vec<&MessageNode> {
        let mut thread = Vec::new();
        let mut current = self.get_message(message_id);
        while let Some(node) = current {
            thread.push(node);
            current = node.parent_id.as_ref().and_then(|id| self.get_message(id));
        }
        thread.reverse();
        thread
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub message: Message,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
