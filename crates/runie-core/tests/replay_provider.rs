//! Runtime-discovered replay tests. Each `.sse` trace owns a sibling `.sse.yaml`.

mod common;

use futures::StreamExt;
use runie_core::{
    provider::{HttpActor, ReplayHttpActor, ReplayProvider, StreamFn},
    types::{
        AgentContext, AgentMessage, AssistantContent, AssistantMessageEvent, Model, UserContent,
        UserMessage,
    },
};
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
}
#[derive(Debug, Deserialize)]
struct StateExpectation {
    assistant_messages: usize,
}
#[derive(Debug, Deserialize)]
struct ToolExpectation {
    name: String,
    required_events: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct ErrorExpectation {
    kind: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces")
}

fn event_name(event: &AssistantMessageEvent) -> &'static str {
    match event {
        AssistantMessageEvent::Start => "Start",
        AssistantMessageEvent::TextStart { .. } => "TextStart",
        AssistantMessageEvent::TextDelta { .. } => "TextDelta",
        AssistantMessageEvent::TextEnd { .. } => "TextEnd",
        AssistantMessageEvent::ThinkingStart { .. } => "ThinkingStart",
        AssistantMessageEvent::ThinkingDelta { .. } => "ThinkingDelta",
        AssistantMessageEvent::ThinkingEnd { .. } => "ThinkingEnd",
        AssistantMessageEvent::ToolCallStart { .. } => "ToolCallStart",
        AssistantMessageEvent::ToolCallDelta { .. } => "ToolCallDelta",
        AssistantMessageEvent::ToolCallEnd { .. } => "ToolCallEnd",
        AssistantMessageEvent::Done { .. } => "Done",
        AssistantMessageEvent::Error { .. } => "Error",
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
        181,
        "every SSE trace must have one sidecar YAML"
    );

    for yaml_path in yaml_files {
        let trace_path = PathBuf::from(yaml_path.to_string_lossy().trim_end_matches(".yaml"));
        let expectation: Expectation =
            serde_yaml::from_str(&std::fs::read_to_string(&yaml_path).unwrap()).unwrap();
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
        let replay = match ReplayHttpActor::from_sse(&trace_path) {
            Ok(http) => ReplayProvider::from_http(Arc::new(http)).await,
            Err(error) => Err(error),
        };
        if expectation.outcome == "error" {
            assert_eq!(
                expectation.error.as_ref().map(|e| e.kind.as_str()),
                Some("provider_decode")
            );
            assert!(replay.is_err(), "expected error: {}", trace_path.display());
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
        // With auto-continue (p05), a trace that requests a tool call yields a
        // follow-up assistant turn (the replay provider's terminating ack), so
        // the assistant count is 2 for tool traces and 1 otherwise.
        let has_tool_call = events.iter().any(|e| *e == "ToolCallDelta");
        let expected_assistants = if has_tool_call { 2 } else { 1 };
        let actual_assistants = output
            .iter()
            .filter(|m| matches!(m, AgentMessage::Assistant(_)))
            .count();
        assert_eq!(
            actual_assistants,
            expected_assistants,
            "{} assistant count",
            trace_path.display()
        );
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        let core_events = test.events.lock();
        let core_names: Vec<_> = core_events
            .iter()
            .map(|event| match event {
                runie_core::types::AgentEvent::AgentStart => "AgentStart",
                runie_core::types::AgentEvent::TurnStart => "TurnStart",
                runie_core::types::AgentEvent::MessageStart { .. } => "MessageStart",
                runie_core::types::AgentEvent::MessageEnd { .. } => "MessageEnd",
                runie_core::types::AgentEvent::MessageUpdate { .. } => "MessageUpdate",
                runie_core::types::AgentEvent::TurnEnd { .. } => "TurnEnd",
                runie_core::types::AgentEvent::AgentEnd { .. } => "AgentEnd",
                runie_core::types::AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
                runie_core::types::AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
                runie_core::types::AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
            })
            .collect();
        for expected in &expectation.core.required_events {
            assert!(
                core_names.contains(&expected.as_str()),
                "{} missing core event {expected}",
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
        }
    }
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
