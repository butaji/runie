//! Runie Agent - Agent components
//! 
//! Provider trait and implementations.

use serde::{Deserialize, Serialize};

// === Message Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}

#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

// === Command ===

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
}

// === Provider Trait ===

pub trait Provider: Send {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}

// === Mock Provider ===

#[derive(Default, Clone)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk> {
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        user_input
            .split_whitespace()
            .map(|word| ResponseChunk {
                content: format!("{} ", word),
            })
            .collect()
    }
}
