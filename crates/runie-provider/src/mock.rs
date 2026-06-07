use std::time::Duration;
use anyhow::Result;
use runie_core::provider::{Message, Provider, ResponseChunk};

#[derive(Default, Clone)]
pub struct MockProvider {
    delay_ms: Option<(u64, u64)>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_delay(min_ms: u64, max_ms: u64) -> Self {
        Self {
            delay_ms: Some((min_ms, max_ms)),
        }
    }

    pub fn delay_ms(&self) -> Option<(u64, u64)> {
        self.delay_ms
    }
}

impl Provider for MockProvider {
    async fn generate<F>(
        &self,
        messages: Vec<Message>,
        mut on_chunk: F,
    ) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send,
    {
        if let Some((min, max)) = self.delay_ms {
            let delay = rand::random::<u64>() % (max - min + 1) + min;
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        let last = messages.last();

        if matches!(last, Some(Message::ToolResult { .. })) {
            on_chunk(ResponseChunk {
                content: "Done. I have the information you requested.".to_string(),
            });
            return Ok(());
        }

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
                content: "I'll list the files in the current directory.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: "TOOL:list_dir:.".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("read") {
            on_chunk(ResponseChunk {
                content: "Let me read that file for you.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: "TOOL:read_file:README.md".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("write") {
            on_chunk(ResponseChunk {
                content: "I'll create that file for you.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: "TOOL:write_file:hello.txt:Hello World".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("edit") {
            on_chunk(ResponseChunk {
                content: "I'll make that edit for you.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#.to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("run") || user_input.to_lowercase().contains("cmd")
        {
            on_chunk(ResponseChunk {
                content: "I'll run that command for you.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: "TOOL:bash:echo hello".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("grep") || user_input.to_lowercase().contains("search")
        {
            on_chunk(ResponseChunk {
                content: "I'll search for that pattern.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#.to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("find") || user_input.to_lowercase().contains("glob")
        {
            on_chunk(ResponseChunk {
                content: "I'll find those files for you.\n".to_string(),
            });
            on_chunk(ResponseChunk {
                content: r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#.to_string(),
            });
            return Ok(());
        }

        for word in user_input.split_whitespace() {
            on_chunk(ResponseChunk {
                content: format!("{} ", word),
            });
        }

        Ok(())
    }
}
