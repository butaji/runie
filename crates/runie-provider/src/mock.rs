use futures::Stream;
use runie_core::message::{ChatMessage, Role};
use runie_core::provider::Provider;
use runie_core::provider_event::{ProviderEvent, StopReason};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct MockProvider {
    delay_ms: Option<(u64, u64)>,
    seed: Option<u64>,
    counter: Arc<AtomicU64>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            delay_ms: None,
            seed: None,
            counter: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a provider with a deterministic delay sequence for reproducible
    /// tests. The same seed always yields the same delays across calls.
    pub fn with_delay(min_ms: u64, max_ms: u64) -> Self {
        Self::with_seed(min_ms, max_ms, 42)
    }

    /// Create a provider with an explicit seed for deterministic delays.
    pub fn with_seed(min_ms: u64, max_ms: u64, seed: u64) -> Self {
        Self {
            delay_ms: Some((min_ms, max_ms)),
            seed: Some(seed),
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn delay_ms(&self) -> Option<(u64, u64)> {
        self.delay_ms
    }

    pub(crate) fn random_delay(&self) -> Option<Duration> {
        self.delay_ms.map(|(min, max)| {
            let range = max.saturating_sub(min) + 1;
            let offset = if let Some(seed) = self.seed {
                let n = self.counter.fetch_add(1, Ordering::Relaxed);
                xorshift64star(seed.wrapping_add(n)) % range
            } else {
                rand::random::<u64>() % range
            };
            Duration::from_millis(offset + min)
        })
    }
}

fn xorshift64star(mut x: u64) -> u64 {
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(0x2545F4914F6CDD1D)
}

fn file_tool_chunks(input: &str) -> Option<Vec<String>> {
    if input.contains("markup") {
        return Some(vec![
            "I'll list the files in the current directory.\n".to_owned(),
            r#"[TOOL_CALL]{tool => "list_dir", args => {"path" => "."}}[/TOOL_CALL]"#.to_owned(),
        ]);
    }
    if input.contains("list files") || input.contains("files") {
        return Some(vec![
            "I'll list the files in the current directory.\n".to_owned(),
            "TOOL:list_dir:.".to_owned(),
        ]);
    }
    if input.contains("read") {
        return Some(vec![
            "Let me read that file for you.\n".to_owned(),
            "TOOL:read_file:README.md".to_owned(),
        ]);
    }
    if input.contains("write") {
        return Some(vec![
            "I'll create that file for you.\n".to_owned(),
            "TOOL:write_file:hello.txt:Hello World".to_owned(),
        ]);
    }
    if input.contains("edit") {
        return Some(vec![
            "I'll make that edit for you.\n".to_owned(),
            r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#.to_owned(),
        ]);
    }
    None
}

fn malformed_tool_chunks(input: &str) -> Option<Vec<String>> {
    if input.contains("malformed") {
        return Some(vec![
            "I will call a malformed tool.\n".to_owned(),
            r#"{"name": "bash" "arguments": {"command": "echo hi"}}"#.to_owned(),
        ]);
    }
    None
}

fn command_tool_chunks(input: &str) -> Option<Vec<String>> {
    if input.contains("run") || input.contains("cmd") {
        return Some(vec![
            "I'll run that command for you.\n".to_owned(),
            "TOOL:bash:echo hello".to_owned(),
        ]);
    }
    if input.contains("grep") || input.contains("search") {
        return Some(vec![
            "I'll search for that pattern.\n".to_owned(),
            r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#.to_owned(),
        ]);
    }
    if input.contains("find") || input.contains("glob") {
        return Some(vec![
            "I'll find those files for you.\n".to_owned(),
            r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#.to_owned(),
        ]);
    }
    None
}

fn response_chunks(last: Option<&ChatMessage>, user_input: &str) -> Vec<String> {
    if matches!(last, Some(m) if m.role == Role::Tool) {
        return vec!["Done. I have the information you requested.".to_owned()];
    }
    let input_lower = user_input.to_lowercase();
    if let Some(chunks) = file_tool_chunks(&input_lower) {
        return chunks;
    }
    if let Some(chunks) = command_tool_chunks(&input_lower) {
        return chunks;
    }
    if let Some(chunks) = malformed_tool_chunks(&input_lower) {
        return chunks;
    }
    user_input
        .split_whitespace()
        .map(|w| format!("{} ", w))
        .collect()
}

fn last_user_content(messages: &[ChatMessage]) -> Option<String> {
    messages.iter().rev().find_map(|m| {
        if m.role == Role::User {
            Some(m.content())
        } else {
            None
        }
    })
}

impl Provider for MockProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let delay_ms = self.random_delay();
        let last = messages.last();
        let user_input = last_user_content(&messages).unwrap_or_default();
        let chunks = response_chunks(last, &user_input);

        Box::pin(async_stream::stream! {
            for chunk_text in chunks {
                if let Some(d) = delay_ms {
                    tokio::time::sleep(d).await;
                }
                yield Ok(ProviderEvent::TextDelta(chunk_text));
            }
            yield Ok(ProviderEvent::Finish { reason: StopReason::Stop });
        })
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let user_input = last_user_content(&messages).unwrap_or_default();
        if user_input.contains("native tool") {
            return Box::pin(native_tool_stream());
        }
        self.generate(messages)
    }
}

fn native_tool_stream(
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>> {
    Box::pin(async_stream::stream! {
        yield Ok(ProviderEvent::TextDelta("I'll run a command.\n".into()));
        yield Ok(ProviderEvent::ToolCallStart {
            id: "call_1".into(),
            name: "bash".into(),
        });
        yield Ok(ProviderEvent::ToolCallInputDelta {
            id: "call_1".into(),
            delta: "{\"command\":\"echo hi\"}".into(),
        });
        yield Ok(ProviderEvent::ToolCallEnd { id: "call_1".into() });
        yield Ok(ProviderEvent::Finish { reason: StopReason::ToolCalls });
    })
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
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let user_input = last_user_content(&messages)
            .unwrap_or_else(|| "This is a test response with multiple words.".to_owned());
        let response = format!(
            "You said: '{}'. I understand and will help you with that task. ",
            user_input
        );
        let total_chunks = self
            .total_chunks
            .unwrap_or_else(|| response.len().div_ceil(self.chunk_size));

        stream_response(response, self.chunk_size, total_chunks, self.delay_ms)
    }
}

fn stream_response(
    response: String,
    chunk_size: usize,
    total_chunks: usize,
    delay_ms: u64,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>> {
    Box::pin(async_stream::stream! {
        for i in 0..total_chunks {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(response.len());
            let chunk = String::from_utf8_lossy(&response.as_bytes()[start..end]).to_string();
            yield Ok(ProviderEvent::TextDelta(chunk));
            if i < total_chunks - 1 && delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
        yield Ok(ProviderEvent::Finish { reason: StopReason::Stop });
    })
}
