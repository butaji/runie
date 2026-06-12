use anyhow::Result;
use runie_core::provider::{Message, Provider, ResponseChunk};
use std::time::Duration;

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

    fn random_delay(&self) -> Option<Duration> {
        self.delay_ms.map(|(min, max)| {
            let range = max.saturating_sub(min) + 1;
            Duration::from_millis(rand::random::<u64>() % range + min)
        })
    }

    async fn maybe_sleep(&self) {
        if let Some(delay) = self.random_delay() {
            tokio::time::sleep(delay).await;
        }
    }
}

impl Provider for MockProvider {
    async fn generate<F>(&self, messages: Vec<Message>, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send,
    {
        self.maybe_sleep().await;

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
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll list the files in the current directory.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "TOOL:list_dir:.".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("read") {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "Let me read that file for you.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "TOOL:read_file:README.md".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("write") {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll create that file for you.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "TOOL:write_file:hello.txt:Hello World".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("edit") {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll make that edit for you.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#.to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("run") || user_input.to_lowercase().contains("cmd") {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll run that command for you.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "TOOL:bash:echo hello".to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("grep")
            || user_input.to_lowercase().contains("search")
        {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll search for that pattern.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#.to_string(),
            });
            return Ok(());
        }

        if user_input.to_lowercase().contains("find") || user_input.to_lowercase().contains("glob")
        {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: "I'll find those files for you.\n".to_string(),
            });
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#
                    .to_string(),
            });
            return Ok(());
        }

        for word in user_input.split_whitespace() {
            self.maybe_sleep().await;
            on_chunk(ResponseChunk {
                content: format!("{} ", word),
            });
        }

        Ok(())
    }
}

/// Mock provider that streams tokens character-by-character for testing animations.
/// Useful for testing token counter animations and speed calculations.
#[derive(Clone)]
pub struct MockStreamingProvider {
    /// Characters per chunk (default: 1 for char-by-char)
    pub chunk_size: usize,
    /// Delay between chunks in milliseconds
    pub delay_ms: u64,
    /// Total chunks to stream (None = based on response length)
    pub total_chunks: Option<usize>,
}

impl Default for MockStreamingProvider {
    fn default() -> Self {
        Self {
            chunk_size: 1, // Default to char-by-char streaming
            delay_ms: 10,  // Default 10ms between chunks
            total_chunks: None,
        }
    }
}

impl MockStreamingProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a provider that streams at a specific rate (tokens/sec)
    pub fn with_rate(tokens_per_sec: f64) -> Self {
        // Assuming ~4 chars per token, calculate delay per chunk
        let chars_per_token = 4.0;
        let delay_ms = if tokens_per_sec > 0.0 {
            ((chars_per_token / tokens_per_sec) * 1000.0) as u64
        } else {
            50 // Default 50ms
        };
        Self {
            chunk_size: 1,
            delay_ms,
            total_chunks: None,
        }
    }

    /// Create a provider that streams at a variable rate for testing animation
    pub fn with_variable_rate() -> Self {
        Self {
            chunk_size: 1,
            delay_ms: 30, // Fast for testing
            total_chunks: None,
        }
    }
}

impl Provider for MockStreamingProvider {
    async fn generate<F>(&self, messages: Vec<Message>, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send,
    {
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "This is a test response with multiple words.".to_string());

        // Create a response that echoes the input with some expansion
        let response = format!(
            "You said: '{}'. I understand and will help you with that task. ",
            user_input
        );

        // Stream the response in chunks
        let total_chunks = self
            .total_chunks
            .unwrap_or_else(|| response.len().div_ceil(self.chunk_size));

        for i in 0..total_chunks {
            let start = i * self.chunk_size;
            let end = (start + self.chunk_size).min(response.len());
            let chunk = &response[start..end];

            if !chunk.is_empty() {
                on_chunk(ResponseChunk {
                    content: chunk.to_string(),
                });
            }

            // Don't delay after the last chunk
            if i < total_chunks - 1 && self.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            }
        }

        Ok(())
    }
}
