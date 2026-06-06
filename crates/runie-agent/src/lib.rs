//! Runie Agent - Agent components

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
}

pub trait Provider: Send {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}

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

/// Check if the content is a special command that needs tool execution
pub fn needs_tool_execution(content: &str) -> bool {
    content.to_lowercase().contains("list files")
}

/// Get fake file list for testing
pub fn get_fake_file_list() -> String {
    r#"src/
  main.rs
  lib.rs
  Cargo.toml
tests/
  test_main.rs
  test_lib.rs
README.md
Cargo.lock"#.to_string()
}
