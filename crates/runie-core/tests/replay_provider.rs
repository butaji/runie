use runie_core::{
    provider::{ReplayProvider, StreamFn},
    types::{AgentContext, Model, AssistantMessageEvent},
};
use futures::StreamExt;
use serde::Deserialize;
mod common;

#[derive(Deserialize)]
struct Flow {
    trace: String,
    expected: Vec<String>,
}

#[tokio::test]
async fn replays_historical_openai_trace_without_network() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/traces/openai/opencode_go_deepseek_v4_flash_simple.sse");
    let provider = ReplayProvider::from_sse(path).unwrap();
    let mut stream = provider.stream(&Model::default(), &AgentContext::default(), None).await.unwrap();
    let mut text = String::new();
    let mut done = false;
    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta }
            | AssistantMessageEvent::ThinkingDelta { delta } => text.push_str(&delta),
            AssistantMessageEvent::Done { .. } => done = true,
            _ => {}
        }
    }
    assert!(!text.is_empty());
    assert!(done);
}

#[tokio::test]
async fn replays_anthropic_text_and_openai_tool_trace() {
    for relative in [
        "anthropic/opencode_go_minimax_m3_simple.sse",
        "openai/opencode_go_deepseek_v4_flash_tool.sse",
    ] {
        let path = format!("{}/../../tests/traces/{relative}", env!("CARGO_MANIFEST_DIR"));
        let provider = ReplayProvider::from_sse(path).unwrap();
        let mut stream = provider.stream(&Model::default(), &AgentContext::default(), None).await.unwrap();
        let mut saw_done = false;
        let mut saw_tool = false;
        while let Some(event) = stream.next().await {
            saw_done |= matches!(event, AssistantMessageEvent::Done { .. });
            saw_tool |= matches!(event, AssistantMessageEvent::ToolCallDelta { .. });
        }
        assert!(saw_done);
        if relative.contains("tool") { assert!(saw_tool); }
    }
}

#[test]
fn rejects_non_sse_or_incomplete_trace() {
    let result = ReplayProvider::from_sse("/definitely/not/a/trace.sse");
    assert!(result.is_err());
}

#[test]
fn every_recovered_trace_is_classified_by_the_replay_provider() {
    fn visit(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { visit(&path, files); }
            else if path.extension().and_then(|x| x.to_str()) == Some("sse") { files.push(path); }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces");
    let mut files = Vec::new();
    visit(&root, &mut files);
    assert!(files.len() >= 180, "recovered trace corpus unexpectedly small: {}", files.len());

    let mut replayable = 0;
    let mut expected_failures = 0;
    for path in files {
        match ReplayProvider::from_sse(&path) {
            Ok(_) => replayable += 1,
            Err(_) if path.file_name().unwrap().to_string_lossy().contains("error")
                || path.to_string_lossy().contains("status_")
                || path.file_name().unwrap().to_string_lossy().contains("context_length_exceeded")
                || path.file_name().unwrap().to_string_lossy().contains("invalid_api_key")
                || path.file_name().unwrap().to_string_lossy().contains("model_not_found") => expected_failures += 1,
            Err(error) => panic!("unexpected replay failure for {}: {error}", path.display()),
        }
    }
    assert!(replayable >= 150);
    assert!(expected_failures >= 10);
}

#[tokio::test]
async fn yaml_flows_drive_representative_provider_replays() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces");
    for name in ["openai-tool-flow.yaml", "anthropic-tool-flow.yaml"] {
        let flow: Flow = serde_yaml::from_str(&std::fs::read_to_string(root.join(name)).unwrap()).unwrap();
        let provider = ReplayProvider::from_sse(root.join(&flow.trace)).unwrap();
        let mut stream = provider.stream(&Model::default(), &AgentContext::default(), None).await.unwrap();
        let mut actual = Vec::new();
        while let Some(event) = stream.next().await {
            actual.push(match event {
                AssistantMessageEvent::Start => "Start",
                AssistantMessageEvent::ToolCallDelta { .. } => "ToolCallDelta",
                AssistantMessageEvent::Done { .. } => "Done",
                _ => continue,
            });
        }
        assert_eq!(actual, flow.expected);
    }
}

#[tokio::test]
async fn replay_provider_drives_the_real_core_loop() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/traces/openai/opencode_go_deepseek_v4_flash_simple.sse");
    let provider = std::sync::Arc::new(ReplayProvider::from_sse(path).unwrap());
    let test = common::TestLoopBuilder::new(provider).build();
    let output = test.actor.prompt(
        vec![runie_core::types::AgentMessage::User(runie_core::types::UserMessage {
            content: vec![runie_core::types::UserContent::Text { text: "hello".into() }],
            timestamp: 1,
        })],
        AgentContext::default(),
    ).await.unwrap();
    assert!(output.iter().any(|message| matches!(message, runie_core::types::AgentMessage::Assistant(_))));
}

#[tokio::test]
async fn every_recovered_trace_executes_through_the_core_loop() {
    fn visit(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { visit(&path, files); }
            else if path.extension().and_then(|x| x.to_str()) == Some("sse") { files.push(path); }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/traces");
    let mut files = Vec::new();
    visit(&root, &mut files);
    files.sort();
    assert_eq!(files.len(), 181, "SSE trace corpus changed; update coverage expectations");

    let mut replayed = 0;
    let mut rejected = 0;
    for path in files {
        let filename = path.file_name().unwrap().to_string_lossy();
        match ReplayProvider::from_sse(&path) {
            Ok(provider) => {
                let test = common::TestLoopBuilder::new(std::sync::Arc::new(provider)).build();
                let output = test.actor.prompt(
                    vec![runie_core::types::AgentMessage::User(runie_core::types::UserMessage {
                        content: vec![runie_core::types::UserContent::Text { text: "trace replay".into() }],
                        timestamp: 1,
                    })],
                    AgentContext::default(),
                ).await.unwrap();
                assert!(output.iter().any(|message| matches!(message, runie_core::types::AgentMessage::Assistant(_))),
                    "successful trace produced no assistant message: {}", path.display());
                replayed += 1;
            }
            Err(error) => {
                assert!(filename.contains("error")
                    || filename.contains("status_")
                    || filename.contains("context_length_exceeded")
                    || filename.contains("invalid_api_key")
                    || filename.contains("model_not_found"),
                    "unexpected replay failure for {}: {error}", path.display());
                rejected += 1;
            }
        }
    }
    assert!(replayed >= 150, "too few traces executed: {replayed}");
    assert!(rejected >= 10, "error corpus not exercised: {rejected}");
}
