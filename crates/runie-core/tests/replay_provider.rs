//! Runtime-discovered replay tests. Each `.sse` trace owns a sibling `.sse.yaml`.

#![allow(
    dead_code,
    clippy::too_many_lines,
    reason = "the replay matrix intentionally retains metadata and one end-to-end fixture loop"
)]

mod common;

use futures::StreamExt;
use parking_lot::Mutex;
use runie_core::{
    provider::stream_fn::StreamError,
    provider::{ReplayHttpActor, ReplayProvider, StreamFn},
    types::{
        AgentContext, AgentEvent, AgentMessage, AssistantContent, AssistantMessage,
        AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, Usage, UserContent,
        UserMessage,
    },
};

struct StartupErrorStream;

#[async_trait::async_trait]
impl StreamFn for StartupErrorStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<runie_core::types::SimpleStreamOptions>,
    ) -> Result<runie_core::provider::stream_fn::AssistantMessageEventStream, StreamError> {
        Err(StreamError::Api("upstream unavailable".into()))
    }
}

struct AbortStream {
    started: tokio::sync::watch::Sender<bool>,
    release: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
}

#[async_trait::async_trait]
impl StreamFn for AbortStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<runie_core::provider::stream_fn::AssistantMessageEventStream, StreamError> {
        let release = self.release.lock().take();
        let _ = self.started.send(true);
        let Some(release) = release else {
            return Err(StreamError::Api("missing abort release channel".into()));
        };
        let stream = futures::stream::unfold((0u8, release), |(state, mut release)| async move {
            if state == 0 {
                return Some((
                    AssistantMessageEvent::TextDelta {
                        index: 0,
                        delta: "partial".into(),
                        partial: AssistantMessage::default(),
                    },
                    (1, release),
                ));
            }
            let _ = release.wait_for(|ready| *ready).await;
            Some((
                AssistantMessageEvent::Done {
                    stop_reason: StopReason::Stop,
                    usage: Usage::default(),
                    message: None,
                },
                (2, release),
            ))
        });
        Ok(Box::pin(stream))
    }
}
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Deserialize)]
struct Expectation {
    trace_id: String,
    conversation: String,
    turn: u32,
    #[serde(default)]
    next_trace: Option<String>,
    #[serde(default)]
    previous_trace: Option<String>,
    outcome: String,
    transport: Transport,
    request: Request,
    provider: ProviderExpectation,
    payload: PayloadExpectation,
    core: CoreExpectation,
    state: StateExpectation,
    #[serde(default)]
    tools: Vec<ToolExpectation>,
    #[serde(default)]
    error: Option<ErrorExpectation>,
}

#[derive(Debug, Deserialize)]
struct Transport {
    status: u16,
    sse: bool,
}
#[derive(Debug, Deserialize)]
struct Request {
    model: String,
}
#[derive(Debug, Deserialize)]
struct ProviderExpectation {
    expected_events: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct PayloadExpectation {
    non_empty_text: bool,
}
#[derive(Debug, Deserialize)]
struct CoreExpectation {
    required_events: Vec<String>,
    #[serde(default)]
    ordered_events: Vec<String>,
    #[serde(default)]
    exact_events: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct StateExpectation {
    assistant_messages: usize,
    #[serde(default)]
    is_streaming: Option<bool>,
    #[serde(default)]
    pending_tool_calls: Option<usize>,
    #[serde(default)]
    error_message: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ToolExpectation {
    name: String,
    required_events: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct ErrorExpectation {
    kind: String,
    #[serde(default)]
    message_contains: Option<String>,
}

/// Declarative assertion DSL for actor-owned state projections. Trace YAML
/// supplies the values; branches only provide the snapshot and diagnostics
/// label.
macro_rules! assert_declared_state {
    ($state:expr, $expectation:expr, $trace:expr, $label:literal) => {{
        let state = $state;
        if let Some(expected) = $expectation.state.is_streaming {
            assert_eq!(
                state.is_streaming,
                expected,
                concat!("{} ", $label, " is_streaming"),
                $trace.display()
            );
        }
        if let Some(expected) = $expectation.state.pending_tool_calls {
            assert_eq!(
                state.pending_tool_calls.len(),
                expected,
                concat!("{} ", $label, " pending tools"),
                $trace.display()
            );
        }
        if let Some(expected) = &$expectation.state.error_message {
            assert_eq!(
                state.error_message.as_deref(),
                Some(expected.as_str()),
                concat!("{} ", $label, " error"),
                $trace.display()
            );
        }
    }};
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces")
}

fn event_name(event: &AssistantMessageEvent) -> &'static str {
    runie_core::assistant_event_kind!(event)
}

fn assert_event_order(names: &[&str], expected: &[&str], trace: &Path) {
    let mut cursor = 0;
    for wanted in expected {
        let Some(relative) = names[cursor..].iter().position(|name| name == wanted) else {
            panic!(
                "{} missing ordered event {wanted}; observed {names:?}",
                trace.display()
            );
        };
        cursor += relative + 1;
    }
}

fn assert_declared_event_order(names: &[&str], expected: &[String], trace: &Path) {
    let mut cursor = 0;
    for wanted in expected {
        let Some(relative) = names[cursor..].iter().position(|name| *name == wanted) else {
            panic!(
                "{} missing declared ordered event {wanted}; observed {names:?}",
                trace.display()
            );
        };
        cursor += relative + 1;
    }
}

fn expand_exact_events(expected: &[String], trace: &Path) -> Vec<String> {
    let mut expanded = Vec::new();
    for item in expected {
        let Some((name, count)) = item.rsplit_once('*') else {
            expanded.push(item.clone());
            continue;
        };
        let Ok(count) = count.parse::<usize>() else {
            expanded.push(item.clone());
            continue;
        };
        assert!(
            !name.is_empty(),
            "{} has an empty exact event name",
            trace.display()
        );
        expanded.extend(std::iter::repeat_n(name.to_owned(), count));
    }
    expanded
}

#[cfg(test)]
mod exact_event_dsl_tests {
    use super::expand_exact_events;
    use std::path::Path;

    #[test]
    fn repetition_tokens_expand_without_changing_literal_events() {
        let trace = Path::new("dsl-fixture");
        let actual = expand_exact_events(
            &[
                "AgentStart".into(),
                "MessageUpdate*3".into(),
                "AgentEnd".into(),
            ],
            trace,
        );
        assert_eq!(
            actual,
            vec![
                "AgentStart".to_owned(),
                "MessageUpdate".to_owned(),
                "MessageUpdate".to_owned(),
                "MessageUpdate".to_owned(),
                "AgentEnd".to_owned(),
            ]
        );
    }
}

struct DeclaredTool {
    name: String,
}

#[async_trait::async_trait]
impl runie_core::types::AgentTool for DeclaredTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        "trace fixture tool"
    }
    fn description(&self) -> &str {
        "Deterministic tool used by replay contracts."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<runie_core::types::AgentToolResult, String> {
        Ok(runie_core::types::AgentToolResult {
            content: vec![runie_core::types::ToolResultContent::Text {
                text: "trace tool result".into(),
            }],
            ..Default::default()
        })
    }
}

fn sidecars(dir: &Path, result: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sidecars(&path, result);
        } else if path.extension().and_then(|x| x.to_str()) == Some("yaml") {
            result.push(path);
        }
    }
}

#[path = "support/replay_provider_matrix.rs"]
mod replay_provider_matrix;
