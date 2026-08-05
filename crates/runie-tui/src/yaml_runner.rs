//! YAML-driven e2e test runner for `runie-tui`.
//!
//! Each YAML fixture under `tests/yaml_fixtures/*.yaml` is loaded, parsed into
//! a `Scenario`, then executed against a real `LoopActor` + `EventRenderer`.
//! The runner applies the fixture's assertions against the recorded events
//! and the rendered scrollback.

use std::path::Path;
use std::sync::Arc;

use crate::event_renderer::EventRenderer;
use crate::widgets::{Line, LineKind, Scrollback};
use parking_lot::Mutex;
use ratatui::buffer::Buffer;
use runie_core::events::EventBus;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::{ToolExecutorActor, ToolRegistry};
use runie_core::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantMessageEvent,
    Model, SimpleStreamOptions, StopReason, ToolExecutionMode, ToolResultContent, Usage,
    UserContent, UserMessage,
};
use serde::Deserialize;
use tokio::sync::broadcast;

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub initial_prompt: Option<String>,
    #[serde(default)]
    pub follow_up: Vec<String>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    pub events: Vec<EventSpec>,
    #[serde(default)]
    pub assertions: Assertions,
}

#[derive(Debug, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(default = "default_tool_kind")]
    pub kind: String,
}

fn default_tool_kind() -> String {
    "echo".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum EventSpec {
    Bare(String),
    TextDelta { text_delta: String },
    ThinkingDelta { thinking_delta: String },
    ToolCall { tool_call: ToolCallSpec },
    Done { done: DoneSpec },
    Error { error: String },
}

#[derive(Debug, Deserialize, Clone)]
pub struct DoneSpec {
    #[serde(default)]
    pub stop_reason: StopReasonSpec,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallSpec {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StopReasonSpec {
    #[default]
    Stop,
    ToolUse,
    MaxTokens,
    Aborted,
}

impl From<&StopReasonSpec> for StopReason {
    fn from(s: &StopReasonSpec) -> Self {
        match s {
            StopReasonSpec::Stop => StopReason::Stop,
            StopReasonSpec::ToolUse => StopReason::ToolUse,
            StopReasonSpec::MaxTokens => StopReason::MaxTokens,
            StopReasonSpec::Aborted => StopReason::Aborted,
        }
    }
}

impl EventSpec {
    fn to_assistant_event(&self) -> AssistantMessageEvent {
        match self {
            Self::Bare(s) if s == "start" => AssistantMessageEvent::Start,
            Self::TextDelta { text_delta } => AssistantMessageEvent::TextDelta {
                delta: text_delta.clone(),
            },
            Self::ThinkingDelta { thinking_delta } => AssistantMessageEvent::ThinkingDelta {
                delta: thinking_delta.clone(),
            },
            Self::ToolCall { tool_call } => AssistantMessageEvent::ToolCallDelta {
                index: 0,
                partial: runie_core::types::ToolCall {
                    id: format!("call-{}", uuid_like()),
                    name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                },
            },
            Self::Done { done } => AssistantMessageEvent::Done {
                stop_reason: StopReason::from(&done.stop_reason),
                usage: Usage::default(),
            },
            Self::Error { error } => AssistantMessageEvent::Error {
                error: error.clone(),
            },
            Self::Bare(other) => panic!("unknown event kind: {other:?}"),
        }
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{n}")
}

#[derive(Debug, Deserialize, Default)]
pub struct Assertions {
    #[serde(default)]
    pub transcript_contains: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub turn_starts: Option<usize>,
    #[serde(default)]
    pub scrollback_lines: Vec<LineAssertion>,
    /// Optional in-process visual check: drive the TUI App via TestBackend
    /// at the given viewport size, then assert substrings appear in / are
    /// excluded from the rendered screen.
    #[serde(default)]
    pub visual: Option<VisualAssertions>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct VisualAssertions {
    #[serde(default = "default_visual_cols")]
    pub cols: u16,
    #[serde(default = "default_visual_rows")]
    pub rows: u16,
    #[serde(default)]
    pub screen_text: Vec<String>,
    #[serde(default)]
    pub screen_excludes: Vec<String>,
    /// Steps the TUI should perform before snapshotting the screen.
    /// Each step is a key event (e.g. "hello", "Enter", "Ctrl+C").
    #[serde(default)]
    pub steps: Vec<String>,
    /// If true, also spawn the real `runie` binary in a pty and assert
    /// the same `screen_text` / `screen_excludes` substrings there.
    /// Requires `portable-pty` (currently a no-op stub).
    #[serde(default)]
    pub pty: bool,
}

fn default_visual_cols() -> u16 {
    120
}
fn default_visual_rows() -> u16 {
    30
}

#[derive(Debug, Deserialize)]
pub struct LineAssertion {
    pub kind: LineKindName,
    pub contains: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LineKindName {
    User,
    Assistant,
    Tool,
    ToolResult,
    ToolOutput,
    System,
    Activity,
    Reasoning,
}

impl From<LineKindName> for LineKind {
    fn from(k: LineKindName) -> Self {
        match k {
            LineKindName::User => LineKind::User,
            LineKindName::Assistant => LineKind::Assistant,
            LineKindName::Tool => LineKind::Tool,
            LineKindName::ToolResult => LineKind::ToolResult,
            LineKindName::ToolOutput => LineKind::ToolOutput,
            LineKindName::System => LineKind::System,
            LineKindName::Activity => LineKind::Activity,
            LineKindName::Reasoning => LineKind::Reasoning,
        }
    }
}

/// StreamFn impl driven by a `Vec<AssistantMessageEvent>`.
pub struct ScenarioStream {
    pub events: Vec<AssistantMessageEvent>,
}

#[async_trait::async_trait]
impl StreamFn for ScenarioStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        use futures::stream;
        // `unfold` lets us yield between events so the runtime can poll
        // other tasks (the renderer) between items. Without this the
        // `current_thread` runtime runs the synchronous stream to
        // completion in one scheduling slice and the renderer only sees
        // the first event.
        let events = self.events.clone();
        let s = stream::unfold(0usize, move |idx| {
            let events = events.clone();
            async move {
                if idx >= events.len() {
                    return None;
                }
                tokio::task::yield_now().await;
                let e = events[idx].clone();
                Some((e, idx + 1))
            }
        });
        Ok(Box::pin(s))
    }
}

/// Echo tool that returns its args verbatim.
pub struct EchoTool;
#[async_trait::async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn label(&self) -> &str {
        "Echo"
    }
    fn description(&self) -> &str {
        "Echoes args."
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

/// Deterministic named tool used by strict visual replays. Its output is
/// fixed so the TestBackend frame never depends on the host filesystem.
pub struct ReplayTool {
    name: String,
    output: String,
}

impl ReplayTool {
    fn new(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for ReplayTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "Deterministic visual replay tool."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: self.output.clone(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub events: Vec<AgentEvent>,
    pub scrollback: Vec<Line>,
}

pub struct ScenarioError(pub String);

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub async fn run_scenario(scenario: &Scenario) -> Result<ScenarioOutcome, ScenarioError> {
    // Build actors.
    let bus = EventBus::new();
    let state = AgentStateActor::new();
    let steering = SteeringQueueActor::new();
    let follow_up = FollowUpQueueActor::new();
    let mut reg = ToolRegistry::new();
    for t in &scenario.tools {
        match t.kind.as_str() {
            "echo" => reg.register(Arc::new(EchoTool)),
            "list_dir" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "Cargo.toml\nsrc\ncrates",
            ))),
            "read" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "# runie\n\nThis is **Runie**.",
            ))),
            other => return Err(ScenarioError(format!("unknown tool kind: {other}"))),
        }
    }
    let tool_executor = ToolExecutorActor::new(Arc::new(reg));
    let provider = ProviderActor::new(Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .map(EventSpec::to_assistant_event)
            .collect(),
    }));

    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: runie_core::types::QueueMode::OneAtATime,
        follow_up_mode: runie_core::types::QueueMode::OneAtATime,
    };
    let actor = LoopActor::new(deps);

    // Recorder: separate subscription to the bus, returns the captured Vec.
    // We only record events here; the scrollback is filled *synchronously*
    // after the loop finishes by replaying the captured events through a
    // fresh renderer. This sidesteps a runtime-scheduling race where a
    // bus-driven renderer on a `current_thread` runtime only sees the
    // first event before the loop future completes.
    let mut rec_rx = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rec_rx.recv() => {
                    match result {
                        Ok(ev) => captured.push(ev),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        captured
    });

    // Pre-push follow-ups (will drain after first turn).
    for text in &scenario.follow_up {
        actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }

    // Submit initial prompt (if any).
    let prompts = match &scenario.initial_prompt {
        Some(text) => vec![AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: text.clone() }],
            timestamp: 1,
        })],
        None => vec![],
    };

    if let Err(e) = actor.prompt(prompts, AgentContext::default()).await {
        eprintln!("[yaml_runner] prompt error: {e:?}");
    }

    // Signal the recorder to stop (the bus is still held by LoopActor,
    // so it doesn't close — we use a dedicated oneshot to break the loop).
    let _ = rec_stop_tx.send(());
    let events_from_task = rec_handle.await.unwrap_or_default();

    // Replay the captured events synchronously through a fresh renderer.
    // This sidesteps the runtime-scheduling race we hit when the renderer
    // was driven via the bus on a `current_thread` runtime.
    let scrollback_arc = Arc::new(Mutex::new(Scrollback::new()));
    let status_arc = Arc::new(Mutex::new(crate::widgets::StatusBar::new()));
    let mut renderer = EventRenderer::with_welcome(
        scrollback_arc.clone(),
        status_arc.clone(),
        scenario.initial_prompt.is_none(),
    );
    for ev in &events_from_task {
        renderer.apply_event(ev.clone());
    }

    let scrollback_lines = {
        let guard = scrollback_arc.lock();
        guard.snapshot_lines()
    };

    Ok(ScenarioOutcome {
        events: events_from_task,
        scrollback: scrollback_lines,
    })
}

pub fn assert_scenario(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    // Reuse the current tokio runtime if any (the e2e binary runs each
    // scenario inside `#[tokio::main]`); otherwise spin up a fresh one.
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(assert_scenario_async(outcome, scenario)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("build tokio runtime: {e}"))?;
            rt.block_on(assert_scenario_async(outcome, scenario))
        }
    }
}

pub async fn assert_scenario_async(
    outcome: &ScenarioOutcome,
    scenario: &Scenario,
) -> Result<(), String> {
    use runie_core::types::AgentEvent::*;
    // Build expected kind sequence.
    let kinds: Vec<&'static str> = outcome
        .events
        .iter()
        .map(|e| match e {
            AgentStart => "agent_start",
            AgentEnd { .. } => "agent_end",
            TurnStart => "turn_start",
            TurnEnd { .. } => "turn_end",
            MessageStart { .. } => "message_start",
            MessageUpdate { .. } => "message_update",
            MessageEnd { .. } => "message_end",
            ToolExecutionStart { .. } => "tool_execution_start",
            ToolExecutionUpdate { .. } => "tool_execution_update",
            ToolExecutionEnd { .. } => "tool_execution_end",
        })
        .collect();

    for expected in &scenario.assertions.events {
        if !kinds.iter().any(|k| k == &expected.as_str()) {
            return Err(format!("expected event kind {expected:?} not in {kinds:?}"));
        }
    }
    if let Some(n) = scenario.assertions.turn_starts {
        let count = kinds.iter().filter(|k| **k == "turn_start").count();
        if count != n {
            return Err(format!("expected {n} turn_start events, got {count}"));
        }
    }
    let haystack: String = outcome
        .scrollback
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in &scenario.assertions.transcript_contains {
        if !haystack.contains(needle) {
            return Err(format!(
                "transcript missing {needle:?}; full haystack:\n{haystack}"
            ));
        }
    }
    for la in &scenario.assertions.scrollback_lines {
        let kind: LineKind = la.kind.into();
        let found = outcome
            .scrollback
            .iter()
            .any(|l| l.kind == kind && l.text.contains(&la.contains));
        if !found {
            return Err(format!(
                "expected {kind:?} line containing {:?}; scrollback:\n{haystack}",
                la.contains
            ));
        }
    }
    if let Some(vis) = &scenario.assertions.visual {
        let screen = render_visual(scenario, vis)
            .await
            .map_err(|e| e.to_string())?;
        for needle in &vis.screen_text {
            if !screen.contains(needle) {
                return Err(format!("screen missing {needle:?}\nscreen:\n{screen}"));
            }
        }
        for needle in &vis.screen_excludes {
            if screen.contains(needle) {
                return Err(format!(
                    "screen unexpectedly contains {needle:?}\nscreen:\n{screen}"
                ));
            }
        }
        if vis.pty {
            // PTY support is a future hook: keep the field for fixture
            // forward-compatibility, but skip the assertion in this build.
            eprintln!("[visual] pty assertion requested but not yet implemented");
        }
    }
    Ok(())
}

/// Drive the TUI App via `TestBackend` and return the rendered screen text.
///
/// Mirrors grok-build's `harness.screen_contents()` contract: the result is
/// the full viewport (rows × cols) joined with `\n`, suitable for substring
/// assertions via the YAML `screen_text` / `screen_excludes` lists.
pub async fn render_visual_buffer(
    scenario: &Scenario,
    vis: &VisualAssertions,
) -> Result<Buffer, String> {
    use crate::app::App;
    use crate::layout::chat_layout;
    use crate::widgets::{PromptOutcome, WelcomeWidget};
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Widget;
    use ratatui::Terminal;

    // Build the same wiring as run_scenario.
    let bus = runie_core::events::EventBus::new();
    let state = runie_core::state::AgentStateActor::new();
    let steering = runie_core::queues::SteeringQueueActor::new();
    let follow_up = runie_core::queues::FollowUpQueueActor::new();
    let mut reg = ToolRegistry::new();
    for t in &scenario.tools {
        match t.kind.as_str() {
            "echo" => reg.register(Arc::new(EchoTool)),
            "list_dir" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "Cargo.toml\nsrc\ncrates",
            ))),
            "read" => reg.register(Arc::new(ReplayTool::new(
                &t.name,
                "# runie\n\nThis is **Runie**.",
            ))),
            other => return Err(format!("unknown tool kind: {other}")),
        }
    }
    let tool_executor = ToolExecutorActor::new(Arc::new(reg));
    let provider = ProviderActor::new(Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .map(EventSpec::to_assistant_event)
            .collect(),
    }));
    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: runie_core::types::QueueMode::OneAtATime,
        follow_up_mode: runie_core::types::QueueMode::OneAtATime,
    };
    let actor = LoopActor::new(deps);
    let mut app = App::new(actor, bus.clone());

    // Collect bus events in parallel: the recorder task subscribes once and
    // drains the bus into a `Vec` while the loop runs. This avoids a
    // runtime-scheduling race where the renderer misses events on a
    // `current_thread` runtime.
    let collected: Arc<Mutex<Vec<runie_core::types::AgentEvent>>> =
        Arc::new(Mutex::new(Vec::new()));
    let collected_clone = collected.clone();
    let rec_bus = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut rx = rec_bus;
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rx.recv() => {
                    match result {
                        Ok(ev) => collected_clone.lock().push(ev),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Pre-push follow-ups.
    for text in &scenario.follow_up {
        app.loop_actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }

    // If no prompt and no events, the loop never runs and the bus stays
    // empty. Synthesise a minimal AgentStart/AgentEnd pair so the welcome
    // modal is emitted (matches grok's idle screen).
    if scenario.events.is_empty() && scenario.initial_prompt.is_none() {
        use runie_core::types::AgentEvent;
        bus.publish(AgentEvent::AgentStart);
        tokio::task::yield_now().await;
        bus.publish(AgentEvent::AgentEnd { messages: vec![] });
        tokio::task::yield_now().await;
    }

    // Apply keystrokes.
    for step in &vis.steps {
        if step == "Enter" {
            let outcome = app.prompt.handle_key(crossterm::event::KeyEvent {
                code: crossterm::event::KeyCode::Enter,
                modifiers: crossterm::event::KeyModifiers::NONE,
                kind: crossterm::event::KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            });
            if let PromptOutcome::Submitted(text) = outcome {
                let user_msg = AgentMessage::User(UserMessage {
                    content: vec![UserContent::Text { text }],
                    timestamp: 1,
                });
                let _ = app
                    .loop_actor
                    .prompt(vec![user_msg], AgentContext::default())
                    .await;
            }
        } else {
            for ch in step.chars() {
                let outcome = app.prompt.handle_key(crossterm::event::KeyEvent {
                    code: crossterm::event::KeyCode::Char(ch),
                    modifiers: crossterm::event::KeyModifiers::NONE,
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                });
                if matches!(outcome, PromptOutcome::Edited) {
                    app.show_welcome = false;
                }
            }
        }
    }

    // Grok clears the idle welcome surface as soon as editing begins; the
    // synthetic idle events above must not remain in the typed frame.
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.scrollback.lock().clear();
    }

    // If scenario has an initial_prompt and no Enter step, submit it.
    if let Some(text) = &scenario.initial_prompt {
        app.show_welcome = false;
        if !vis.steps.iter().any(|s| s == "Enter") {
            let user_msg = AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 1,
            });
            let _ = app
                .loop_actor
                .prompt(vec![user_msg], AgentContext::default())
                .await;
        }
    }

    // Let the recorder make progress without introducing timing-dependent
    // sleeps into visual tests.
    for _ in 0..3 {
        tokio::task::yield_now().await;
    }

    // Render to a TestBackend.
    let backend = TestBackend::new(vis.cols, vis.rows);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    terminal
        .draw(|f| {
            let area = f.area();
            let layout = chat_layout(area);
            if app.show_welcome {
                WelcomeWidget.render(layout.scrollback, f.buffer_mut());
            } else {
                app.scrollback
                    .lock()
                    .render(layout.scrollback, f.buffer_mut());
            }
            if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
                crate::widgets::TurnStatus::new(0).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: layout.prompt.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            let prompt = app.prompt.clone();
            Widget::render(prompt, layout.prompt, f.buffer_mut());
            let sb = app.status.clone();
            sb.lock().render(layout.status, f.buffer_mut());
            Widget::render(
                ratatui::widgets::Paragraph::new(" main ~/Code/GitHub/runie-tests/runie")
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)),
                layout.header,
                f.buffer_mut(),
            );
            f.set_cursor_position(app.prompt.cursor_position(layout.prompt));
        })
        .map_err(|e| e.to_string())?;

    // Stop the recorder task. The bus is held by LoopActor so dropping it
    // would not close the channel — we use a dedicated oneshot to break
    // the recorder's recv loop.
    let _ = rec_stop_tx.send(());
    let _ = rec_handle.await;

    // Apply every collected event SYNCHRONOUSLY to a fresh renderer. This
    // bypasses the runtime-scheduling race that prevented the
    // bus-driven renderer from seeing all events on a `current_thread`
    // runtime.
    let mut renderer = EventRenderer::with_welcome(
        app.scrollback.clone(),
        app.status.clone(),
        scenario.initial_prompt.is_none(),
    );
    let events = collected.lock().clone();
    for ev in events.into_iter() {
        renderer.apply_event(ev);
    }
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.scrollback.lock().clear();
    }

    // Render to a TestBackend.
    let backend = TestBackend::new(vis.cols, vis.rows);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    terminal
        .draw(|f| {
            let area = f.area();
            let layout = chat_layout(area);
            if app.show_welcome {
                WelcomeWidget.render(layout.scrollback, f.buffer_mut());
            } else {
                app.scrollback
                    .lock()
                    .render(layout.scrollback, f.buffer_mut());
            }
            if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
                crate::widgets::TurnStatus::new(0).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: layout.prompt.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            let prompt = app.prompt.clone();
            Widget::render(prompt, layout.prompt, f.buffer_mut());
            let sb = app.status.clone();
            sb.lock().render(layout.status, f.buffer_mut());
            Widget::render(
                ratatui::widgets::Paragraph::new(" main ~/Code/GitHub/runie-tests/runie")
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)),
                layout.header,
                f.buffer_mut(),
            );
            f.set_cursor_position(app.prompt.cursor_position(layout.prompt));
        })
        .map_err(|e| e.to_string())?;

    Ok(terminal.backend().buffer().clone())
}

pub async fn render_visual(scenario: &Scenario, vis: &VisualAssertions) -> Result<String, String> {
    let buf = render_visual_buffer(scenario, vis).await?;
    let mut out = String::with_capacity((buf.area.width as usize + 1) * (buf.area.height as usize));
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(c) = buf.cell((x, y)) {
                out.push_str(c.symbol());
            }
        }
        out.push('\n');
    }
    Ok(out)
}

pub fn load_scenario(path: &Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

// Bridge: pull lines out of Scrollback for assertions.
trait ScrollbackExt {
    fn snapshot_lines(&self) -> Vec<Line>;
}
impl ScrollbackExt for Scrollback {
    fn snapshot_lines(&self) -> Vec<Line> {
        self.lines().to_vec()
    }
}
