//! `runie` — minimal TUI binary. Bootstraps the loop actor and runs the App.
//!
//! Note: without a real `StreamFn` adapter wired up, this binary is a UI
//! shell — the loop will publish events but no real LLM stream will drive
//! it. A `StreamFn` adapter is a follow-up task.

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::FutureExt;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use std::panic::AssertUnwindSafe;
use ratatui::Terminal;
use runie_core::events::EventBus;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::ToolExecutorActor;
use runie_core::tools::ToolRegistry;
use runie_core::types::{AgentContext, AgentMessage, Model, QueueMode, SimpleStreamOptions, ToolExecutionMode};

use runie_tui::app::{App, AppExit};
use runie_tui::key::{map_key, Action};
use runie_tui::widgets::PromptOutcome;

/// Placeholder StreamFn: emits a single "Hello from runie!" then Done.
struct PlaceholderStream;
#[async_trait::async_trait]
impl StreamFn for PlaceholderStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        use futures::stream;
        use runie_core::types::{AssistantMessageEvent, StopReason, Usage};
        let events = vec![
            AssistantMessageEvent::Start,
            AssistantMessageEvent::TextDelta { delta: "Hello from runie!".into() },
            AssistantMessageEvent::Done { stop_reason: StopReason::Stop, usage: Usage::default() },
        ];
        Ok(Box::pin(stream::iter(events)))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;
    let _ = res; // discard AppExit
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<AppExit> {
    let bus = EventBus::new();
    let state = AgentStateActor::new();
    let steering = SteeringQueueActor::new();
    let follow_up = FollowUpQueueActor::new();
    let tool_executor = ToolExecutorActor::new(std::sync::Arc::new(ToolRegistry::new()));
    let provider = ProviderActor::new(std::sync::Arc::new(PlaceholderStream));
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
        steering_mode: QueueMode::OneAtATime,
        follow_up_mode: QueueMode::OneAtATime,
    };
    let actor = LoopActor::new(deps);
    let mut app = App::new(actor, bus.clone());

    let _renderer_handle = app.spawn_renderer();

    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            biased;
            _ = tick.tick() => {
                if let Err(e) = terminal.draw(|f| {
                    use ratatui::layout::Rect;
                    use ratatui::widgets::Widget;
                    use runie_tui::layout::chat_layout;
                    let layout = chat_layout(f.area());
                    // Wrap each render in catch_unwind so a widget bug
                    // doesn't kill the binary — log + continue.
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let buf = f.buffer_mut();
                        let sb = app.status.clone();
                        sb.lock().render(layout.status, buf);
                        app.scrollback.lock().render(layout.scrollback, buf);
                        let prompt = app.prompt.clone();
                        Widget::render(prompt, layout.prompt, buf);
                        // Header: cwd + branch (matches grok-build's
                        // `main ~/...` minimal-mode chrome).
                        let cwd = std::env::current_dir()
                            .ok()
                            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                            .unwrap_or_else(|| "runie".into());
                        let branch = std::process::Command::new("git")
                            .args(["rev-parse", "--abbrev-ref", "HEAD"])
                            .output()
                            .ok()
                            .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "main".into());
                        let header_text = format!("  {} {}", branch, cwd);
                        let header_line = ratatui::text::Line::from(header_text)
                            .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
                        let p = ratatui::widgets::Paragraph::new(header_line);
                        Widget::render(p, layout.header, buf);
                        let _ = Rect::default();
                    }));
                    if let Err(e) = res {
                        eprintln!("render panic: {:?}", e);
                    }
                }) {
                    return Ok(AppExit::Error(format!("draw: {e}")));
                }
            }
            maybe = events.next() => {
                let Some(Ok(ev)) = maybe else { continue };
                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press { continue; }
                    // Submit path: route via PromptWidget.
                    if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace)
                        || (key.code == KeyCode::Enter && key.modifiers == KeyModifiers::NONE)
                    {
                        let outcome = {
                            let p = &mut app.prompt;
                            p.handle_key(key)
                        };
                        if let PromptOutcome::Submitted(text) = outcome {
                            let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                                content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                                timestamp: 0,
                            });
                            if let Err(e) = app.loop_actor.prompt(vec![user_msg], AgentContext::default()).await {
                                eprintln!("prompt error: {e:?}");
                            }
                        }
                        continue;
                    }
                    // Other actions.
                    let streaming = app.status.lock().current() != &runie_tui::widgets::Status::Ready;
                    let prompt_non_empty = !app.prompt.is_empty();
                    let action = map_key(key, prompt_non_empty, streaming);
                    match action {
                        Action::Quit => return Ok(AppExit::Quit),
                        Action::Abort => app.loop_actor.abort(),
                        Action::ClearScrollback => app.scrollback.lock().clear(),
                        Action::ClearPrompt => app.prompt.clear(),
                        Action::Submit(_) | Action::FocusPrompt | Action::Noop => {}
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
fn _key_marker(_k: KeyEvent) {}