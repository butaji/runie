//! `runie` — minimal TUI binary. Bootstraps the loop actor and runs the App.
//!
//! Note: without a real `StreamFn` adapter wired up, this binary is a UI
//! shell — the loop will publish events but no real LLM stream will drive
//! it. A `StreamFn` adapter is a follow-up task.

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
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
use runie_core::types::{
    AgentContext, AgentMessage, Model, QueueMode, SimpleStreamOptions, ToolExecutionMode,
};

use runie_tui::app::{App, AppExit};
use runie_tui::key::is_quit_command;
use runie_tui::widgets::{PromptOutcome, WelcomeWidget};

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
            AssistantMessageEvent::TextDelta {
                delta: "Hello from runie!".into(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
            },
        ];
        Ok(Box::pin(stream::iter(events)))
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;
    if let Err(error) = res {
        eprintln!("runie TUI error: {error:#}");
    }
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    // Alternate-screen terminals can retain the previous buffer until the
    // first differential draw. Clear it explicitly so startup is visible
    // even when the terminal reports an unchanged frame.
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
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

    // Paint a first frame synchronously before entering the event loop. This
    // guarantees the alternate screen is initialized even when the input
    // stream has not produced its first readiness notification yet.
    terminal.draw(|frame| {
        use ratatui::widgets::Widget;
        let layout = runie_tui::layout::chat_layout(frame.area());
        app.status.lock().render(layout.status, frame.buffer_mut());
        WelcomeWidget.render(layout.scrollback, frame.buffer_mut());
        Widget::render(app.prompt.clone(), layout.prompt, frame.buffer_mut());
        let header = ratatui::widgets::Paragraph::new("  Runie");
        Widget::render(header, layout.header, frame.buffer_mut());
        frame.set_cursor_position(app.prompt.cursor_position(layout.prompt));
    })?;

    let _renderer_handle = app.spawn_renderer();

    let mut tick = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                app.status.lock().advance_animation();
                // Poll the controlling terminal on the render cadence as a
                // fallback for PTYs whose async reader does not wake.
                if event::poll(Duration::ZERO).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind == KeyEventKind::Press {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('q')
                            {
                                return Ok(AppExit::Quit);
                            }
                            if key.modifiers.is_empty() {
                                match key.code {
                                    KeyCode::Char(_) => {
                                        app.prompt.handle_key(key);
                                        app.show_welcome = false;
                                    }
                                    KeyCode::Backspace => {
                                        app.prompt.handle_key(key);
                                    }
                                    KeyCode::Enter => {
                                        let outcome = app.prompt.handle_key(key);
                                        if let PromptOutcome::Submitted(text) = outcome {
                                            app.show_welcome = false;
                                            if is_quit_command(&text) {
                                                return Ok(AppExit::Quit);
                                            }
                                            let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                                                content: vec![runie_core::types::UserContent::Text { text }],
                                                timestamp: 0,
                                            });
                                            if let Err(error) = app
                                                .loop_actor
                                                .prompt(vec![user_msg], AgentContext::default())
                                                .await
                                            {
                                                eprintln!("prompt error: {error:?}");
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
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
                        let (status, frame) = {
                            let status_bar = sb.lock();
                            (status_bar.current().clone(), status_bar.animation_frame())
                        };
                        let phase = match status {
                            runie_tui::widgets::Status::Thinking => Some(runie_tui::widgets::TurnStatusPhase::Thinking),
                            runie_tui::widgets::Status::Streaming => Some(runie_tui::widgets::TurnStatusPhase::Responding),
                            _ => None,
                        };
                        let active = phase.is_some();
                        sb.lock().render(layout.status, buf);
                        if app.show_welcome {
                            WelcomeWidget.render(layout.scrollback, buf);
                        } else {
                            app.scrollback.lock().render(layout.scrollback, buf);
                        }
                        if active {
                            runie_tui::widgets::TurnStatus::new(frame).phase(phase.expect("active phase")).render(
                                ratatui::layout::Rect {
                                    x: layout.scrollback.x,
                                    y: layout.prompt.y.saturating_sub(2),
                                    width: layout.scrollback.width,
                                    height: 1,
                                },
                                buf,
                            );
                        }
                        let prompt = app.prompt.clone();
                        Widget::render(prompt, layout.prompt, buf);
                        // Header: cwd + branch (matches grok-build's
                        // `main ~/...` minimal-mode chrome).
                        let cwd = std::env::current_dir()
                            .map(|p| {
                                let path = p.to_string_lossy();
                                std::env::var("HOME")
                                    .ok()
                                    .filter(|home| path.starts_with(home))
                                    .map(|home| format!("~{}", &path[home.len()..]))
                                    .unwrap_or_else(|| path.into_owned())
                            })
                            .unwrap_or_else(|_| "runie".into());
                        let header_text = format!("  {}", cwd);
                        let header_line = ratatui::text::Line::from(header_text)
                            .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
                        let p = ratatui::widgets::Paragraph::new(header_line);
                        Widget::render(p, layout.header, buf);
                        f.set_cursor_position(app.prompt.cursor_position(layout.prompt));
                        let _ = Rect::default();
                    }));
                    if let Err(e) = res {
                        eprintln!("render panic: {:?}", e);
                        std::panic::resume_unwind(e);
                    }
                }) {
                    return Ok(AppExit::Error(format!("draw: {e}")));
                }
            }
        }
    }
}

#[allow(dead_code)]
fn _key_marker(_k: KeyEvent) {}
