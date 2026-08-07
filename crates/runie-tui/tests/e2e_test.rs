//! End-to-end test driving the TUI App via TestBackend.
//!
//! Reuses the `MockStreamFn` pattern from `runie-core` tests.

#![allow(
    clippy::too_many_lines,
    reason = "the E2E test keeps actor setup and transcript assertions in one scenario"
)]

use std::path::PathBuf;
use std::sync::Arc;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use runie_core::events::EventBus;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::{ToolExecutorActor, ToolRegistry};
use runie_core::types::{
    AgentContext, AgentMessage, AgentTool, AgentToolResult, AssistantMessage,
    AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, ToolExecutionMode,
    ToolResultContent, Usage, UserContent, UserMessage,
};
use runie_tui::app::App;
use runie_tui::event_renderer::EventRenderer;
use runie_tui::layout::chat_layout;
use runie_tui::widgets::PromptOutcome;
use runie_tui::yaml_runner::{assert_scenario_async, load_scenario, run_scenario};

mod common;
use common::test_model;

static E2E_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct TwoTurnMock;

#[async_trait::async_trait]
impl StreamFn for TwoTurnMock {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        use futures::stream;
        let events = vec![
            AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "Hello".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: " world".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            },
            // After Done, recv returns Err(Closed) and the inner loop exits.
        ];
        Ok(Box::pin(stream::iter(events)))
    }
}

struct PendingMock;

#[async_trait::async_trait]
impl StreamFn for PendingMock {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        std::future::pending().await
    }
}

struct EchoTool;
#[async_trait::async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn label(&self) -> &str {
        "Bash"
    }
    fn description(&self) -> &str {
        "Echoes args"
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: args.to_string(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

fn build_app_with(provider_stream: Arc<dyn StreamFn>) -> App {
    let bus = EventBus::new();
    let state = AgentStateActor::new();
    let steering = SteeringQueueActor::new();
    let follow_up = FollowUpQueueActor::new();
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(EchoTool));
    let tool_executor = ToolExecutorActor::new(Arc::new(reg));
    let provider = ProviderActor::new(provider_stream);
    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: Default::default(),
        abort: None,
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: runie_core::types::QueueMode::OneAtATime,
        follow_up_mode: runie_core::types::QueueMode::OneAtATime,
    };
    let actor = LoopActor::new(deps);
    App::new(actor, bus)
}

fn build_app() -> App {
    build_app_with(Arc::new(TwoTurnMock))
}

#[tokio::test]
async fn prompt_submission_ack_does_not_wait_for_provider() {
    let app = build_app_with(Arc::new(PendingMock));
    let accepted = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        app.handle_prompt_outcome(PromptOutcome::Submitted("pending".into())),
    )
    .await
    .expect("submission mailbox acceptance must not await provider work");
    assert_eq!(accepted.as_deref(), Some("pending"));

    // A provider run already in progress must not block admission of the
    // next prompt. This is the regression that a single sequential mailbox
    // worker would miss.
    let second = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        app.handle_prompt_outcome(PromptOutcome::Submitted("second".into())),
    )
    .await
    .expect("submission mailbox must remain reactive while a run is pending");
    assert_eq!(second.as_deref(), Some("second"));
}

#[tokio::test]
async fn end_to_end_prompt_renders_transcript() {
    let _test_lock = E2E_TEST_LOCK.lock().await;
    let app = build_app();
    eprintln!("[e2e] built app");

    // Spawn the renderer.
    let renderer = EventRenderer::with_actors(
        app.scrollback_actor.clone(),
        app.status_actor.clone(),
        false,
    );
    let rx = app.bus.subscribe();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let handle = tokio::spawn(async move { renderer.run(rx, shutdown_rx).await });
    eprintln!("[e2e] spawned renderer");

    let prompt = vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "hi".into() }],
        timestamp: 1,
    })];
    eprintln!("[e2e] before prompt().await");
    let outcome = app
        .loop_actor
        .prompt(prompt, AgentContext::default())
        .await
        .unwrap();
    eprintln!("[e2e] prompt returned: {} messages", outcome.len());
    assert!(!outcome.is_empty());

    eprintln!("[e2e] before shutdown");
    let _ = shutdown_tx.send(true);
    let _ = handle.await;
    eprintln!("[e2e] after handle.await");
    let mut scrollback = app.scrollback_snapshot();
    drop(app.bus);

    let backend = TestBackend::new(24, 80);
    eprintln!("[e2e] created backend");
    let mut terminal = Terminal::new(backend).unwrap();
    eprintln!("[e2e] created terminal");
    terminal
        .draw(|f| {
            let area = f.area();
            let layout = chat_layout(area);
            scrollback.render(layout.scrollback, f.buffer_mut());
        })
        .unwrap();

    let buf: Buffer = terminal.backend().buffer().clone();
    let _ = test_model();

    // Pull every symbol into a string so we can assert.
    let area = buf.area;
    let mut haystack = String::with_capacity((area.width as usize) * (area.height as usize));
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(c) = buf.cell((x, y)) {
                haystack.push_str(c.symbol());
            }
        }
    }

    assert!(
        haystack.contains("Hello"),
        "expected 'Hello' in rendered buffer"
    );
    assert!(
        haystack.contains("world"),
        "expected 'world' in rendered buffer"
    );
}

#[tokio::test]
async fn every_yaml_fixture_is_discovered_and_exercised() {
    let _test_lock = E2E_TEST_LOCK.lock().await;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/e2e");
    let mut fixtures = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("fixture directory entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "no YAML fixtures found in {}",
        dir.display()
    );

    for path in fixtures {
        let scenario = load_scenario(&path)
            .unwrap_or_else(|error| panic!("malformed YAML fixture {}: {error}", path.display()));
        let outcome = run_scenario(&scenario)
            .await
            .unwrap_or_else(|error| panic!("fixture failed to run {}: {error}", path.display()));
        assert_scenario_async(&outcome, &scenario)
            .await
            .unwrap_or_else(|error| {
                panic!("fixture assertions failed {}: {error}", path.display())
            });
    }
}
