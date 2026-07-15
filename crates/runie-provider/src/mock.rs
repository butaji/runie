//! Mock providers for testing.
//!
//! This module provides deterministic mock providers that can be configured
//! with fixture responses instead of relying on keyword matching heuristics.
//!
//! # Fixtures
//!
//! Fixtures are stored as inline constants and selected based on explicit
//! configuration. This replaces the previous brittle keyword-matching approach.

use crate::{Provider, ProviderMetadata};
use futures::Stream;
use runie_core::proto::message::{ChatMessage, Role};
use runie_core::provider_event::{ProviderEvent, StopReason};
use std::default::Default;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Seed used by [`MockProvider::default`] to produce deterministic delays.
const MOCK_DEFAULT_SEED: u64 = 42;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Response fixtures mapped by a unique key.
///
/// Each fixture defines the text chunks to emit and whether it triggers
/// a tool call. This replaces keyword-matching with explicit configuration.
#[derive(Debug, Clone)]
struct Fixture {
    /// Text chunks to emit (includes prelude text and TOOL: markers).
    prelude: Vec<String>,
    /// Tool call metadata for documentation/testing purposes.
    /// The actual tool calls are emitted as "TOOL:" markers in prelude.
    #[allow(dead_code)]
    tool_call: Option<(String, String)>,
}

/// Built-in fixtures for common tool scenarios.
mod fixtures {
    use super::Fixture;

    /// Fixture for list_dir tool call.
    /// Returns text that includes the "TOOL:" marker for agent parsing.
    pub fn list_dir() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll list the files in the current directory.\n".to_owned(),
                "TOOL:list_dir:.".to_owned(),
            ],
            tool_call: Some(("list_dir".to_owned(), r#"{"path": "."}"#.to_owned())),
        }
    }

    /// Fixture for read_file tool call.
    pub fn read_file() -> Fixture {
        Fixture {
            prelude: vec![
                "Let me read that file for you.\n".to_owned(),
                "TOOL:read_file:README.md".to_owned(),
            ],
            tool_call: Some((
                "read_file".to_owned(),
                r#"{"path": "README.md"}"#.to_owned(),
            )),
        }
    }

    /// Fixture for write_file tool call.
    pub fn write_file() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll create that file for you.\n".to_owned(),
                "TOOL:write_file:hello.txt:Hello World".to_owned(),
            ],
            tool_call: Some((
                "write_file".to_owned(),
                r#"{"path": "hello.txt", "content": "Hello World"}"#.to_owned(),
            )),
        }
    }

    /// Fixture for edit_file tool call.
    pub fn edit_file() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll make that edit for you.\n".to_owned(),
                r#"{"name": "edit_file", "arguments": {"path": "src/main.rs", "search": "old", "replace": "new"}}"#.to_owned(),
            ],
            tool_call: Some((
                "edit_file".to_owned(),
                r#"{"path": "src/main.rs", "search": "old", "replace": "new"}"#.to_owned(),
            )),
        }
    }

    /// Fixture for bash tool call.
    pub fn bash() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll run that command for you.\n".to_owned(),
                "TOOL:bash:echo hello".to_owned(),
            ],
            tool_call: Some(("bash".to_owned(), r#"{"command": "echo hello"}"#.to_owned())),
        }
    }

    /// Fixture for grep tool call.
    pub fn grep() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll search for that pattern.\n".to_owned(),
                r#"{"name": "grep", "arguments": {"pattern": "fn main", "path": ".", "glob": "*.rs"}}"#.to_owned(),
            ],
            tool_call: Some((
                "grep".to_owned(),
                r#"{"pattern": "fn main", "path": ".", "glob": "*.rs"}"#.to_owned(),
            )),
        }
    }

    /// Fixture for find tool call.
    pub fn find() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll find those files for you.\n".to_owned(),
                r#"{"name": "find", "arguments": {"pattern": "*.rs", "path": "."}}"#.to_owned(),
            ],
            tool_call: Some((
                "find".to_owned(),
                r#"{"pattern": "*.rs", "path": "."}"#.to_owned(),
            )),
        }
    }

    /// Fixture for malformed tool call (testing parse errors).
    pub fn malformed_tool() -> Fixture {
        Fixture {
            prelude: vec![
                "I will call a malformed tool.\n".to_owned(),
                r#"{"name": "bash" "arguments": {"command": "echo hi"}}"#.to_owned(),
            ],
            tool_call: Some((
                "bash".to_owned(),
                r#"{"name": "bash" "arguments": {"command": "echo hi"}}"#.to_owned(),
            )),
        }
    }

    /// Fixture for markup tool call (XML format).
    pub fn markup_tool() -> Fixture {
        Fixture {
            prelude: vec![
                "I'll list the files in the current directory.\n".to_owned(),
                r#"[TOOL_CALL]{tool => "list_dir", args => {"path" => "."}}[/TOOL_CALL]"#
                    .to_owned(),
            ],
            tool_call: Some(("list_dir".to_owned(), r#"{"path": "."}"#.to_owned())),
        }
    }

    /// Simple echo fixture that echoes back the user's input exactly.
    /// Adds a trailing newline so the streaming buffer can flush the response.
    pub fn echo(input: &str) -> Vec<String> {
        if input.is_empty() {
            Vec::new()
        } else {
            vec![format!("{}\n", input)]
        }
    }

    /// Completion response after tool result.
    pub fn done() -> Vec<String> {
        vec!["Done. I have the information you requested.".to_owned()]
    }
}

/// MiniMax-shaped reasoning stream: native `ThinkingStart`/`ThinkingDelta`/
/// `ThinkingEnd` events (no `<think>` markup in content), a tool call in the
/// first iteration, and fresh reasoning plus the final answer in the second.
/// Regression fixture for the live bug where iteration-2 reasoning rendered
/// as a bare fragment that looked like a duplicated assistant post.
fn thinking_tool_stream(
    delay_ms: Option<Duration>,
    second_iteration: bool,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>> {
    Box::pin(async_stream::stream! {
        if !second_iteration {
            yield Ok(ProviderEvent::ThinkingStart { id: "reasoning".into() });
            yield Ok(ProviderEvent::ThinkingDelta("Deciding to run a check.".into()));
            yield Ok(ProviderEvent::ThinkingEnd { id: "reasoning".into() });
            if let Some(d) = delay_ms {
                tokio::time::sleep(d).await;
            }
            yield Ok(ProviderEvent::TextDelta("I'll verify this with a quick check.\n".into()));
            yield Ok(ProviderEvent::TextDelta("TOOL:list_dir:.".into()));
            yield Ok(ProviderEvent::Finish { reason: StopReason::Stop });
        } else {
            yield Ok(ProviderEvent::ThinkingStart { id: "reasoning".into() });
            yield Ok(ProviderEvent::ThinkingDelta("The check confirmed it.".into()));
            yield Ok(ProviderEvent::ThinkingEnd { id: "reasoning".into() });
            if let Some(d) = delay_ms {
                tokio::time::sleep(d).await;
            }
            yield Ok(ProviderEvent::TextDelta("Yes, verified.\n".into()));
            yield Ok(ProviderEvent::Finish { reason: StopReason::Stop });
        }
    })
}

// ─── MockProvider ─────────────────────────────────────────────────────────────

/// Mock provider with configurable fixture-based responses.
///
/// This provider selects responses based on explicit fixture configuration
/// rather than fragile keyword matching. Use [`MockProviderBuilder`] to
/// configure which fixtures are active.
#[derive(Clone)]
pub struct MockProvider {
    delay_ms: Option<(u64, u64)>,
    seed: u64,
    counter: Arc<AtomicU64>,
    /// Explicit fixture to use, overriding auto-detection.
    fixture: Option<Fixture>,
    /// Whether to echo back user input when no fixture matches.
    echo_fallback: bool,
}

/// Builder for configuring a `MockProvider`.
#[derive(Default)]
pub struct MockProviderBuilder {
    delay_ms: Option<(u64, u64)>,
    seed: Option<u64>,
    fixture: Option<Fixture>,
    echo_fallback: Option<bool>,
}

impl MockProviderBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure deterministic delay range.
    pub fn with_delay(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.delay_ms = Some((min_ms, max_ms));
        self
    }

    /// Configure explicit seed for deterministic delays.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set an explicit fixture to always use.
    /// Note: Fixture construction is intentionally private; use the predefined
    /// builder methods (`.list_dir()`, `.read_file()`, etc.) instead.
    #[allow(private_interfaces)]
    pub fn with_fixture(mut self, fixture: Fixture) -> Self {
        self.fixture = Some(fixture);
        self
    }

    /// Set whether to echo back input when no fixture matches (default: true).
    pub fn with_echo_fallback(mut self, enabled: bool) -> Self {
        self.echo_fallback = Some(enabled);
        self
    }

    /// Configure for list_dir fixture.
    pub fn list_dir(self) -> Self {
        self.with_fixture(fixtures::list_dir())
    }

    /// Configure for read_file fixture.
    pub fn read_file(self) -> Self {
        self.with_fixture(fixtures::read_file())
    }

    /// Configure for write_file fixture.
    pub fn write_file(self) -> Self {
        self.with_fixture(fixtures::write_file())
    }

    /// Configure for edit_file fixture.
    pub fn edit_file(self) -> Self {
        self.with_fixture(fixtures::edit_file())
    }

    /// Configure for bash fixture.
    pub fn bash(self) -> Self {
        self.with_fixture(fixtures::bash())
    }

    /// Configure for grep fixture.
    pub fn grep(self) -> Self {
        self.with_fixture(fixtures::grep())
    }

    /// Configure for find fixture.
    pub fn find(self) -> Self {
        self.with_fixture(fixtures::find())
    }

    /// Configure for malformed tool fixture.
    pub fn malformed(self) -> Self {
        self.with_fixture(fixtures::malformed_tool())
    }

    /// Configure for markup-formatted tool fixture.
    pub fn markup(self) -> Self {
        self.with_fixture(fixtures::markup_tool())
    }

    pub fn build(self) -> MockProvider {
        MockProvider {
            delay_ms: self.delay_ms,
            seed: self.seed.unwrap_or(MOCK_DEFAULT_SEED),
            counter: Arc::new(AtomicU64::new(0)),
            fixture: self.fixture,
            echo_fallback: self.echo_fallback.unwrap_or(true),
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        // Delay by default so streaming is visible and events are processed in order.
        // Tests that need zero delay can use `MockProviderBuilder::new().with_echo_fallback(true).build()`.
        MockProviderBuilder::new().with_delay(100, 200).build()
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a provider with a deterministic delay sequence for reproducible
    /// tests. The same seed always yields the same delays across calls.
    pub fn with_delay(min_ms: u64, max_ms: u64) -> Self {
        MockProviderBuilder::new()
            .with_delay(min_ms, max_ms)
            .build()
    }

    /// Create a provider with an explicit seed for deterministic delays.
    pub fn with_seed(min_ms: u64, max_ms: u64, seed: u64) -> Self {
        MockProviderBuilder::new()
            .with_delay(min_ms, max_ms)
            .with_seed(seed)
            .build()
    }

    pub fn delay_ms(&self) -> Option<(u64, u64)> {
        self.delay_ms
    }

    pub(crate) fn random_delay(&self) -> Option<Duration> {
        self.delay_ms.map(|(min, max)| {
            let range = max.saturating_sub(min) + 1;
            let n = self.counter.fetch_add(1, Ordering::Relaxed);
            let offset = xorshift64star(self.seed.wrapping_add(n)) % range;
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

/// Detect which fixture to use based on the user input.
///
/// This replaces the previous keyword-matching functions with a
/// cleaner, explicit fixture selection.
fn detect_fixture(input: &str) -> Option<Fixture> {
    let input_lower = input.to_lowercase();

    // Order matters: check more specific patterns first
    if input_lower.contains("markup") {
        return Some(fixtures::markup_tool());
    }
    if input_lower.contains("list files") || input_lower.contains("files") {
        return Some(fixtures::list_dir());
    }
    if input_lower.contains("read") {
        return Some(fixtures::read_file());
    }
    if input_lower.contains("write") {
        return Some(fixtures::write_file());
    }
    if input_lower.contains("edit") {
        return Some(fixtures::edit_file());
    }
    if input_lower.contains("grep") || input_lower.contains("search") {
        return Some(fixtures::grep());
    }
    if input_lower.contains("find") || input_lower.contains("glob") {
        return Some(fixtures::find());
    }
    if input_lower.contains("malformed") {
        return Some(fixtures::malformed_tool());
    }
    if input_lower.contains("run") || input_lower.contains("cmd") {
        return Some(fixtures::bash());
    }

    None
}

/// Build response chunks from fixture or echo fallback.
fn response_from_fixture(
    fixture: Option<Fixture>,
    user_input: &str,
    echo_fallback: bool,
) -> Vec<String> {
    if let Some(f) = fixture {
        return f.prelude;
    }
    if echo_fallback {
        fixtures::echo(user_input)
    } else {
        vec!["I understand.".to_owned()]
    }
}

/// Check if last message is a tool result (triggers completion response).
fn is_after_tool_result(last: Option<&ChatMessage>) -> bool {
    matches!(last, Some(m) if m.role == Role::Tool)
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

/// Deterministic canned responses for the pattern orchestration prompts
/// (`[swarm-plan …]` / `[swarm-synthesize]` / `[improve-*]` markers, see
/// runie-patterns). Worker prompts carry no marker and fall through to the
/// normal fixture/echo path. The dag plan check must come before the
/// generic `[swarm-plan` prefix.
fn swarm_marker_response(user_input: &str) -> Option<Vec<String>> {
    if user_input.starts_with("[swarm-plan dag") {
        Some(vec![
            "[{\"task\": \"Summarize the task\", \"deps\": []}, {\"task\": \"Draft an implementation outline\", \"deps\": [0]}]"
                .to_owned(),
        ])
    } else if user_input.starts_with("[swarm-plan") {
        Some(vec![
            "[\"Summarize the task\", \"Draft an implementation outline\"]".to_owned(),
        ])
    } else if user_input.starts_with("[swarm-synthesize") {
        Some(vec![
            "Swarm complete: all workers finished successfully.".to_owned()
        ])
    } else if user_input.starts_with("[improve-generate]") {
        Some(vec![
            "Draft: here is the best answer to your task.".to_owned()
        ])
    } else if user_input.starts_with("[improve-revise]") {
        Some(vec![
            "Revised draft addressing all reviewer feedback.".to_owned()
        ])
    } else if user_input.starts_with("[improve-review]") {
        Some(vec!["APPROVED".to_owned()])
    } else {
        None
    }
}

/// Stream text chunks as TextDelta events (same chunking as fixture streams).
fn text_chunks_stream(
    delay_ms: Option<Duration>,
    chunks: Vec<String>,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>> {
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

impl Provider for MockProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let delay_ms = self.random_delay();
        let last = messages.last();
        let user_input = last_user_content(&messages).unwrap_or_default();

        // Swarm orchestration markers (runie-patterns swarm pattern):
        // deterministic canned responses, no input dependence. Worker prompts
        // carry no marker and fall through to the fixture/echo path.
        if let Some(chunks) = swarm_marker_response(&user_input) {
            return text_chunks_stream(delay_ms, chunks);
        }

        // MiniMax-shaped reasoning+tool scenario (regression fixture).
        if user_input.contains("think tool") {
            return thinking_tool_stream(delay_ms, is_after_tool_result(last));
        }

        // Check for completion after tool result
        if is_after_tool_result(last) {
            let chunks = fixtures::done();
            return Box::pin(async_stream::stream! {
                for chunk_text in chunks {
                    if let Some(d) = delay_ms {
                        tokio::time::sleep(d).await;
                    }
                    yield Ok(ProviderEvent::TextDelta(chunk_text));
                }
                yield Ok(ProviderEvent::Finish { reason: StopReason::Stop });
            });
        }

        // Use explicit fixture or auto-detect
        let fixture = self.fixture.clone().or_else(|| detect_fixture(&user_input));
        let chunks = response_from_fixture(fixture, &user_input, self.echo_fallback);

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
            return Box::pin(native_tool_stream(self.random_delay()));
        }
        self.generate(messages)
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata::new()
            .with_streaming(true)
            .with_supports_tools(true)
            .with_retry_config(crate::RetryConfig::no_retry())
    }
}

fn native_tool_stream(
    delay_ms: Option<Duration>,
) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + 'static>> {
    Box::pin(async_stream::stream! {
        yield Ok(ProviderEvent::TextDelta("I'll run a command.\n".into()));
        if let Some(d) = delay_ms {
            tokio::time::sleep(d).await;
        }
        yield Ok(ProviderEvent::ToolCallStart {
            id: "call_1".into(),
            name: "bash".into(),
        });
        yield Ok(ProviderEvent::ToolCallInputDelta {
            id: "call_1".into(),
            delta: r#"{"command":"echo hi"}"#.into(),
        });
        yield Ok(ProviderEvent::ToolCallEnd { id: "call_1".into() });
        yield Ok(ProviderEvent::Finish { reason: StopReason::ToolCalls });
    })
}

// ─── MockStreamingProvider ────────────────────────────────────────────────────

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

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata::new()
            .with_streaming(true)
            .with_supports_tools(false)
            .with_retry_config(crate::RetryConfig::no_retry())
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

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_list_dir_contains_tool_call() {
        let f = fixtures::list_dir();
        assert!(f.tool_call.is_some());
        let (name, _) = f.tool_call.unwrap();
        assert_eq!(name, "list_dir");
    }

    #[test]
    fn fixture_read_file_contains_tool_call() {
        let f = fixtures::read_file();
        assert!(f.tool_call.is_some());
        let (name, _) = f.tool_call.unwrap();
        assert_eq!(name, "read_file");
    }

    #[test]
    fn detect_fixture_finds_list_files() {
        let f = detect_fixture("list files").unwrap();
        assert_eq!(f.tool_call.as_ref().unwrap().0, "list_dir");
    }

    #[test]
    fn detect_fixture_finds_read() {
        let f = detect_fixture("read the file").unwrap();
        assert_eq!(f.tool_call.as_ref().unwrap().0, "read_file");
    }

    #[test]
    fn detect_fixture_finds_write() {
        let f = detect_fixture("write something").unwrap();
        assert_eq!(f.tool_call.as_ref().unwrap().0, "write_file");
    }

    #[test]
    fn detect_fixture_finds_edit() {
        let f = detect_fixture("edit the code").unwrap();
        assert_eq!(f.tool_call.as_ref().unwrap().0, "edit_file");
    }

    #[test]
    fn detect_fixture_finds_bash() {
        let f = detect_fixture("run a command").unwrap();
        assert_eq!(f.tool_call.as_ref().unwrap().0, "bash");
    }

    #[test]
    fn detect_fixture_finds_markup() {
        let f = detect_fixture("use markup format").unwrap();
        // markup uses list_dir fixture
        assert_eq!(f.tool_call.as_ref().unwrap().0, "list_dir");
    }

    #[test]
    fn detect_fixture_returns_none_for_unknown() {
        assert!(detect_fixture("hello world").is_none());
    }

    #[test]
    fn echo_preserves_exact_input() {
        assert_eq!(
            fixtures::echo("Hello World"),
            vec!["Hello World\n".to_owned()]
        );
        assert_eq!(
            fixtures::echo("  spaced  "),
            vec!["  spaced  \n".to_owned()]
        );
        assert_eq!(
            fixtures::echo("multi\nline\ttext"),
            vec!["multi\nline\ttext\n".to_owned()]
        );
        assert!(fixtures::echo("").is_empty());
    }

    #[test]
    fn mock_provider_builder_creates_list_dir_fixture() {
        let provider = MockProviderBuilder::new().list_dir().build();
        let chunks =
            response_from_fixture(provider.fixture.clone(), "hello", provider.echo_fallback);
        assert_eq!(
            chunks,
            vec![
                "I'll list the files in the current directory.\n".to_owned(),
                "TOOL:list_dir:.".to_owned()
            ]
        );
    }

    #[test]
    fn is_after_tool_result_detects_tool_message() {
        let tool_msg = ChatMessage::tool("file content".to_string());
        assert!(is_after_tool_result(Some(&tool_msg)));

        let user_msg = ChatMessage::user("hello".to_string());
        assert!(!is_after_tool_result(Some(&user_msg)));

        assert!(!is_after_tool_result(None));
    }

    #[test]
    fn echo_fixture_returns_full_input_with_trailing_newline() {
        // The echo fixture should return the full user input with trailing newline
        let chunks = fixtures::echo("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world\n");
    }

    #[test]
    fn response_from_fixture_uses_echo_when_no_fixture_matches() {
        let chunks = response_from_fixture(None, "hello world", true);
        assert_eq!(chunks, vec!["hello world\n"]);
    }

    #[test]
    fn detect_fixture_returns_none_for_unknown_input() {
        // A truly unknown input should NOT match any fixture
        let result = detect_fixture("xyzabc123 what is 2+2");
        assert!(
            result.is_none(),
            "Unexpected fixture detected: {:?}",
            result
        );
    }

    /// Collect all streamed text from a mock `generate` call.
    async fn collect_text(provider: &MockProvider, input: &str) -> String {
        use futures::StreamExt;
        let messages = vec![ChatMessage::user(input.to_string())];
        let mut stream = provider.generate(messages);
        let mut text = String::new();
        while let Some(event) = stream.next().await {
            if let Ok(ProviderEvent::TextDelta(chunk)) = event {
                text.push_str(&chunk);
            }
        }
        text
    }

    #[tokio::test]
    async fn swarm_plan_marker_streams_task_array() {
        let provider = MockProviderBuilder::new().build();
        let text = collect_text(&provider, "[swarm-plan parallel]\nTask: build a feature").await;
        assert_eq!(
            text,
            "[\"Summarize the task\", \"Draft an implementation outline\"]"
        );
    }

    #[tokio::test]
    async fn swarm_synthesize_marker_streams_completion() {
        let provider = MockProviderBuilder::new().build();
        let text = collect_text(&provider, "[swarm-synthesize]\nOriginal task: x").await;
        assert_eq!(text, "Swarm complete: all workers finished successfully.");
    }

    #[tokio::test]
    async fn worker_prompt_falls_through_to_echo() {
        let provider = MockProviderBuilder::new().build();
        let text = collect_text(&provider, "regular worker prompt").await;
        assert_eq!(text, "regular worker prompt\n");
    }

    #[tokio::test]
    async fn swarm_dag_plan_marker_streams_object_array() {
        let provider = MockProviderBuilder::new().build();
        let text = collect_text(&provider, "[swarm-plan dag]\nTask: build a feature").await;
        assert!(text.starts_with("[{"));
        assert!(text.contains("\"deps\""));
    }

    #[tokio::test]
    async fn improve_markers_stream_draft_revision_and_approval() {
        let provider = MockProviderBuilder::new().build();
        let draft = collect_text(&provider, "[improve-generate]\nTask: x").await;
        assert!(draft.starts_with("Draft:"));
        let revised = collect_text(&provider, "[improve-revise]\nTask: x").await;
        assert!(revised.starts_with("Revised draft"));
        let review = collect_text(&provider, "[improve-review]\nTask: x").await;
        assert_eq!(review, "APPROVED");
    }
}
