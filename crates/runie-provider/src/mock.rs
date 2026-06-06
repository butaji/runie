//! Mock provider for testing

use anyhow::Result;
use runie_core::provider::{Message, Provider, ResponseChunk};

/// A mock provider that simulates LLM responses for testing.
#[derive(Default, Clone)]
pub struct MockProvider;

impl Provider for MockProvider {
    async fn generate<F>(
        &self,
        messages: Vec<Message>,
        mut on_chunk: F,
    ) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send,
    {
        let last = messages.last();

        // If last message is a tool result, respond with a final answer
        if matches!(last, Some(Message::ToolResult { .. })) {
            on_chunk(ResponseChunk {
                content: "Done. I have the information you requested.".to_string(),
            });
            return Ok(());
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
            on_chunk(ResponseChunk {
                content: "TOOL:list_dir:.".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("read") {
            on_chunk(ResponseChunk {
                content: "TOOL:read_file:README.md".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("write") {
            on_chunk(ResponseChunk {
                content: "TOOL:write_file:hello.txt:Hello World".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("run") || user_input.to_lowercase().contains("cmd")
        {
            on_chunk(ResponseChunk {
                content: "TOOL:bash:echo hello".to_string(),
            });
            return Ok(());
        }

        // Default: echo back the input word by word
        for word in user_input.split_whitespace() {
            on_chunk(ResponseChunk {
                content: format!("{} ", word),
            });
        }

        Ok(())
    }
}
