//! Replay captured MiniMax streams through the full agent turn.

use anyhow::Result;
use futures::Stream;
use runie_agent::{run_agent_turn, AgentCommand, PermissionGate};
use runie_core::event::AgentEvent;
use runie_core::llm_event::LLMEvent;
use runie_core::message::ChatMessage;
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_provider::openai::stream::replay_sse;
use runie_core::provider::Provider;
use runie_provider::DynProvider;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/minimax/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    ))
    .unwrap()
}

fn allow_all_gate() -> PermissionGate {
    PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink))
}

fn command(content: &str) -> AgentCommand {
    AgentCommand {
        content: content.to_string(),
        id: "req.0".to_string(),
        provider: "minimax".to_string(),
        model: "MiniMax-M3".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: runie_agent::truncate::TruncationPolicy::default(),
    }
}

struct ReplayProvider {
    fixtures: Vec<String>,
    index: AtomicUsize,
}

impl ReplayProvider {
    fn new(fixtures: Vec<String>) -> Self {
        Self {
            fixtures,
            index: AtomicUsize::new(0),
        }
    }
}

impl Provider for ReplayProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send + '_>> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        let events = self
            .fixtures
            .get(idx)
            .map(|f| replay_sse(f))
            .unwrap_or_default();
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

fn dyn_replay(fixtures: &[&str]) -> DynProvider {
    let provider = ReplayProvider::new(fixtures.iter().map(|f| fixture(f)).collect());
    DynProvider::from_provider(Box::new(provider), "minimax", "MiniMax-M3")
}

fn capture_events() -> (Arc<Mutex<Vec<AgentEvent>>>, runie_agent::stream_response::EmitFn) {
    let events: Arc<Mutex<Vec<AgentEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let emit: runie_agent::stream_response::EmitFn = Arc::new(Mutex::new(move |evt| {
        captured.lock().unwrap().push(evt);
    }));
    (events, emit)
}

#[tokio::test]
async fn m3_list_files_turn_executes_list_dir() {
    let provider = dyn_replay(&["m3_list_files_call.sse", "m3_list_files_final.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &command("list files in the current directory"), emit, 5, allow_all_gate())
        .await
        .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::ToolStart { name, .. } if name == "list_dir"
    )));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}

#[tokio::test]
async fn m3_read_file_turn_executes_read_file() {
    let provider = dyn_replay(&["m3_read_file_call.sse", "m3_read_file_final.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &command("read /tmp/runie_minimax_notes_99892.txt"), emit, 5, allow_all_gate())
        .await
        .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::ToolStart { name, .. } if name == "read_file"
    )));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}

#[tokio::test]
async fn m3_multi_tool_turn_executes_list_dir_and_read_file() {
    let provider = dyn_replay(&[
        "m3_multi_tool_list_dir.sse",
        "m3_multi_tool_readme.sse",
    ]);
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &command("list files and read README.md"), emit, 5, allow_all_gate())
        .await
        .unwrap();

    let events = events.lock().unwrap();
    let tool_names: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolStart { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(tool_names.contains(&"list_dir"));
    assert!(tool_names.contains(&"read_file"));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}

#[tokio::test]
async fn m27_multi_tool_turn_executes_read_file() {
    let provider = dyn_replay(&["m27_multi_tool_readme.sse"]);
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &command("read README.md"), emit, 5, allow_all_gate())
        .await
        .unwrap();

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        AgentEvent::ToolStart { name, .. } if name == "read_file"
    )));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}
