//! `runie` — minimal TUI binary. Bootstraps the loop actor and runs the App.
//!
//! Note: without a real `StreamFn` adapter wired up, this binary is a UI
//! shell — the loop will publish events but no real LLM stream will drive
//! it. A `StreamFn` adapter is a follow-up task.

use std::io::{self, Stdout};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;
use runie_core::commands::{parse_mappable_builtin_command, MappableBuiltinCommand};
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
    AgentContext, Model, QueueMode, SimpleStreamOptions, ThemeKind, ToolExecutionMode,
};

use futures::StreamExt;
use runie_tui::app::{App, AppExit, PaletteAction, UiCommand};
use runie_tui::key::{is_quit_command, map_key, Action};
use runie_tui::widgets::{PromptOutcome, PromptWidget, Scrollback, Status, StatusBar};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

/// Placeholder StreamFn: emits a single "Hello from runie!" then Done.
struct PlaceholderStream;

enum InputEvent {
    Key(KeyEvent),
    Mouse(i32),
    MouseSelectionStart(u16, u16),
    MouseSelectionExtend(u16, u16),
    MouseSelectionCommit,
}

enum InputConfig {
    ScrollViewport(u16),
    SelectionOrigin { row: u16, column: u16 },
}

fn mouse_selection_input(
    kind: MouseEventKind,
    row: u16,
    column: u16,
    origin: (u16, u16),
) -> Option<InputEvent> {
    let position = || {
        (
            row.saturating_sub(origin.0),
            column.saturating_sub(origin.1),
        )
    };
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let (row, column) = position();
            Some(InputEvent::MouseSelectionStart(row, column))
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let (row, column) = position();
            Some(InputEvent::MouseSelectionExtend(row, column))
        }
        MouseEventKind::Up(MouseButton::Left) => Some(InputEvent::MouseSelectionCommit),
        _ => None,
    }
}

fn render_command_palette(
    area: Rect,
    buf: &mut Buffer,
    query: &str,
    selected: usize,
    theme: runie_core::types::ThemeKind,
) {
    ratatui::widgets::Widget::render(
        runie_tui::widgets::CommandPaletteWidget::new(query, selected).with_theme(theme),
        area,
        buf,
    );
}

fn render_model_selector(
    area: Rect,
    buf: &mut Buffer,
    ui: &runie_tui::app::UiState,
    theme: runie_core::types::ThemeKind,
) {
    ratatui::widgets::Widget::render(
        runie_tui::widgets::ModelSelectorWidget::new(
            &ui.model_selector_query,
            ui.model_selector_index,
            ui.model_selector_scoped_only,
            ui.model_selector_result_count,
            ui.model_selector_rows.clone(),
        )
        .with_theme(theme),
        area,
        buf,
    );
}

fn render_compact_hint(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            "Tight on space? Try ",
            runie_tui::appearance::muted_style_for(theme),
        ),
        ratatui::text::Span::styled(
            "/compact-mode",
            runie_tui::appearance::base_style_for(theme)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::styled("", runie_tui::appearance::header_path_style_for(theme)),
    ])
    .style(runie_tui::appearance::base_style_for(theme));
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

fn render_header(area: Rect, buf: &mut Buffer, meter: &str, theme: runie_core::types::ThemeKind) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // Grok's working view keeps this chrome deliberately quiet: branch icon
    // at the transcript rail, repository path after it, no product/version
    // badge competing with the feed.
    let left = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {}", current_branch()),
            // Grok dims the branch identity while keeping the repository
            // path on its dedicated semantic token.
            runie_tui::appearance::base_style_for(theme)
                .add_modifier(ratatui::style::Modifier::DIM),
        ),
        ratatui::text::Span::raw(" "),
        ratatui::text::Span::styled(
            repository_label(),
            runie_tui::appearance::header_path_style_for(theme),
        ),
    ]));
    ratatui::widgets::Widget::render(left, area, buf);
    let x = area.right().saturating_sub(meter.len() as u16);
    buf.set_string(
        x,
        area.y,
        meter,
        // The context meter is primary header chrome in Grok, not muted
        // transcript text; the value itself remains actor-owned.
        runie_tui::appearance::header_meter_style_for(theme),
    );
}

fn render_live_ready_footer(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Paragraph, Widget};

    let key_style = runie_tui::appearance::footer_key_style_for(theme);
    let muted_style = runie_tui::appearance::muted_style_for(theme);

    let segments = [
        ("Shift+Tab", key_style.add_modifier(Modifier::BOLD)),
        (":mode  │  ", muted_style),
        ("Ctrl+x", key_style.add_modifier(Modifier::BOLD)),
        (":shortcuts", muted_style),
    ];
    let spans = segments
        .into_iter()
        .map(|(text, style)| Span::styled(text, style))
        .collect::<Vec<_>>();
    Paragraph::new(Line::from(spans))
        .style(muted_style)
        .render(area, buf);
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
        use runie_core::types::{AssistantMessage, AssistantMessageEvent, StopReason, Usage};
        let events = vec![
            AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingDelta {
                index: 1,
                delta: "briefly considering the request".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingEnd {
                index: 1,
                content: "briefly considering the request".into(),
                elapsed_ms: Some(runie_tui::clock::parity_thinking_elapsed_ms().unwrap_or(200)),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "Hey — what would you like to work on in runie?".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage {
                    total_tokens: 14_000,
                    ..Usage::default()
                },
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
    let terminal_native = std::env::args().any(|arg| arg == "--terminal-native");
    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal, terminal_native).await;
    restore_terminal(&mut terminal)?;
    if let Err(error) = res {
        eprintln!("runie TUI error: {error:#}");
    }
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
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
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "terminal event-loop ownership and redraw are intentionally co-located"
)]
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    terminal_native: bool,
) -> Result<AppExit> {
    let bus = EventBus::new();
    let state = AgentStateActor::new();
    let steering = SteeringQueueActor::new();
    let follow_up = FollowUpQueueActor::new();
    let tool_executor = ToolExecutorActor::new_live(std::sync::Arc::new(ToolRegistry::new()));
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
    if terminal_native {
        app.set_theme(ThemeKind::TerminalNative).await;
    }
    let mut ui_commands = app.subscribe_ui_commands();
    let mut placeholder_hidden = false;
    app.prompt
        .set_model_caption("Grok 4.5 (high) · always-approve".into())
        .await;
    let color_level = runie_tui::terminal_color::ColorLevel::from_environment();

    // Paint a first frame synchronously before entering the event loop. This
    // guarantees the alternate screen is initialized even when the input
    // stream has not produced its first readiness notification yet.
    terminal.draw(|frame| {
        use ratatui::widgets::Widget;
        let frame_area = frame.area();
        let layout = runie_tui::layout::chat_layout(frame_area);
        let model = app.model_snapshot();
        let document = App::view_document_from_model(&model);
        let view = &document.root;
        let status = StatusBar::from_model_snapshot(document.props.status.clone());
        frame.buffer_mut().set_style(
            frame_area,
            runie_tui::appearance::background_style_for(status.theme()),
        );
        if matches!(status.current(), Status::Ready) {
            render_live_ready_footer(layout.status, frame.buffer_mut(), status.theme());
        } else {
            status.render(layout.status, frame.buffer_mut());
        }
        let mut scrollback = Scrollback::from_model_snapshot(document.props.feed.clone());
        scrollback.set_live_grok_layout(true);
        scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
        scrollback.normalize_live_completed_assistants();
        scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
        scrollback.render_with_terminal_height(
            layout.scrollback,
            frame_area.height,
            frame.buffer_mut(),
        );
        if view
            .slots()
            .any(|slot| slot == runie_tui::view::Slot::CompactModeHint)
            && runie_tui::layout::grok_small_screen_tip_visible(frame_area.height)
        {
            render_compact_hint(layout.prompt, frame.buffer_mut(), status.theme());
        }
        Widget::render(
            PromptWidget::from_model_snapshot(document.props.prompt.clone()),
            layout.prompt,
            frame.buffer_mut(),
        );
        if view
            .slots()
            .any(|slot| slot == runie_tui::view::Slot::ShortcutsOverlay)
        {
            runie_tui::widgets::shortcuts::render(frame.area(), frame.buffer_mut(), status.theme());
        }
        if view
            .slots()
            .any(|slot| slot == runie_tui::view::Slot::CommandPaletteOverlay)
        {
            render_command_palette(
                frame.area(),
                frame.buffer_mut(),
                &document.props.ui.command_palette_query,
                document.props.ui.command_palette_index,
                status.theme(),
            );
        }
        let header = &document.props.header;
        render_header(
            layout.header,
            frame.buffer_mut(),
            &header.meter,
            header.theme,
        );
        runie_tui::terminal_color::quantize_buffer(frame.buffer_mut(), color_level);
        frame.set_cursor_position(
            PromptWidget::from_model_snapshot(document.props.prompt.clone())
                .cursor_position(layout.prompt),
        );
    })?;

    let (mut renderer_handle, renderer_shutdown) = app.spawn_renderer();

    // OWNER: interactive input actor; the mailbox and worker live until the
    // terminal loop exits, and the owned task is dropped on shutdown.
    let (input_tx, mut input_rx) = mpsc::channel::<InputEvent>(32);
    let (input_config_tx, mut input_config_rx) = mpsc::channel::<InputConfig>(4);
    let _input_owner = runie_core::spawn_owned_worker!(async move {
        let mut input = EventStream::new();
        let terminal_brand = std::env::var("TERM_PROGRAM")
            .or_else(|_| std::env::var("TERM"))
            .unwrap_or_else(|_| "unknown".into());
        let remuxed = ["TMUX", "STY", "ZELLIJ"]
            .into_iter()
            .any(|name| std::env::var_os(name).is_some());
        let scroll_speed = std::env::var("RUNIE_SCROLL_SPEED")
            .ok()
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(50);
        let inverted = std::env::var("RUNIE_INVERT_SCROLL")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        let scroll_mode = match std::env::var("RUNIE_SCROLL_MODE").ok().as_deref() {
            Some("wheel") => runie_tui_model::ScrollMode::Wheel,
            Some("trackpad") => runie_tui_model::ScrollMode::Trackpad,
            _ => runie_tui_model::ScrollMode::Auto,
        };
        let scroll_normalizer =
            runie_tui_model::ScrollNormalizer::for_terminal_context(&terminal_brand, remuxed)
                .with_speed(scroll_speed)
                .with_inversion(inverted)
                .with_mode(scroll_mode);
        const INPUT_SCROLL_VIEWPORT_ROWS: u16 = 24;
        let mut scroll_flush =
            runie_tui_model::ScrollFlushState::new(scroll_normalizer, INPUT_SCROLL_VIEWPORT_ROWS);
        let mut selection_origin = (0_u16, 0_u16);
        let scroll_epoch = Instant::now();
        let mut cadence = tokio::time::interval(Duration::from_millis(
            runie_tui_model::DEFAULT_SCROLL_FLUSH_CADENCE_MS,
        ));
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                result = input.next() => {
                    let Some(result) = result else { break };
                    let Ok(event) = result else { continue };
                    match event {
                        Event::Key(key) => {
                            if input_tx.send(InputEvent::Key(key)).await.is_err() { break; }
                        }
                        Event::Mouse(mouse) => {
                            if let Some(selection) = mouse_selection_input(
                                mouse.kind,
                                mouse.row,
                                mouse.column,
                                selection_origin,
                            ) {
                                if input_tx.send(selection).await.is_err() {
                                    break;
                                }
                                continue;
                            }
                            let direction = match mouse.kind {
                                MouseEventKind::ScrollUp => runie_tui_model::ScrollDirection::Up,
                                MouseEventKind::ScrollDown => runie_tui_model::ScrollDirection::Down,
                                _ => continue,
                            };
                            let at_ms = scroll_epoch.elapsed().as_millis() as u64;
                            let (next, _) = scroll_flush.input_at(at_ms, direction);
                            scroll_flush = next;
                        }
                        _ => {}
                    }
                }
                config = input_config_rx.recv() => {
                    let Some(config) = config else { break; };
                    match config {
                        InputConfig::ScrollViewport(rows) => {
                            scroll_flush = scroll_flush.with_viewport_rows(rows);
                        }
                        InputConfig::SelectionOrigin { row, column } => {
                            selection_origin = (row, column);
                        }
                    }
                }
                _ = cadence.tick() => {
                    let at_ms = scroll_epoch.elapsed().as_millis() as u64;
                    if scroll_flush.flush_due(at_ms) {
                        let (next, flush) = scroll_flush.flush_at(at_ms);
                        scroll_flush = next;
                        if flush.lines != 0 && input_tx.send(InputEvent::Mouse(flush.lines)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
        let (_, _) = scroll_flush.finalize();
    });

    let mut tick = tokio::time::interval(Duration::from_millis(50));

    // Routing for a single key event. The helper is invoked synchronously
    // from the input arm so every key crosses the prompt/UI actor mailbox
    // and the render frame observes the reduced state without waiting for
    // the next animation tick. The broadcast receiver is moved in and
    // returned back so the caller retains ownership across the await.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "key routing table is intentionally co-located"
    )]
    async fn dispatch_key(
        app: &App,
        mut ui_commands: broadcast::Receiver<UiCommand>,
        renderer_shutdown: &watch::Sender<bool>,
        renderer_handle: &mut JoinHandle<()>,
        key: KeyEvent,
    ) -> Result<broadcast::Receiver<UiCommand>, AppExit> {
        if app.ui.snapshot().model_selector_open {
            match key.code {
                KeyCode::Esc => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorEscape)
                        .await;
                }
                KeyCode::Backspace => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorBackspace)
                        .await;
                }
                KeyCode::Up => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorMove(-1))
                        .await;
                }
                KeyCode::Down => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorMove(1))
                        .await;
                }
                KeyCode::Tab => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorToggleScope)
                        .await;
                }
                KeyCode::Enter => {
                    let _ = app.activate_model_selector().await;
                }
                KeyCode::Char(ch) if key.modifiers.is_empty() => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorChar(ch))
                        .await;
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        if app.ui.snapshot().command_palette_open && key.code == KeyCode::Esc {
            app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteEscape)
                .await;
            return Ok(ui_commands);
        }
        if app.ui.snapshot().command_palette_open {
            match key.code {
                KeyCode::Char(ch) if key.modifiers.is_empty() => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteChar(ch))
                        .await;
                }
                KeyCode::Backspace => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteBackspace)
                        .await;
                }
                KeyCode::Up => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteMove(-1))
                        .await;
                }
                KeyCode::Down => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteMove(1))
                        .await;
                }
                KeyCode::Enter => {
                    let _ = app.activate_command_palette().await;
                    match ui_commands.recv().await {
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::NewSession)) => {
                            let _ = app.reset_session().await;
                        }
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::KeyboardShortcuts)) => {
                            app.toggle_shortcuts().await;
                        }
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::Quit)) => {
                            let _ = renderer_shutdown.send(true);
                            let _ = renderer_handle.await;
                            return Err(AppExit::Quit);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        let status = app.status_snapshot().current().clone();
        let streaming = matches!(status, Status::Thinking | Status::Streaming);
        let prompt_model = app.model_snapshot().prompt;
        match map_key(key, !prompt_model.is_empty(), streaming) {
            Action::ClearPrompt => {
                app.prompt.clear().await;
                return Ok(ui_commands);
            }
            Action::Abort => {
                app.loop_actor.abort().await;
                return Ok(ui_commands);
            }
            Action::ModeCycle => {
                app.prompt.cycle_mode().await;
                return Ok(ui_commands);
            }
            Action::OpenShortcuts => {
                app.toggle_shortcuts().await;
                return Ok(ui_commands);
            }
            Action::OpenCommandPalette => {
                app.toggle_command_palette().await;
                return Ok(ui_commands);
            }
            Action::OpenModelSelector => {
                app.toggle_model_selector().await;
                return Ok(ui_commands);
            }
            Action::OpenFileSearch => {
                app.prompt.open_file_search().await;
                return Ok(ui_commands);
            }
            Action::ToggleFold => {
                app.toggle_selected_tool_fold().await;
                return Ok(ui_commands);
            }
            Action::SelectNextTool => {
                app.select_next_tool().await;
                return Ok(ui_commands);
            }
            Action::SelectPreviousTool => {
                app.select_previous_tool().await;
                return Ok(ui_commands);
            }
            Action::SelectNextEntry => {
                app.select_next_entry().await;
                return Ok(ui_commands);
            }
            Action::SelectPreviousEntry => {
                app.select_previous_entry().await;
                return Ok(ui_commands);
            }
            Action::ExtendSelectionNext => {
                app.extend_selection(1).await;
                return Ok(ui_commands);
            }
            Action::ExtendSelectionPrevious => {
                app.extend_selection(-1).await;
                return Ok(ui_commands);
            }
            Action::ScrollUp => {
                app.scroll_scrollback_by(-1).await;
                return Ok(ui_commands);
            }
            Action::ScrollDown => {
                app.scroll_scrollback_by(1).await;
                return Ok(ui_commands);
            }
            Action::Quit => {
                let _ = renderer_shutdown.send(true);
                let _ = renderer_handle.await;
                return Err(AppExit::Quit);
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
                        if let Some(command) = parse_mappable_builtin_command(&text) {
                            if matches!(command, MappableBuiltinCommand::Quit) {
                                return Err(AppExit::Quit);
                            }
                            let _ = app.route_mappable_command(command).await;
                            return Ok(ui_commands);
                        }
                        if is_quit_command(&text) {
                            return Err(AppExit::Quit);
                        }
                        let _ = app
                            .handle_prompt_outcome(PromptOutcome::Submitted(text))
                            .await;
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
        Ok(ui_commands)
    }

    // One terminal frame. Called from the input arm after a key is
    // dispatched and from the tick arm for animation/agent activity
    // refreshes, so a fast key burst is visible key-by-key instead of
    // after a tick-batched draw.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "terminal draw is intentionally co-located with state update"
    )]
    async fn render_frame(
        app: &App,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        input_config_tx: &mpsc::Sender<InputConfig>,
        color_level: runie_tui::terminal_color::ColorLevel,
        placeholder_hidden: &mut bool,
    ) -> Result<(), String> {
        let model = app.model_snapshot();
        let settled = matches!(model.status.state, Status::Ready) && !model.feed.is_empty();
        if settled != *placeholder_hidden {
            app.prompt.set_placeholder_visible(!settled).await;
            *placeholder_hidden = settled;
        }
        if let Err(e) = terminal.draw(|f| {
            use ratatui::layout::Rect;
            use ratatui::widgets::Widget;
            use runie_tui::layout::chat_layout;
            let layout = chat_layout(f.area());
            let _ = input_config_tx.try_send(InputConfig::ScrollViewport(layout.scrollback.height));
            let _ = input_config_tx.try_send(InputConfig::SelectionOrigin {
                row: layout.scrollback.y,
                column: layout.scrollback.x,
            });
            // Wrap each render in catch_unwind so a widget bug
            // doesn't kill the binary — log + continue.
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let frame_area = f.area();
                let buf = f.buffer_mut();
                let document = App::view_document_from_model(&model);
                let view = &document.root;
                let status = StatusBar::from_model_snapshot(document.props.status.clone());
                buf.set_style(
                    frame_area,
                    runie_tui::appearance::background_style_for(status.theme()),
                );
                let turn_status = status.turn_status();
                if matches!(status.current(), Status::Ready) {
                    render_live_ready_footer(layout.status, buf, status.theme());
                } else {
                    status.render(layout.status, buf);
                }
                let mut scrollback = Scrollback::from_model_snapshot(document.props.feed.clone());
                scrollback.set_live_grok_layout(true);
                scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
                scrollback.normalize_live_completed_assistants();
                scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
                scrollback.render_with_terminal_height(layout.scrollback, frame_area.height, buf);
                if view
                    .slots()
                    .any(|slot| slot == runie_tui::view::Slot::CompactModeHint)
                    && runie_tui::layout::grok_small_screen_tip_visible(frame_area.height)
                {
                    render_compact_hint(layout.prompt, buf, status.theme());
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
                let prompt = PromptWidget::from_model_snapshot(document.props.prompt.clone());
                let cursor_position = prompt.cursor_position(layout.prompt);
                Widget::render(prompt, layout.prompt, buf);
                if view
                    .slots()
                    .any(|slot| slot == runie_tui::view::Slot::ShortcutsOverlay)
                {
                    runie_tui::widgets::shortcuts::render(frame_area, buf, status.theme());
                }
                if view
                    .slots()
                    .any(|slot| slot == runie_tui::view::Slot::CommandPaletteOverlay)
                {
                    render_command_palette(
                        frame_area,
                        buf,
                        &document.props.ui.command_palette_query,
                        document.props.ui.command_palette_index,
                        status.theme(),
                    );
                }
                if view
                    .slots()
                    .any(|slot| slot == runie_tui::view::Slot::ModelSelectorOverlay)
                {
                    render_model_selector(frame_area, buf, &document.props.ui, status.theme());
                }
                if document.props.ui.session_info_open {
                    let session = app.session_snapshot();
                    runie_tui::widgets::SessionInfoWidget::new(&session)
                        .with_theme(status.theme())
                        .render(frame_area, buf);
                }
                let header = &document.props.header;
                render_header(layout.header, buf, &header.meter, header.theme);
                runie_tui::terminal_color::quantize_buffer(buf, color_level);
                f.set_cursor_position(cursor_position);
                let _ = Rect::default();
            }));
            if let Err(e) = res {
                eprintln!("render panic: {:?}", e);
                std::panic::resume_unwind(e);
            }
        }) {
            return Err(format!("draw: {e}"));
        }
        Ok(())
    }

    loop {
        tokio::select! {
            biased;
            input = input_rx.recv() => {
                let Some(input) = input else { return Ok(AppExit::Quit) };
                match input {
                    InputEvent::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            ui_commands = match dispatch_key(
                                &app,
                                ui_commands,
                                &renderer_shutdown,
                                &mut renderer_handle,
                                key,
                            )
                            .await
                            {
                                Ok(rx) => rx,
                                Err(exit) => return Ok(exit),
                            };
                            if let Err(err) = render_frame(
                                &app,
                                terminal,
                                &input_config_tx,
                                color_level,
                                &mut placeholder_hidden,
                            )
                            .await
                            {
                                return Ok(AppExit::Error(err));
                            }
                        }
                    }
                    InputEvent::Mouse(delta) => app.scroll_scrollback_by(delta).await,
                    InputEvent::MouseSelectionStart(row, column) => {
                        app.mouse_selection_start(runie_tui::widgets::CellPosition { row, column }).await;
                    }
                    InputEvent::MouseSelectionExtend(row, column) => {
                        app.mouse_selection_extend(runie_tui::widgets::CellPosition { row, column }).await;
                    }
                    InputEvent::MouseSelectionCommit => app.mouse_selection_commit().await,
                }
            }
            _ = tick.tick() => {
                // Animation and agent activity refreshes run on the tick
                // cadence; key events render in the input arm so a fast
                // burst is visible key-by-key instead of waiting for the
                // tick to drain a queued batch.
                if let Err(err) =
                    render_frame(
                        &app,
                        terminal,
                        &input_config_tx,
                        color_level,
                        &mut placeholder_hidden,
                    )
                    .await
                {
                    return Ok(AppExit::Error(err));
                }
            }
        }
    }
}

#[allow(dead_code)]
fn _key_marker(_k: KeyEvent) {}

#[cfg(test)]
mod tests {
    use super::{
        current_branch, mouse_selection_input, render_header, render_live_ready_footer,
        repository_label, InputEvent,
    };
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    #[test]
    fn header_uses_cached_repository_branch() {
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        render_header(
            area,
            &mut buffer,
            "0 / 500K",
            runie_core::types::ThemeKind::GrokNight,
        );
        let row = (0..area.width)
            .filter_map(|x| buffer.cell((x, 0)))
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(row.contains(current_branch()));
        assert!(row.contains(&repository_label()));
    }

    #[test]
    fn live_footer_advances_by_terminal_cells_for_unicode_separators() {
        let area = Rect::new(0, 0, 40, 1);
        let mut buffer = Buffer::empty(area);
        render_live_ready_footer(area, &mut buffer, runie_core::types::ThemeKind::GrokNight);

        // "Shift+Tab" is nine cells and ":mode  │  " is ten cells, so the
        // second shortcut starts at cell nineteen, not at the UTF-8 byte
        // offset produced by `str::len()`.
        let shortcut = buffer.cell((19, 0)).expect("second shortcut");
        assert_eq!(shortcut.symbol(), "C");
        assert!(shortcut.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn mouse_selection_input_is_pure_and_origin_relative() {
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Down(MouseButton::Left), 12, 18, (5, 7)),
            Some(InputEvent::MouseSelectionStart(7, 11))
        ));
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Drag(MouseButton::Left), 8, 10, (5, 7)),
            Some(InputEvent::MouseSelectionExtend(3, 3))
        ));
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Up(MouseButton::Left), 8, 10, (5, 7)),
            Some(InputEvent::MouseSelectionCommit)
        ));
        assert!(mouse_selection_input(MouseEventKind::Moved, 8, 10, (5, 7)).is_none());
    }

    /// Pin down the contract that the live main loop's `dispatch_key`
    /// helper relies on: every Press key in a burst crosses the
    /// `PromptActor` mailbox in order and updates the prompt snapshot
    /// synchronously, so a render after each key is sufficient to make
    /// the typed text visible. A regression that batches keys through a
    /// single awaited reducer would surface here as a stale snapshot
    /// after `N` awaits.
    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "self-contained App wiring mirrors the runie.rs bin for fidelity"
    )]
    async fn prompt_actor_reduces_each_press_key_independently() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
        use runie_core::events::EventBus;
        use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
        use runie_core::provider::ProviderActor;
        use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
        use runie_core::r#loop::{LoopActor, LoopDeps};
        use runie_core::state::AgentStateActor;
        use runie_core::tools::executor::ToolExecHooks;
        use runie_core::tools::{ToolExecutorActor, ToolRegistry};
        use runie_core::types::{AgentContext, Model, SimpleStreamOptions, ToolExecutionMode};
        use runie_tui::app::App;
        use std::sync::Arc;

        struct NoopStream;
        #[async_trait::async_trait]
        impl StreamFn for NoopStream {
            async fn stream(
                &self,
                _model: &Model,
                _context: &AgentContext,
                _options: Option<SimpleStreamOptions>,
            ) -> Result<AssistantMessageEventStream, StreamError> {
                use futures::stream;
                Ok(Box::pin(stream::iter(std::iter::empty())))
            }
        }

        let bus = EventBus::new();
        let state = AgentStateActor::new();
        let steering = SteeringQueueActor::new();
        let follow_up = FollowUpQueueActor::new();
        let tool_executor = ToolExecutorActor::new_live(Arc::new(ToolRegistry::new()));
        let provider = ProviderActor::new(Arc::new(NoopStream));
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
        let app = App::new(actor, bus);

        let mut expected = String::new();
        for ch in "hello".chars() {
            let key = KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            };
            app.prompt.handle_key(key).await;
            expected.push(ch);
            // The typed text must appear in the prompt snapshot after a
            // single `handle_key` await, not deferred to a later tick. The
            // `dispatch_key` helper relies on this so a render right after
            // a key produces the visible character.
            let text = app.prompt.snapshot().text();
            assert!(
                text.contains(&expected),
                "after typing {expected:?} the prompt text {text:?} must include it"
            );
        }
    }
}
