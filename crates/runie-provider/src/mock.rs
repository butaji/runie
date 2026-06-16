use futures::Stream;
use runie_core::llm_event::{LLMEvent, StopReason};
use runie_core::provider::{Message, Provider};
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

fn file_tool_chunks(input: &str) -> Option<Vec<String>> {
    if input.contains("list files") || input.contains("files") {
        return Some(vec![
            "I'll list the files in the current directory.\n".to_string(),
            "TOOL:list_dir:.".to_string(),
        ]);
    }
    if input.contains("read") {
        return Some(vec![
            "Let me read that file for you.\n".to_string(),
            "TOOL:read_file:README.md".to_string(),
        ]);
    }
    if input.contains("write") {
        return Some(vec![
            "I'll create that file for you.\n".to_string(),
            "TOOL:write_file:hello.txt:Hello World".to_string(),
        ]);
    }
    if input.contains("edit") {
        return Some(vec![
            "I'll make that edit for you.\n".to_string(),
            r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#
                .to_string(),
        ]);
    }
    None
}

fn command_tool_chunks(input: &str) -> Option<Vec<String>> {
    if input.contains("run") || input.contains("cmd") {
        return Some(vec![
            "I'll run that command for you.\n".to_string(),
            "TOOL:bash:echo hello".to_string(),
        ]);
    }
    if input.contains("grep") || input.contains("search") {
        return Some(vec![
            "I'll search for that pattern.\n".to_string(),
            r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#
                .to_string(),
        ]);
    }
    if input.contains("find") || input.contains("glob") {
        return Some(vec![
            "I'll find those files for you.\n".to_string(),
            r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#.to_string(),
        ]);
    }
    None
}

fn response_chunks(last: Option<&Message>, user_input: &str) -> Vec<String> {
    if matches!(last, Some(Message::ToolResult { .. })) {
        return vec!["Done. I have the information you requested.".to_string()];
    }
    let input_lower = user_input.to_lowercase();
    if let Some(chunks) = file_tool_chunks(&input_lower) {
        return chunks;
    }
    if let Some(chunks) = command_tool_chunks(&input_lower) {
        return chunks;
    }
    user_input
        .split_whitespace()
        .map(|w| format!("{} ", w))
        .collect()
}

impl Provider for MockProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<LLMEvent>> + Send + '_>> {
        let delay_ms = self.random_delay();
        let last = messages.last();
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let chunks = response_chunks(last, &user_input);

        Box::pin(async_stream::stream! {
            for chunk_text in chunks {
                if let Some(d) = delay_ms {
                    tokio::time::sleep(d).await;
                }
                yield Ok(LLMEvent::TextDelta(chunk_text));
            }
            yield Ok(LLMEvent::Finish { reason: StopReason::Stop });
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
        Self {
            chunk_size: 1,
            delay_ms,
            total_chunks: None,
        }
    }

    pub fn with_variable_rate() -> Self {
        Self {
            chunk_size: 1,
            delay_ms: 30,
            total_chunks: None,
        }
    }
}

impl Provider for MockStreamingProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<LLMEvent>> + Send + '_>> {
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
                let chunk = String::from_utf8_lossy(&response.as_bytes()[start..end]).to_string();
                yield Ok(LLMEvent::TextDelta(chunk));
                if i < total_chunks - 1 && delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
            yield Ok(LLMEvent::Finish { reason: StopReason::Stop });
        })
    }
}
