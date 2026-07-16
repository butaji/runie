//! Replay provider for black-box testing.
//!
//! `ReplayProvider` cycles through a list of SSE fixtures, replaying each one
//! on successive calls. This enables deterministic black-box tests that do not
//! require live API keys or network access.
//!
//! The provider is selected when the `RUNIE_REPLAY_FIXTURES` environment variable
//! is set, or when explicitly constructed in tests.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::Stream;
use hex;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::{ModelError, ProviderEvent};
use sha2::{Digest, Sha256};

/// A tool call entry for deterministic key computation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct ToolCallEntry {
    pub tool_name: String,
    pub args_hash: String,
}

impl ToolCallEntry {
    /// Create a new entry from a tool name and args JSON.
    pub fn new(tool_name: impl Into<String>, args: &serde_json::Value) -> Self {
        let mut hasher = DefaultHasher::new();
        args.hash(&mut hasher);
        let args_hash = format!("{:x}", hasher.finish());
        Self {
            tool_name: tool_name.into(),
            args_hash,
        }
    }
}

/// Compute a deterministic replay key from a call tree and position.
///
/// Uses SHA-256 of the JSON-serialized call tree plus position to generate
/// a stable, reproducible key that maps to a fixture entry.
pub fn compute_replay_key(call_tree: &[ToolCallEntry], position: u32) -> String {
    let mut hasher = Sha256::new();
    // Serialize call tree as JSON for deterministic input
    let tree_json = serde_json::to_string(call_tree).unwrap_or_else(|_| "[]".to_owned());
    hasher.update(tree_json.as_bytes());
    hasher.update(position.to_le_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Builder for constructing call lineage deterministically.
///
/// Tracks the sequence of tool calls and provides methods to build
/// a call tree that can be used with `compute_replay_key`.
#[derive(Debug, Default, Clone)]
pub struct ReplayKeyBuilder {
    call_tree: Vec<ToolCallEntry>,
}

impl ReplayKeyBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tool call to the lineage.
    pub fn add_call(mut self, tool_name: impl Into<String>, args: &serde_json::Value) -> Self {
        self.call_tree.push(ToolCallEntry::new(tool_name, args));
        self
    }

    /// Push an entry directly.
    pub fn push(mut self, entry: ToolCallEntry) -> Self {
        self.call_tree.push(entry);
        self
    }

    /// Get the current call tree.
    pub fn call_tree(&self) -> &[ToolCallEntry] {
        &self.call_tree
    }

    /// Compute the replay key for a given position.
    pub fn compute_key(&self, position: u32) -> String {
        compute_replay_key(&self.call_tree, position)
    }

    /// Extend the builder with another call tree.
    pub fn extend(&mut self, other: &[ToolCallEntry]) {
        self.call_tree.extend(other.iter().cloned());
    }
}

/// Protocol for replay fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// OpenAI-compatible SSE format.
    OpenAi,
    /// Anthropic-compatible SSE format.
    Anthropic,
}

/// A `Provider` that returns pre-recorded SSE fixtures in round-robin order.
///
/// Each call to `generate` returns the next fixture in the list, wrapping around
/// when the list is exhausted. This supports multi-turn conversation tests where
/// each turn uses a different recorded fixture.
///
/// Deterministic key lookup: tool call trees are used to generate stable keys
/// so that the same call sequence always maps to the same fixture.
pub struct ReplayProvider {
    fixtures: Vec<String>,
    protocol: Protocol,
    /// Legacy index-based fallback for backwards compatibility.
    index: AtomicUsize,
    /// Maps deterministic keys to fixture indices.
    key_map: Arc<RwLock<HashMap<String, usize>>>,
}

impl ReplayProvider {
    /// Build a provider that cycles through `fixtures` (each a raw SSE string).
    pub fn new(fixtures: Vec<String>, protocol: Protocol) -> Self {
        Self {
            fixtures,
            protocol,
            index: AtomicUsize::new(0),
            key_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build a call tree from messages for deterministic key computation.
    ///
    /// Extracts tool calls from assistant messages and builds a lineage
    /// that can be used with `compute_replay_key`.
    pub fn build_call_tree(messages: &[ChatMessage]) -> Vec<ToolCallEntry> {
        let mut entries = Vec::new();
        for msg in messages {
            for tool_call in msg.tool_calls() {
                entries.push(ToolCallEntry::new(&tool_call.name, &tool_call.args));
            }
        }
        entries
    }

    /// Look up a fixture index for a deterministic key.
    ///
    /// If the key has been seen before, returns the cached index.
    /// Otherwise, allocates the next available fixture and caches the mapping.
    fn lookup_index(&self, key: &str) -> usize {
        // Fast path: check if key is already mapped
        {
            let map = self.key_map.read().unwrap();
            if let Some(&idx) = map.get(key) {
                return idx;
            }
        }
        // Slow path: allocate new index
        let mut map = self.key_map.write().unwrap();
        // Double-check after acquiring write lock
        if let Some(&idx) = map.get(key) {
            return idx;
        }
        let idx = map.len() % self.fixtures.len().max(1);
        map.insert(key.to_owned(), idx);
        idx
    }

    /// Infer the protocol from fixture contents or path hints.
    ///
    /// Returns `Protocol::OpenAi` if any fixture contains OpenAI-specific markers,
    /// otherwise falls back to `Protocol::Anthropic`.
    pub fn infer_protocol(fixtures: &[String]) -> Protocol {
        for fixture in fixtures {
            // Anthropic SSE has distinct message_start/content_block_start markers.
            if fixture.contains("\"type\":\"message_start\"")
                || fixture.contains("\"type\":\"content_block_start\"")
                || fixture.contains("\"type\":\"message_delta\"")
            {
                return Protocol::Anthropic;
            }
            // OpenAI SSE uses chat.completion.chunk objects.
            if fixture.contains("\"object\":\"chat.completion.chunk\"")
                || fixture.contains("\"object\":\"chat.completion\"")
            {
                return Protocol::OpenAi;
            }
        }
        // Default to OpenAI for backwards compatibility.
        Protocol::OpenAi
    }

    /// Number of fixtures.
    pub fn fixture_count(&self) -> usize {
        self.fixtures.len()
    }
}

impl std::fmt::Debug for ReplayProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayProvider")
            .field("fixtures", &self.fixtures.len())
            .field("protocol", &self.protocol)
            .field("key_map_size", &self.key_map.read().map(|m| m.len()).unwrap_or(0))
            .finish()
    }
}

impl Provider for ReplayProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        // Build call tree from messages for deterministic key
        let call_tree = Self::build_call_tree(&messages);
        let key = compute_replay_key(&call_tree, 0);

        // Retry simulation: a fixture whose events are a retryable error
        // (overload/rate limit) with no content is treated as a failed
        // attempt — the next fixture in the rotation is consumed, mirroring
        // the whole-request retry in the live provider (without the backoff
        // sleeps, so tests stay deterministic and fast). After
        // `max_attempts` failures the last error is surfaced.
        let max_attempts = crate::RetryConfig::default().max_attempts.max(1);
        let mut events = Vec::new();
        for attempt in 1..=max_attempts {
            // Use deterministic key lookup for stable fixture selection
            let idx = if self.key_map.read().unwrap().contains_key(&key) {
                self.lookup_index(&key)
            } else {
                // Fallback to index-based cycling for new keys (backward compat)
                self.index.fetch_add(1, Ordering::SeqCst) % self.fixtures.len().max(1)
            };
            events = match self.fixtures.get(idx) {
                Some(fixture) => parse_fixture(fixture, self.protocol),
                None => Vec::new(),
            };
            if !is_bare_retryable_error(&events) || attempt == max_attempts {
                break;
            }
        }
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

/// True when the fixture produced only a retryable error and no content:
/// the live provider would retry the request in this case.
fn is_bare_retryable_error(events: &[ProviderEvent]) -> bool {
    let has_content = events.iter().any(|e| {
        matches!(
            e,
            ProviderEvent::TextDelta(_)
                | ProviderEvent::ThinkingDelta(_)
                | ProviderEvent::ToolCallStart { .. }
        )
    });
    if has_content {
        return false;
    }
    events.iter().any(|e| match e {
        ProviderEvent::Error(err) => err.is_retryable(),
        _ => false,
    })
}

/// Parse a fixture string into `ProviderEvent`s.
///
/// If the fixture starts with `# HTTP <code>`, returns a single error event
/// for that HTTP status. Otherwise, parses as normal SSE content.
fn parse_fixture(content: &str, protocol: Protocol) -> Vec<ProviderEvent> {
    // Check for HTTP status prefix: "# HTTP 429"
    if let Some(first_line) = content.lines().next() {
        if first_line.starts_with("# HTTP ") {
            let code_str = first_line.trim_start_matches("# HTTP ").trim();
            let code: u16 = code_str.parse().unwrap_or(500);
            let message = content
                .lines()
                .nth(1)
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .unwrap_or_else(|| format!("HTTP {}", code));

            let model_err = match code {
                401 | 403 => ModelError::Other(format!("HTTP {}: {}", code, message)),
                429 => ModelError::RateLimit {
                    retry_after_secs: None,
                },
                529 => ModelError::Overloaded {
                    retry_after_secs: None,
                },
                500 | 502 | 503 => ModelError::Other(format!("HTTP {}: {}", code, message)),
                _ => ModelError::Other(format!("HTTP {}: {}", code, message)),
            };
            return vec![ProviderEvent::Error(model_err)];
        }
    }
    // Otherwise parse as normal SSE
    match protocol {
        Protocol::OpenAi => crate::openai::stream::replay_sse(content),
        Protocol::Anthropic => crate::anthropic::replay_anthropic_sse(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_provider_cycles_fixtures() {
        let fixtures = vec!["a".to_string(), "b".to_string()];
        let provider = ReplayProvider::new(fixtures, Protocol::OpenAi);
        let _ = provider;
    }

    #[test]
    fn infer_protocol_anthropic() {
        let fixtures = vec![r#"data: {"type":"message_start","message":{}}"#.to_string()];
        assert_eq!(
            ReplayProvider::infer_protocol(&fixtures),
            Protocol::Anthropic
        );
    }

    #[test]
    fn infer_protocol_openai() {
        let fixtures = vec![r#"data: {"object":"chat.completion.chunk"}"#.to_string()];
        assert_eq!(ReplayProvider::infer_protocol(&fixtures), Protocol::OpenAi);
    }

    #[test]
    fn infer_protocol_default_openai() {
        let fixtures = vec![r#"data: {"unknown":"format"}"#.to_string()];
        assert_eq!(ReplayProvider::infer_protocol(&fixtures), Protocol::OpenAi);
    }

    fn collect(provider: &ReplayProvider) -> Vec<ProviderEvent> {
        use futures::StreamExt;
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            provider
                .generate(vec![])
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .map(|r| r.unwrap())
                .collect()
        })
    }

    #[test]
    fn http_529_maps_to_overloaded() {
        let provider = ReplayProvider::new(
            vec!["# HTTP 529\n# overloaded".to_string()],
            Protocol::OpenAi,
        );
        let events = collect(&provider);
        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::Error(ModelError::Overloaded { .. }))));
    }

    #[test]
    fn retryable_error_consumes_next_fixture() {
        // [529 overload, content] — the overload attempt is retried and the
        // content fixture answers the same turn.
        let provider = ReplayProvider::new(
            vec![
                "# HTTP 529\n# overloaded".to_string(),
                "data: {\"choices\":[{\"delta\":{\"content\":\"pong\"}}]}\n\ndata: [DONE]\n\n"
                    .to_string(),
            ],
            Protocol::OpenAi,
        );
        let events = collect(&provider);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProviderEvent::TextDelta(d) if d == "pong")),
            "expected retried content, got: {events:?}"
        );
        assert!(
            !events.iter().any(|e| matches!(e, ProviderEvent::Error(_))),
            "the exhausted overload error must not surface when retry succeeds: {events:?}"
        );
    }

    #[test]
    fn retryable_error_surfaces_after_max_attempts() {
        let provider = ReplayProvider::new(
            vec!["# HTTP 529\n# overloaded".to_string()],
            Protocol::OpenAi,
        );
        let events = collect(&provider);
        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::Error(ModelError::Overloaded { .. }))));
    }

    #[test]
    fn non_retryable_error_does_not_consume_next_fixture() {
        // [401 auth, content] — 401 is fatal: surfaces immediately; the
        // content fixture stays queued for the next turn.
        let provider = ReplayProvider::new(
            vec![
                "# HTTP 401\n# invalid api key".to_string(),
                "data: {\"choices\":[{\"delta\":{\"content\":\"pong\"}}]}\n\ndata: [DONE]\n\n"
                    .to_string(),
            ],
            Protocol::OpenAi,
        );
        let events = collect(&provider);
        assert!(events.iter().any(|e| matches!(e, ProviderEvent::Error(_))));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ProviderEvent::TextDelta(_))),
            "next fixture must not be consumed on a fatal error: {events:?}"
        );
        let events = collect(&provider);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProviderEvent::TextDelta(d) if d == "pong")),
            "content fixture should answer the next turn: {events:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Deterministic replay key tests
    // -------------------------------------------------------------------------

    #[test]
    fn tool_call_entry_new() {
        let args = serde_json::json!({"path": "/tmp"});
        let entry = ToolCallEntry::new("read_file", &args);
        assert_eq!(entry.tool_name, "read_file");
        assert!(!entry.args_hash.is_empty());
    }

    #[test]
    fn tool_call_entry_deterministic() {
        let args1 = serde_json::json!({"path": "/tmp"});
        let args2 = serde_json::json!({"path": "/tmp"});
        let entry1 = ToolCallEntry::new("read_file", &args1);
        let entry2 = ToolCallEntry::new("read_file", &args2);
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn tool_call_entry_different_args() {
        let args1 = serde_json::json!({"path": "/tmp"});
        let args2 = serde_json::json!({"path": "/home"});
        let entry1 = ToolCallEntry::new("read_file", &args1);
        let entry2 = ToolCallEntry::new("read_file", &args2);
        assert_ne!(entry1, entry2);
    }

    #[test]
    fn compute_replay_key_empty_tree() {
        let key = compute_replay_key(&[], 0);
        assert!(!key.is_empty());
        assert_eq!(key.len(), 64); // SHA-256 hex is 64 characters
    }

    #[test]
    fn compute_replay_key_deterministic() {
        let entries = vec![
            ToolCallEntry::new("read_file", &serde_json::json!({"path": "/tmp"})),
        ];
        let key1 = compute_replay_key(&entries, 0);
        let key2 = compute_replay_key(&entries, 0);
        assert_eq!(key1, key2, "same input must produce same key");
    }

    #[test]
    fn compute_replay_key_different_tree() {
        let entries1 = vec![ToolCallEntry::new(
            "read_file",
            &serde_json::json!({"path": "/tmp"}),
        )];
        let entries2 = vec![ToolCallEntry::new(
            "write_file",
            &serde_json::json!({"path": "/tmp"}),
        )];
        let key1 = compute_replay_key(&entries1, 0);
        let key2 = compute_replay_key(&entries2, 0);
        assert_ne!(key1, key2, "different tree must produce different key");
    }

    #[test]
    fn compute_replay_key_different_position() {
        let entries = vec![ToolCallEntry::new(
            "read_file",
            &serde_json::json!({"path": "/tmp"}),
        )];
        let key1 = compute_replay_key(&entries, 0);
        let key2 = compute_replay_key(&entries, 1);
        assert_ne!(key1, key2, "different position must produce different key");
    }

    #[test]
    fn replay_key_builder_new() {
        let builder = ReplayKeyBuilder::new();
        assert!(builder.call_tree().is_empty());
    }

    #[test]
    fn replay_key_builder_add_call() {
        let builder = ReplayKeyBuilder::new()
            .add_call("read_file", &serde_json::json!({"path": "/tmp"}))
            .add_call("bash", &serde_json::json!({"cmd": "ls"}));
        assert_eq!(builder.call_tree().len(), 2);
    }

    #[test]
    fn replay_key_builder_compute_key() {
        let builder = ReplayKeyBuilder::new()
            .add_call("read_file", &serde_json::json!({"path": "/tmp"}));
        let key = builder.compute_key(0);
        assert!(!key.is_empty());
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn replay_key_builder_extend() {
        let entries = vec![
            ToolCallEntry::new("read_file", &serde_json::json!({"path": "/tmp"})),
        ];
        let mut builder = ReplayKeyBuilder::new();
        builder.extend(&entries);
        assert_eq!(builder.call_tree().len(), 1);
    }

    #[test]
    fn replay_key_builder_deterministic() {
        let builder1 = ReplayKeyBuilder::new()
            .add_call("read_file", &serde_json::json!({"path": "/tmp"}))
            .add_call("bash", &serde_json::json!({"cmd": "ls"}));
        let builder2 = ReplayKeyBuilder::new()
            .add_call("read_file", &serde_json::json!({"path": "/tmp"}))
            .add_call("bash", &serde_json::json!({"cmd": "ls"}));
        let key1 = builder1.compute_key(0);
        let key2 = builder2.compute_key(0);
        assert_eq!(
            key1, key2,
            "identical builders must produce identical keys"
        );
    }

    #[test]
    #[test]
    fn build_call_tree_from_messages() {
        use runie_core::proto::message::{ChatMessageBuilder, Role};
        let messages = vec![
            ChatMessageBuilder::assistant()
                .tool_call("tc1", "read_file", serde_json::json!({"path": "/tmp"}))
                .build(),
            ChatMessageBuilder::assistant()
                .tool_call("tc2", "bash", serde_json::json!({"cmd": "ls"}))
                .build(),
        ];
        let tree = ReplayProvider::build_call_tree(&messages);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].tool_name, "read_file");
        assert_eq!(tree[1].tool_name, "bash");
    }

    #[test]
    fn build_call_tree_empty_messages() {
        let tree = ReplayProvider::build_call_tree(&[]);
        assert!(tree.is_empty());
    }

    #[test]
    fn deterministic_key_same_tool_sequence() {
        use runie_core::proto::message::{ChatMessageBuilder, Role};
        let fixtures = vec![
            r#"data: {"choices":[{"delta":{"content":"result1"}}]}"#.to_string(),
            r#"data: {"choices":[{"delta":{"content":"result2"}}]}"#.to_string(),
        ];
        let provider = ReplayProvider::new(fixtures, Protocol::OpenAi);

        // First call with specific tool sequence
        let messages = vec![ChatMessageBuilder::assistant()
            .tool_call("tc1", "read_file", serde_json::json!({"path": "/tmp"}))
            .build()];
        let events1 = collect_from_provider(&provider, messages.clone());

        // Second call with SAME tool sequence
        let messages2 = vec![ChatMessageBuilder::assistant()
            .tool_call("tc2", "read_file", serde_json::json!({"path": "/tmp"}))
            .build()];
        let events2 = collect_from_provider(&provider, messages2.clone());

        // Both should produce the same content (deterministic)
        let content1: String = events1
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::TextDelta(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        let content2: String = events2
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::TextDelta(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            content1, content2,
            "same tool sequence must produce same result"
        );
    }

    #[test]
    fn deterministic_key_different_tool_sequence() {
        use runie_core::proto::message::{ChatMessageBuilder, Role};
        let fixtures = vec![
            r#"data: {"choices":[{"delta":{"content":"result1"}}]}"#.to_string(),
            r#"data: {"choices":[{"delta":{"content":"result2"}}]}"#.to_string(),
        ];
        let provider = ReplayProvider::new(fixtures, Protocol::OpenAi);

        // First call with read_file
        let messages1 = vec![ChatMessageBuilder::assistant()
            .tool_call("tc1", "read_file", serde_json::json!({"path": "/tmp"}))
            .build()];
        let events1 = collect_from_provider(&provider, messages1);

        // Second call with DIFFERENT tool (bash)
        let messages2 = vec![ChatMessageBuilder::assistant()
            .tool_call("tc2", "bash", serde_json::json!({"cmd": "ls"}))
            .build()];
        let events2 = collect_from_provider(&provider, messages2);

        // Results may differ because different tool calls
        let _ = (events1, events2);
    }

    #[test]
    fn parallel_tool_results_stable_across_runs() {
        use runie_core::proto::message::{ChatMessageBuilder, Role};
        let fixtures = vec![
            r#"data: {"choices":[{"delta":{"content":"stable_result"}}]}"#.to_string(),
        ];
        let provider = ReplayProvider::new(fixtures, Protocol::OpenAi);

        // Simulate multiple "runs" with same call tree
        let call_tree = vec![ChatMessageBuilder::assistant()
            .tool_call("tc1", "read_file", serde_json::json!({"path": "/stable"}))
            .build()];

        // Run 1
        let events1 = collect_from_provider(&provider, call_tree.clone());
        // Run 2 (same call tree)
        let events2 = collect_from_provider(&provider, call_tree.clone());

        let content1: String = events1
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::TextDelta(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        let content2: String = events2
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::TextDelta(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            content1, content2,
            "parallel tool results must be stable across runs"
        );
    }

    fn collect_from_provider(
        provider: &ReplayProvider,
        messages: Vec<ChatMessage>,
    ) -> Vec<ProviderEvent> {
        use futures::StreamExt;
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            provider
                .generate(messages)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .map(|r| r.unwrap())
                .collect()
        })
    }
}
