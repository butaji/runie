//! Mock provider for testing

use runie_core::provider::{Message, Provider, ResponseChunk};

/// A mock provider that simulates LLM responses for testing.
#[derive(Default, Clone)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk> {
        let last = messages.last();

        // If last message is a tool result, respond with a final answer
        if matches!(last, Some(Message::ToolResult { .. })) {
            return vec![ResponseChunk {
                content: "Done. I have the information you requested.".to_string(),
            }];
        }

        // If user asks for files, use the list_files tool
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        if user_input.to_lowercase().contains("list files")
            || user_input.to_lowercase().contains("files")
        {
            return vec![ResponseChunk {
                content: "TOOL:list_dir:.".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("read") {
            return vec![ResponseChunk {
                content: "TOOL:read_file:README.md".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("write") {
            return vec![ResponseChunk {
                content: "TOOL:write_file:hello.txt:Hello World".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("run") || user_input.to_lowercase().contains("cmd")
        {
            return vec![ResponseChunk {
                content: "TOOL:bash:echo hello".to_string(),
            }];
        }

        // Default: echo back the input word by word
        user_input
            .split_whitespace()
            .map(|word| ResponseChunk {
                content: format!("{} ", word),
            })
            .collect()
    }
}
