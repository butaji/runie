use futures::Stream;
use runie_core::provider::{Message, Provider, ResponseChunk};
use std::pin::Pin;
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
}

impl Provider for MockProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ResponseChunk>> + Send + '_>> {
        let delay = self.random_delay();
        let last = messages.last();
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let input_lower = user_input.to_lowercase();

        // Determine response chunks based on input keywords.
        let chunks: Vec<String> = if matches!(last, Some(Message::ToolResult { .. })) {
            vec!["Done. I have the information you requested.".to_string()]
        } else if input_lower.contains("list files") || input_lower.contains("files") {
            vec![
                "I'll list the files in the current directory.\n".to_string(),
                "TOOL:list_dir:.".to_string(),
            ]
        } else if input_lower.contains("read") {
            vec![
                "Let me read that file for you.\n".to_string(),
                "TOOL:read_file:README.md".to_string(),
            ]
        } else if input_lower.contains("write") {
            vec![
                "I'll create that file for you.\n".to_string(),
                "TOOL:write_file:hello.txt:Hello World".to_string(),
            ]
        } else if input_lower.contains("edit") {
            vec![
                "I'll make that edit for you.\n".to_string(),
                r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#.to_string(),
            ]
        } else if input_lower.contains("run") || input_lower.contains("cmd") {
            vec![
                "I'll run that command for you.\n".to_string(),
                "TOOL:bash:echo hello".to_string(),
            ]
        } else if input_lower.contains("grep") || input_lower.contains("search") {
            vec![
                "I'll search for that pattern.\n".to_string(),
                r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#.to_string(),
            ]
        } else if input_lower.contains("find") || input_lower.contains("glob") {
            vec![
                "I'll find those files for you.\n".to_string(),
                r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#.to_string(),
            ]
        } else {
            // Echo each word as a chunk.
            user_input
                .split_whitespace()
                .map(|w| format!("{} ", w))
                .collect()
        };

        let delay_ms = delay;
        Box::pin(async_stream::stream! {
            for chunk_text in chunks {
                if let Some(d) = delay_ms {
                    tokio::time::sleep(d).await;
                }
                yield Ok(ResponseChunk { content: chunk_text });
            }
        })
    }
}

/// Mock provider that streams tokens character-by-character for testing animations.
#[derive(Clone)]
pub struct MockStreamingProvider {
    pub chunk_size: usize,
    pub delay_ms: u64,
    pub total_chunks: Option<usize>,
}

impl Default for MockStreamingProvider {
    fn default() -> Self {
        Self {
            chunk_size: 1,
            delay_ms: 10,
            total_chunks: None,
        }
    }
}

impl MockStreamingProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rate(tokens_per_sec: f64) -> Self {
        let chars_per_token = 4.0;
        let delay_ms = if tokens_per_sec > 0.0 {
            ((chars_per_token / tokens_per_sec) * 1000.0) as u64
        } else {
            50
        };
        Self { chunk_size: 1, delay_ms, total_chunks: None }
    }

    pub fn with_variable_rate() -> Self {
        Self { chunk_size: 1, delay_ms: 30, total_chunks: None }
    }
}

impl Provider for MockStreamingProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ResponseChunk>> + Send + '_>> {
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "This is a test response with multiple words.".to_string());

        let response = format!(
            "You said: '{}'. I understand and will help you with that task. ",
            user_input
        );

        let total_chunks = self
            .total_chunks
            .unwrap_or_else(|| response.len().div_ceil(self.chunk_size));

        let delay_ms = self.delay_ms;
        let chunk_size = self.chunk_size;

        Box::pin(async_stream::stream! {
            for i in 0..total_chunks {
                let start = i * chunk_size;
                let end = (start + chunk_size).min(response.len());
                let chunk = String::from_utf8_lossy(response[start..end].as_bytes()).to_string();
                yield Ok(ResponseChunk { content: chunk });
                if i < total_chunks - 1 && delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        })
    }
}
