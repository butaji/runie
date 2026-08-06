//! `runie` — minimal TUI binary. Bootstraps the loop actor and runs the App.
//!
//! Note: without a real `StreamFn` adapter wired up, this binary is a UI
//! shell — the loop will publish events but no real LLM stream will drive
//! it. A `StreamFn` adapter is a follow-up task.

use std::io::{self, Stdout};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
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
    AgentContext, AgentMessage, Model, QueueMode, SimpleStreamOptions, ThemeKind, ToolExecutionMode,
};

use runie_tui::app::{App, AppExit};
use runie_tui::key::{is_quit_command, map_key, Action};
use runie_tui::widgets::{PromptOutcome, Status};

/// Placeholder StreamFn: emits a single "Hello from runie!" then Done.
struct PlaceholderStream;

fn render_shortcuts(area: Rect, buf: &mut Buffer) {
    let width = 38.min(area.width.saturating_sub(2));
    let height = 8.min(area.height.saturating_sub(2));
    if width < 10 || height < 3 {
        return;
    }
    let panel = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    ratatui::widgets::Widget::render(
        ratatui::widgets::Paragraph::new(
            "Enter  send\nShift+Tab  cycle mode\nCtrl+C  clear / abort\nEsc  clear prompt\nCtrl+L  file search\ne  fold/unfold feed",
        )
        .block(
            ratatui::widgets::Block::default()
                .title(" Shortcuts ")
                .borders(ratatui::widgets::Borders::ALL),
        ),
        panel,
        buf,
    );
}

fn render_command_palette(area: Rect, buf: &mut Buffer, query: &str, selected: usize) {
    ratatui::widgets::Widget::render(
        runie_tui::widgets::CommandPaletteWidget::new(query, selected),
        area,
        buf,
    );
}

fn render_doctor_hint(area: Rect, buf: &mut Buffer) {
    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("Run ", runie_tui::appearance::muted_style()),
        ratatui::text::Span::styled(
            "/doctor",
            runie_tui::appearance::base_style().add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::styled(
            " for details and fixes.",
            runie_tui::appearance::muted_style(),
        ),
    ]);
    ratatui::widgets::Widget::render(
        ratatui::widgets::Paragraph::new(line),
        Rect {
            x: area.x,
            y: area.y.saturating_sub(1),
            width: area.width,
            height: 1,
        },
        buf,
    );
}

fn current_branch() -> &'static str {
    static BRANCH: OnceLock<String> = OnceLock::new();
    BRANCH
        .get_or_init(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .and_then(|output| {
                    output
                        .status
                        .success()
                        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                })
                .filter(|branch| !branch.is_empty())
                .unwrap_or_else(|| "main".into())
        })
        .as_str()
}

fn repository_label() -> String {
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("runie"));
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if let Some(home) = home {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn render_header(area: Rect, buf: &mut Buffer, meter: &str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // Grok's working view keeps this chrome deliberately quiet: branch icon
    // at the transcript rail, repository path after it, no product/version
    // badge competing with the feed.
    let left = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {}", current_branch()),
            runie_tui::appearance::muted_style(),
        ),
        ratatui::text::Span::styled(
            format!(" {}", repository_label()),
            runie_tui::appearance::base_style(),
        ),
    ]));
    ratatui::widgets::Widget::render(left, area, buf);
    let x = area.right().saturating_sub(meter.len() as u16);
    buf.set_string(x, area.y, meter, runie_tui::appearance::muted_style());
}

fn render_live_ready_footer(area: Rect, buf: &mut Buffer) {
    buf.set_string(
        area.x,
        area.y,
        " ".repeat(area.width as usize),
        runie_tui::appearance::base_style(),
    );
    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            "Shift+Tab",
            runie_tui::appearance::base_style().add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::styled(":mode  │  ", runie_tui::appearance::muted_style()),
        ratatui::text::Span::styled(
            "Ctrl+x",
            runie_tui::appearance::base_style().add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::styled(":shortcuts", runie_tui::appearance::muted_style()),
    ]);
    ratatui::widgets::Widget::render(ratatui::widgets::Paragraph::new(line), area, buf);
}

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
            AssistantMessageEvent::ThinkingDelta {
                delta: "briefly considering the request".into(),
            },
            AssistantMessageEvent::TextDelta {
                delta: "Hey — what would you like to work on in the runie repo?".into(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
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

    // Resolve repository metadata before entering the redraw loop; render
    // paths only read the cached projection.
    let _ = current_branch();
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

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "terminal event-loop ownership and redraw are intentionally co-located"
)]
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
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: Default::default(),
        abort: None,
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: QueueMode::OneAtATime,
        follow_up_mode: QueueMode::OneAtATime,
    };
    let actor = LoopActor::new(deps);
    let app = App::new(actor, bus.clone());
    app.refresh_model_caption().await;
    app.prompt
        .set_model_caption("Grok 4.5 (high) · always-approve".into())
        .await;

    // Paint a first frame synchronously before entering the event loop. This
    // guarantees the alternate screen is initialized even when the input
    // stream has not produced its first readiness notification yet.
    terminal.draw(|frame| {
        use ratatui::widgets::Widget;
        let frame_area = frame.area();
        let layout = runie_tui::layout::chat_layout(frame_area);
        let status = app.status_snapshot();
        frame.buffer_mut().set_style(
            frame_area,
            runie_tui::appearance::background_style_for(ThemeKind::GrokNight),
        );
        status.render(layout.status, frame.buffer_mut());
        if matches!(status.current(), Status::Ready) {
            render_live_ready_footer(layout.status, frame.buffer_mut());
        }
        let theme = status.theme();
        let mut scrollback = app.scrollback_snapshot();
        scrollback.set_theme(theme);
        scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
        scrollback.normalize_live_completed_assistants();
        scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
        scrollback.render(layout.scrollback, frame.buffer_mut());
        if matches!(status.current(), Status::Ready) && app.scrollback_snapshot().is_empty() {
            render_doctor_hint(layout.prompt, frame.buffer_mut());
        }
        Widget::render(app.prompt.snapshot(), layout.prompt, frame.buffer_mut());
        if app.ui.snapshot().shortcuts_open {
            render_shortcuts(frame.area(), frame.buffer_mut());
        }
        if app.ui.snapshot().command_palette_open {
            let palette = app.ui.snapshot();
            render_command_palette(
                frame.area(),
                frame.buffer_mut(),
                &palette.command_palette_query,
                palette.command_palette_index,
            );
        }
        let meter = status.header_meter();
        render_header(layout.header, frame.buffer_mut(), &meter);
        frame.set_cursor_position(app.prompt.snapshot().cursor_position(layout.prompt));
    })?;

    let (renderer_handle, renderer_shutdown) = app.spawn_renderer();

    let mut tick = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                app.refresh_model_caption().await;
                // Poll the controlling terminal on the render cadence as a
                // fallback for PTYs whose async reader does not wake.
                if event::poll(Duration::ZERO).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind == KeyEventKind::Press {
                            if app.ui.snapshot().command_palette_open
                                && key.code == KeyCode::Esc
                            {
                                app.toggle_command_palette().await;
                                continue;
                            }
                            if app.ui.snapshot().command_palette_open {
                                match key.code {
                                    KeyCode::Char(ch) if key.modifiers.is_empty() => {
                                        app.command_palette_key(
                                            runie_tui::app::UiMsg::CommandPaletteChar(ch),
                                        )
                                        .await;
                                    }
                                    KeyCode::Backspace => {
                                        app.command_palette_key(
                                            runie_tui::app::UiMsg::CommandPaletteBackspace,
                                        )
                                        .await;
                                    }
                                    KeyCode::Up => {
                                        app.command_palette_key(
                                            runie_tui::app::UiMsg::CommandPaletteMove(-1),
                                        )
                                        .await;
                                    }
                                    KeyCode::Down => {
                                        app.command_palette_key(
                                            runie_tui::app::UiMsg::CommandPaletteMove(1),
                                        )
                                        .await;
                                    }
                                    KeyCode::Enter => {
                                        match app.activate_command_palette().await.as_deref() {
                                            Some("New Session") => {
                                                app.bus.publish(runie_core::types::AgentEvent::Reset);
                                            }
                                            Some("Keyboard Shortcuts") => {
                                                app.toggle_shortcuts().await;
                                            }
                                            Some("Quit") => {
                                                let _ = renderer_shutdown.send(true);
                                                let _ = renderer_handle.await;
                                                return Ok(AppExit::Quit);
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                            let status = app.status_snapshot().current().clone();
                            let streaming = matches!(status, Status::Thinking | Status::Streaming);
                            match map_key(key, !app.prompt.snapshot().is_empty(), streaming) {
                                Action::ClearPrompt => {
                                    app.prompt.clear().await;
                                    continue;
                                }
                                Action::Abort => {
                                    app.loop_actor.abort();
                                    continue;
                                }
                                Action::ModeCycle => {
                                    app.prompt.cycle_mode().await;
                                    continue;
                                }
                                Action::OpenShortcuts => {
                                    app.toggle_shortcuts().await;
                                    continue;
                                }
                                Action::OpenCommandPalette => {
                                    app.toggle_command_palette().await;
                                    continue;
                                }
                                Action::OpenFileSearch => {
                                    app.prompt.open_file_search().await;
                                    continue;
                                }
                                Action::ToggleFold => {
                                    app.toggle_activity_fold().await;
                                    continue;
                                }
                                Action::Quit => {
                                    let _ = renderer_shutdown.send(true);
                                    let _ = renderer_handle.await;
                                    return Ok(AppExit::Quit);
                                }
                                _ => {}
                            }
                            if key.modifiers.is_empty() {
                                match key.code {
                                    KeyCode::Char(_) => {
                                        app.prompt.handle_key(key).await;
                                    }
                                    KeyCode::Backspace => {
                                        app.prompt.handle_key(key).await;
                                    }
                                    KeyCode::Enter => {
                                        let outcome = app.prompt.handle_key(key).await;
                                        if let PromptOutcome::Submitted(text) = outcome {
                                            if is_quit_command(&text) {
                                                return Ok(AppExit::Quit);
                                            }
                                            let user_msg = AgentMessage::User(runie_core::types::UserMessage {
                                                content: vec![runie_core::types::UserContent::Text { text }],
                                                timestamp: unix_timestamp_seconds(),
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
                            } else if key.modifiers.contains(KeyModifiers::SHIFT)
                                && matches!(key.code, KeyCode::Char(_))
                            {
                                // Crossterm reports an uppercase character
                                // as Shift+Char. It is still prompt input;
                                // dropping it makes `Hey` become `y`.
                                app.prompt.handle_key(key).await;
                            } else if matches!(key.code, KeyCode::Enter) {
                                app.prompt.handle_key(key).await;
                            }
                        }
                    }
                }
                if matches!(app.status_snapshot().current(), Status::Ready)
                    && !app.scrollback_snapshot().is_empty()
                {
                    // In Grok's settled conversation view the prompt keeps
                    // only its cursor marker; placeholder text is an idle
                    // empty-session affordance.
                    app.prompt.set_placeholder_visible(false).await;
                }
                if let Err(e) = terminal.draw(|f| {
                    use ratatui::layout::Rect;
                    use ratatui::widgets::Widget;
                    use runie_tui::layout::chat_layout;
                    let layout = chat_layout(f.area());
                    // Wrap each render in catch_unwind so a widget bug
                    // doesn't kill the binary — log + continue.
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let frame_area = f.area();
                        let buf = f.buffer_mut();
                        buf.set_style(
                            frame_area,
                            runie_tui::appearance::background_style_for(ThemeKind::GrokNight),
                        );
                        let status = app.status_snapshot();
                        let turn_status = status.turn_status();
                        status.render(layout.status, buf);
                        if matches!(status.current(), Status::Ready) {
                            render_live_ready_footer(layout.status, buf);
                        }
                        let theme = status.theme();
                        let mut scrollback = app.scrollback_snapshot();
                        scrollback.set_theme(theme);
                        scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
                        scrollback.normalize_live_completed_assistants();
                        scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
                        scrollback.render(layout.scrollback, buf);
                if matches!(status.current(), Status::Ready) && app.scrollback_snapshot().is_empty() {
                    render_doctor_hint(layout.prompt, buf);
                }
                        if let Some(turn_status) = turn_status {
                            turn_status.render(
                                ratatui::layout::Rect {
                                    x: layout.scrollback.x,
                                    y: layout.prompt.y.saturating_sub(2),
                                    width: layout.scrollback.width,
                                    height: 1,
                                },
                                buf,
                            );
                        }
                        let prompt = app.prompt.snapshot();
                        Widget::render(prompt, layout.prompt, buf);
                        if app.ui.snapshot().shortcuts_open {
                            render_shortcuts(frame_area, buf);
                        }
                        if app.ui.snapshot().command_palette_open {
                            let palette = app.ui.snapshot();
                            render_command_palette(
                                frame_area,
                                buf,
                                &palette.command_palette_query,
                                palette.command_palette_index,
                            );
                        }
                        let meter = status.header_meter();
                        render_header(layout.header, buf, &meter);
                        f.set_cursor_position(app.prompt.snapshot().cursor_position(layout.prompt));
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

fn unix_timestamp_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[allow(dead_code)]
fn _key_marker(_k: KeyEvent) {}

#[cfg(test)]
mod tests {
    use super::{current_branch, render_header, repository_label};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn header_uses_cached_repository_branch() {
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        render_header(area, &mut buffer, "0 / 500K");
        let row = (0..area.width)
            .filter_map(|x| buffer.cell((x, 0)))
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(row.contains(current_branch()));
        assert!(row.contains(&repository_label()));
    }
}
