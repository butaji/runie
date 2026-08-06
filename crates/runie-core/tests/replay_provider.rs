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
    provider::{HttpActor, ReplayHttpActor, ReplayProvider, StreamFn},
    types::{
        AgentContext, AgentMessage, AssistantContent, AssistantMessageEvent, Model,
        SimpleStreamOptions, StopReason, Usage, UserContent, UserMessage,
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
                        delta: "partial".into(),
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

#[tokio::test]
async fn every_trace_uses_its_yaml_expectations_and_runs_through_core() {
    let base = root();
    let mut yaml_files = Vec::new();
    sidecars(&base, &mut yaml_files);
    yaml_files.sort();
    assert_eq!(
        yaml_files.len(),
        183,
        "every SSE trace must have one sidecar YAML"
    );

    let mut exact_fixture_count = 0usize;
    let mut decode_error_fixture_count = 0usize;
    for yaml_path in yaml_files {
        let trace_path = PathBuf::from(yaml_path.to_string_lossy().trim_end_matches(".yaml"));
        let expectation: Expectation =
            serde_yaml::from_str(&std::fs::read_to_string(&yaml_path).unwrap()).unwrap();
        if !expectation.core.exact_events.is_empty() {
            exact_fixture_count += 1;
        }
        if expectation.turn == 1 {
            if let Some(next) = &expectation.next_trace {
                assert!(
                    root().join(next).exists(),
                    "missing linked next trace {next}"
                );
            }
        }
        if expectation.turn > 1 {
            if let Some(previous) = &expectation.previous_trace {
                assert!(
                    root().join(previous).exists(),
                    "missing linked previous trace {previous}"
                );
            }
        }
        assert_eq!(expectation.transport.status, 200);
        assert!(expectation.transport.sse);
        assert!(!expectation.request.model.is_empty());
        if expectation.outcome == "startup_error" {
            let test = common::TestLoopBuilder::new(Arc::new(StartupErrorStream)).build();
            let output = test
                .actor
                .prompt(
                    vec![AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text {
                            text: "startup error replay".into(),
                        }],
                        timestamp: 1,
                    })],
                    AgentContext::default(),
                )
                .await
                .unwrap();
            assert!(output.iter().any(|message| matches!(
                message,
                AgentMessage::Assistant(assistant)
                    if assistant.error_message.as_deref() == Some("api: upstream unavailable")
            )));
            if let Some(error) = &expectation.error {
                assert_eq!(error.kind, "startup");
                if let Some(needle) = &error.message_contains {
                    assert!(
                        "api: upstream unavailable".contains(needle),
                        "{} startup error payload missing {:?}",
                        trace_path.display(),
                        needle
                    );
                }
            }
            assert_declared_state!(test.state.snapshot(), expectation, trace_path, "startup");
            let core_events = test.events.lock();
            let core_names: Vec<_> = core_events
                .iter()
                .map(|event| runie_core::agent_event_kind!(event))
                .collect();
            let exact_events = expand_exact_events(&expectation.core.exact_events, &trace_path);
            assert_eq!(
                core_names,
                exact_events.iter().map(String::as_str).collect::<Vec<_>>(),
                "{} exact startup-error sequence",
                trace_path.display()
            );
            continue;
        }
        if expectation.outcome == "abort" {
            let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
            let (release_tx, release_rx) = tokio::sync::watch::channel(false);
            let stream = Arc::new(AbortStream {
                started: started_tx,
                release: Mutex::new(Some(release_rx)),
            });
            let test = common::TestLoopBuilder::new(stream)
                .tool(common::echo_tool())
                .build();
            let actor = test.actor.clone();
            let run = tokio::spawn(async move {
                actor
                    .prompt(
                        vec![AgentMessage::User(UserMessage {
                            content: vec![UserContent::Text {
                                text: "abort".into(),
                            }],
                            timestamp: 1,
                        })],
                        AgentContext::default(),
                    )
                    .await
            });
            while !*started_rx.borrow() {
                let _ = started_rx.changed().await;
            }
            test.actor.abort();
            let _ = release_tx.send(true);
            let output = run.await.unwrap().unwrap();
            assert!(output.iter().any(|message| matches!(
                message,
                AgentMessage::Assistant(assistant)
                    if assistant.stop_reason == Some(StopReason::Aborted)
            )));
            assert_eq!(
                test.state.snapshot().error_message.as_deref(),
                Some("aborted")
            );
            assert_declared_state!(test.state.snapshot(), expectation, trace_path, "abort");
            let core_events = test.events.lock();
            let core_names: Vec<_> = core_events
                .iter()
                .map(|event| runie_core::agent_event_kind!(event))
                .collect();
            let exact_events = expand_exact_events(&expectation.core.exact_events, &trace_path);
            assert_eq!(
                core_names,
                exact_events.iter().map(String::as_str).collect::<Vec<_>>(),
                "{} exact abort sequence",
                trace_path.display()
            );
            continue;
        }
        let replay = match ReplayHttpActor::from_sse(&trace_path) {
            Ok(http) => ReplayProvider::from_http(Arc::new(http)).await,
            Err(error) => Err(error),
        };
        if expectation.outcome == "error" {
            decode_error_fixture_count += 1;
            assert_eq!(
                expectation.error.as_ref().map(|e| e.kind.as_str()),
                Some("provider_decode")
            );
            let error = match replay {
                Ok(_) => panic!("provider-decode fixture unexpectedly parsed"),
                Err(error) => error,
            };
            assert!(
                !error.to_string().trim().is_empty(),
                "provider-decode fixture must expose an error payload: {}",
                trace_path.display()
            );
            if let Some(expectation) = &expectation.error {
                if let Some(needle) = &expectation.message_contains {
                    assert!(
                        error.to_string().contains(needle),
                        "{} provider error missing {:?}: {}",
                        trace_path.display(),
                        needle,
                        error
                    );
                }
            }
            continue;
        }

        let provider = Arc::new(replay.unwrap());
        let mut stream = provider
            .stream(&Model::default(), &AgentContext::default(), None)
            .await
            .unwrap();
        let mut events = Vec::new();
        let mut payload_text = String::new();
        while let Some(event) = stream.next().await {
            match &event {
                AssistantMessageEvent::TextDelta { delta }
                | AssistantMessageEvent::ThinkingDelta { delta } => payload_text.push_str(delta),
                _ => {}
            }
            events.push(event_name(&event));
        }
        assert_eq!(
            !payload_text.is_empty(),
            expectation.payload.non_empty_text,
            "{} payload assertion",
            trace_path.display()
        );
        for expected in &expectation.provider.expected_events {
            assert!(
                events.contains(&expected.as_str()),
                "{} missing {expected}",
                trace_path.display()
            );
        }

        // The isolated parse test above consumed the first replay turn; reset
        // so the loop run replays the recorded trace from the start.
        provider.reset_turns();
        let mut builder = common::TestLoopBuilder::new(provider);
        for tool in &expectation.tools {
            builder = builder.tool(Arc::new(DeclaredTool {
                name: tool.name.clone(),
            }));
        }
        let test = builder.build();
        let output = test
            .actor
            .prompt(
                vec![AgentMessage::User(UserMessage {
                    content: vec![UserContent::Text {
                        text: "trace replay".into(),
                    }],
                    timestamp: 1,
                })],
                AgentContext::default(),
            )
            .await
            .unwrap();
        assert!(
            output
                .iter()
                .any(|message| matches!(message, AgentMessage::Assistant(_))),
            "no assistant output: {}",
            trace_path.display()
        );
        for tool in &expectation.tools {
            assert!(output.iter().any(|message| matches!(message,
                AgentMessage::Assistant(assistant) if assistant.content.iter().any(|content|
                    matches!(content, AssistantContent::ToolCall(call) if call.name == tool.name)))),
                "{} missing declared tool call {}", trace_path.display(), tool.name);
        }
        let actual_assistants = output
            .iter()
            .filter(|m| matches!(m, AgentMessage::Assistant(_)))
            .count();
        assert_eq!(
            actual_assistants,
            expectation.state.assistant_messages,
            "{} assistant count",
            trace_path.display()
        );
        let state = test.state.snapshot();
        assert!(
            !state.is_streaming,
            "{} must finish with is_streaming=false",
            trace_path.display()
        );
        assert!(
            state.streaming_message.is_none(),
            "{} must clear streaming_message",
            trace_path.display()
        );
        assert!(
            state.pending_tool_calls.is_empty(),
            "{} must clear pending tool calls",
            trace_path.display()
        );
        assert!(
            state.error_message.is_none(),
            "{} successful replay must not retain an error",
            trace_path.display()
        );
        if let Some(expected) = expectation.state.is_streaming {
            assert_eq!(
                state.is_streaming,
                expected,
                "{} is_streaming",
                trace_path.display()
            );
        }
        if let Some(expected) = expectation.state.pending_tool_calls {
            assert_eq!(
                state.pending_tool_calls.len(),
                expected,
                "{} pending tools",
                trace_path.display()
            );
        }
        if let Some(expected) = &expectation.state.error_message {
            assert_eq!(
                state.error_message.as_deref(),
                Some(expected.as_str()),
                "{} error",
                trace_path.display()
            );
        }
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        let core_events = test.events.lock();
        let core_names: Vec<_> = core_events
            .iter()
            .map(|event| runie_core::agent_event_kind!(event))
            .collect();
        for expected in &expectation.core.required_events {
            assert!(
                core_names.contains(&expected.as_str()),
                "{} missing core event {expected}",
                trace_path.display()
            );
        }
        assert_event_order(
            &core_names,
            &[
                "AgentStart",
                "TurnStart",
                "MessageStart",
                "MessageEnd",
                "AgentEnd",
            ],
            &trace_path,
        );
        if !expectation.core.ordered_events.is_empty() {
            assert_declared_event_order(&core_names, &expectation.core.ordered_events, &trace_path);
        }
        if !expectation.core.exact_events.is_empty() {
            let exact_events = expand_exact_events(&expectation.core.exact_events, &trace_path);
            assert_eq!(
                core_names,
                exact_events.iter().map(String::as_str).collect::<Vec<_>>(),
                "{} exact core event sequence",
                trace_path.display()
            );
        }
        for tool in &expectation.tools {
            for expected in &tool.required_events {
                assert!(
                    core_names.contains(&expected.as_str()),
                    "{} missing tool event {expected}",
                    trace_path.display()
                );
            }
            assert_event_order(
                &core_names,
                &["ToolExecutionStart", "ToolExecutionEnd"],
                &trace_path,
            );
        }
    }
    assert_eq!(
        exact_fixture_count, 159,
        "successful-sidecar exact coverage changed: {exact_fixture_count}"
    );
    assert_eq!(
        decode_error_fixture_count, 24,
        "parity matrix lost provider-decode-error fixtures"
    );
}

#[tokio::test]
async fn replay_http_actor_feeds_provider_before_core_loop() {
    let path = root().join("openai/opencode_go_deepseek_v4_flash_simple.sse");
    let http: Arc<dyn HttpActor> = Arc::new(ReplayHttpActor::from_sse(path).unwrap());
    let provider = Arc::new(ReplayProvider::from_http(http).await.unwrap());
    let test = common::TestLoopBuilder::new(provider).build();
    let output = test
        .actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "http replay".into(),
                }],
                timestamp: 1,
            })],
            AgentContext::default(),
        )
        .await
        .unwrap();
    assert!(output
        .iter()
        .any(|message| matches!(message, AgentMessage::Assistant(_))));
}
