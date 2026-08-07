//! `StreamFn` trait — abstract LLM streaming interface.
//!
//! Adapters implement this for concrete providers (Anthropic, OpenAI,
//! Bedrock, etc.). The agent loop calls `stream` exactly once per assistant
//! turn; events arrive on the returned stream.

use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use futures::Stream;

use crate::types::{AssistantMessageEvent, Model, SimpleStreamOptions};

pub type AssistantMessageEventStream = Pin<Box<dyn Stream<Item = AssistantMessageEvent> + Send>>;

/// Module-level default stream fn singleton (pi `stream-fn.ts:15`).
static DEFAULT_STREAM_FN: OnceLock<Arc<dyn StreamFn>> = OnceLock::new();

/// Set the module-level default stream fn.
pub fn set_default_stream_fn(f: Arc<dyn StreamFn>) {
    let _ = DEFAULT_STREAM_FN.set(f);
}

/// Get the module-level default stream fn, or a pi-matched error if unset.
pub fn get_default_stream_fn() -> Result<Arc<dyn StreamFn>, StreamError> {
    DEFAULT_STREAM_FN
        .get()
        .cloned()
        .ok_or_else(|| StreamError::Api(
            "No default stream function configured. Pass streamFn explicitly or call set_default_stream_fn()."
                .into(),
        ))
}

/// Salvage-parse a streaming JSON fragment for tool-call arguments (pi
/// `parseStreamingJson`, proxy.ts:310). Tries a full parse first; if that
/// fails, closes any open string and matching brackets/braces to extract the
/// best-effort value.
pub fn parse_streaming_json(input: &str) -> Option<serde_json::Value> {
    if let Ok(v) = serde_json::from_str(input) {
        return Some(v);
    }
    serde_json::from_str(&close_streaming_fragment(input)).ok()
}

fn close_streaming_fragment(input: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut stack: Vec<char> = Vec::new();
    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                stack.push(ch);
                out.push(ch);
            }
            '}' => {
                stack.pop();
                out.push(ch);
            }
            ']' => {
                stack.pop();
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    finish_streaming_fragment(out, in_string, stack)
}

fn finish_streaming_fragment(mut out: String, in_string: bool, mut stack: Vec<char>) -> String {
    if in_string {
        out.push('"');
    }
    while let Some(open) = stack.pop() {
        out.push(if open == '{' { '}' } else { ']' });
    }
    out
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum StreamError {
    #[error("network: {0}")]
    Network(String),
    #[error("api: {0}")]
    Api(String),
    #[error("provider ({status:?}): {message}")]
    Provider {
        message: String,
        status: Option<u16>,
        headers: std::collections::HashMap<String, String>,
    },
    #[error("aborted")]
    Aborted,
    #[error("invalid: {0}")]
    Invalid(String),
}

#[async_trait::async_trait]
pub trait StreamFn: Send + Sync + 'static {
    async fn stream(
        &self,
        model: &Model,
        context: &crate::types::AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFn;
    #[async_trait::async_trait]
    impl StreamFn for TestFn {
        async fn stream(
            &self,
            _model: &Model,
            _context: &crate::types::AgentContext,
            _options: Option<SimpleStreamOptions>,
        ) -> Result<AssistantMessageEventStream, StreamError> {
            use futures::stream;
            Ok(Box::pin(stream::empty()))
        }
    }

    #[tokio::test]
    async fn trait_object_works() {
        let _f: std::sync::Arc<dyn StreamFn> = std::sync::Arc::new(TestFn);
    }

    #[test]
    fn default_stream_fn_unset_returns_pi_error() {
        // The test binary may have set it elsewhere in the same process; only
        // assert when it is unset.
        if let Err(e) = get_default_stream_fn() {
            assert!(
                e.to_string()
                    .contains("No default stream function configured"),
                "error should match pi"
            );
        }
    }

    #[test]
    fn parse_streaming_json_full_and_salvage() {
        // Full object parses.
        assert_eq!(
            parse_streaming_json(r#"{"a":1,"b":"x"}"#),
            Some(serde_json::json!({"a":1,"b":"x"}))
        );
        // Truncated string is closed.
        assert_eq!(
            parse_streaming_json(r#"{"a":"unterminated"#),
            Some(serde_json::json!({"a":"unterminated"}))
        );
        // Open object is closed.
        assert_eq!(
            parse_streaming_json(r#"{"a":1,"b":{"c":2"#),
            Some(serde_json::json!({"a":1,"b":{"c":2}}))
        );
        // Garbage yields None.
        assert_eq!(parse_streaming_json("not json at all"), None);
    }
}
