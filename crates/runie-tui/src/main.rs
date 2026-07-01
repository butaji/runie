//! Runie Terminal — Non-blocking event loop with render actor
//!
//! Architecture (impossible to block by design):
//!   1. Event loop: single-threaded async, only async ops
//!   2. State: owned by event loop, mutable borrow per event
//!   3. Snapshot: immutable frame description (the UI DSL)
//!   4. Render actor: owns Terminal, receives Snapshots via channel
//!   5. If render is slow, old Snapshots are dropped — event loop never waits
//!
//! Event Bus Integration:
//!   - EventBus<Event> for cross-component communication
//!   - SessionActor subscribes to bus, persists durable events to JSONL

use clap::Parser;
use futures::StreamExt;
use runie_agent::AgentActorFactoryImpl;
use runie_core::actors::leader::{Leader, LeaderHandle};
use runie_core::actors::RactorTurnHandle;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::tracing_init;
use runie_core::{AppState, Snapshot};
use runie_provider::BuiltProviderFactory;
use runie_tui::{
    app_init, keymap, terminal, terminal_setup, theme, ui,
    ui_actor::{AgentHandleBox, LeaderAgentActorHandle, UiActor},
};
use std::{collections::HashMap, io, time::Duration};
use tokio::sync::{mpsc, oneshot, watch};

/// Runie TUI CLI arguments.
#[derive(Parser, Debug)]
#[command(name = "runie", version)]
struct Cli {
    /// Show dry-run preview without starting the TUI.
    #[arg(long)]
    dry_run: bool,
    /// Alias for --dry-run (preview mode).
    #[arg(long, hide = true)]
    preview: bool,
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::DisableFocusChange,
            crossterm::terminal::LeaveAlternateScreen,
        );
        let _ = terminal_setup::reset_keyboard_enhancements(&mut std::io::stdout());
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    // Install human-panic hook for crash reports.
    human_panic::setup_panic!();

    tracing_init::init();

    let cli = Cli::parse();
    if cli.dry_run || cli.preview {
        let report = runie_core::run_dry_run(&runie_core::Config::load(None));
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;

    // Create the event bus upfront so that UiActor can subscribe before actors emit
    // initial facts (ConfigLoaded, TrustLoaded, HistoryLoaded).
    let bus = runie_core::bus::EventBus::<Event>::new(1000);

    // Subscribe UiActor to the bus BEFORE starting the leader so it receives
    // ConfigLoaded and other initial facts that are emitted during actor spawn.
    let bus_rx = bus.subscribe();

    let leader = Leader::new();
    let agent_factory = std::sync::Arc::new(AgentActorFactoryImpl);
    let provider_factory = std::sync::Arc::new(BuiltProviderFactory);
    let leader_handle = leader
        .start_with_bus(provider_factory, agent_factory, bus.clone())
        .await
        .map_err(|e| io::Error::other(format!("leader bootstrap failed: {}", e)))?;

    let mut state = AppState::default();
    state.set_actor_handles(leader_handle.clone());
    app_init::bootstrap(&mut state).await;

    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
    init_terminal_state(&mut state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    spawn_background_tasks(
        terminal,
        state,
        terminal_caps,
        leader_handle.clone(),
        bus_rx,
        shutdown_tx,
    )
    .await;

    shutdown_rx
        .await
        .map_err(|_| io::Error::other("shutdown signal dropped"))?;
    Ok(())
}

fn init_terminal_state(state: &mut AppState) {
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.set_last_content_width(width);
        state.set_last_visible_height(height);
    }
}

/// Forwarder: raw events come in via `input_rx` and are routed through InputMsg
/// to the leader's InputActor. InputActor is the single source of truth for input
/// state; UiActor applies state from InputChanged events.
async fn input_forwarder_task(
    mut input_rx: mpsc::Receiver<Event>,
    input_handle: runie_core::actors::RactorInputHandle,
    submit_tx: mpsc::Sender<Event>,
) {
    use runie_core::actors::InputMsg;
    // Local helper: fire-and-forget send to InputActor (sync send_message with no-op await).
    async fn send_input(handle: &runie_core::actors::RactorInputHandle, msg: InputMsg) {
        let _ = handle.send_message(msg);
    }
    while let Some(evt) = input_rx.recv().await {
        match &evt {
            Event::Input(c) => {
                send_input(&input_handle, InputMsg::InsertChar(*c)).await;
            }
            Event::Backspace => {
                send_input(&input_handle, InputMsg::Backspace).await;
            }
            Event::Newline => {
                send_input(&input_handle, InputMsg::Newline).await;
            }
            Event::DeleteWord => {
                send_input(&input_handle, InputMsg::DeleteWord).await;
            }
            Event::DeleteToEnd => {
                send_input(&input_handle, InputMsg::DeleteToEnd).await;
            }
            Event::DeleteToStart => {
                send_input(&input_handle, InputMsg::DeleteToStart).await;
            }
            Event::CursorLeft => {
                send_input(&input_handle, InputMsg::CursorLeft).await;
            }
            Event::CursorRight => {
                send_input(&input_handle, InputMsg::CursorRight).await;
            }
            Event::CursorStart => {
                send_input(&input_handle, InputMsg::CursorStart).await;
            }
            Event::CursorEnd => {
                send_input(&input_handle, InputMsg::CursorEnd).await;
            }
            Event::CursorWordLeft => {
                send_input(&input_handle, InputMsg::CursorWordLeft).await;
            }
            Event::CursorWordRight => {
                send_input(&input_handle, InputMsg::CursorWordRight).await;
            }
            Event::HistoryPrev => {
                send_input(&input_handle, InputMsg::HistoryPrev).await;
            }
            Event::HistoryNext => {
                send_input(&input_handle, InputMsg::HistoryNext).await;
            }
            Event::Undo => {
                send_input(&input_handle, InputMsg::Undo).await;
            }
            Event::Redo => {
                send_input(&input_handle, InputMsg::Redo).await;
            }
            Event::Paste(s) => {
                send_input(&input_handle, InputMsg::Paste(s.clone())).await;
            }
            Event::PasteImage => {
                send_input(&input_handle, InputMsg::PasteImage).await;
            }
            Event::Submit => {
                // Submit must be handled by UiActor so it can capture the input
                // content before InputActor clears it. Forward it on a dedicated
                // channel; UiActor will then send InputMsg::Submit itself.
                let _ = submit_tx.send(evt).await;
            }
            // Quit, ForceQuit, and Abort must reach UiActor even during an active
            // turn. Forward them on the same submit channel; UiActor handles them
            // at top priority before any turn logic.
            Event::Quit | Event::ForceQuit | Event::Abort => {
                let _ = submit_tx.send(evt).await;
            }
            _ => {}
        }
    }
}

async fn spawn_background_tasks(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    state: AppState,
    caps: terminal::caps::TermCaps,
    leader_handle: LeaderHandle,
    bus_rx: runie_core::bus::Receiver<Event>,
    shutdown_tx: oneshot::Sender<()>,
) {
    let bus = leader_handle.event_bus().clone();
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (submit_tx, submit_rx) = mpsc::channel::<Event>(16);

    let (kb_tx, kb_rx) = watch::channel(state.config().keybindings().clone());
    tokio::spawn(input_forwarder_task(input_rx, leader_handle.input.clone(), submit_tx));

    // UiActor was created before start_with_bus() with a NoOp agent handle and
    // the pre-subscribed bus_rx. Install the real agent handle and run it.
    let mut ui_actor = spawn_ui_actor_with_external_rx(
        state,
        bus_rx,
        leader_handle.turn.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui_actor.set_agent_handle(AgentHandleBox::Leader(LeaderAgentActorHandle::new(
        leader_handle.agent.clone(),
    )));
    let render_rx = ui_actor.take_render_rx();
    tokio::spawn(async move {
        ui_actor.run_with_external_rx(submit_rx).await;
    });

    spawn_agent_tasks(input_tx, kb_rx, terminal, render_rx, caps);
}

fn spawn_agent_tasks(
    input_tx: mpsc::Sender<Event>,
    kb_rx: watch::Receiver<HashMap<String, String>>,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    render_rx: watch::Receiver<Snapshot>,
    caps: terminal::caps::TermCaps,
) {
    tokio::spawn(input_reader(input_tx, kb_rx));
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || render_loop(terminal, rx, caps));
    tokio::spawn(render_forwarder(render_rx, tx));
}

async fn render_forwarder(
    mut render_rx: watch::Receiver<Snapshot>,
    tx: std::sync::mpsc::SyncSender<Snapshot>,
) {
    loop {
        let snap = render_rx.borrow_and_update().clone();
        if tx.try_send(snap).is_err() {
            // Render thread is backed up — skip this frame.
        }
        if render_rx.changed().await.is_err() {
            break;
        }
    }
}

fn render_loop(
    mut terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    rx: std::sync::mpsc::Receiver<Snapshot>,
    caps: terminal::caps::TermCaps,
) {
    const FRAME_TIME: Duration = Duration::from_millis(16);
    let mut last_size: Option<(u16, u16)> = None;

    loop {
        let mut snap = match rx.recv_timeout(FRAME_TIME) {
            Ok(s) => s,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        while let Ok(s) = rx.try_recv() {
            snap = s;
        }

        let new_size = terminal
            .size()
            .map(|r| (r.width, r.height))
            .unwrap_or((0, 0));
        if last_size != Some(new_size) {
            let _ = terminal.clear();
            last_size = Some(new_size);
        }
        theme::set_current_theme_with_caps(&snap.theme_name, caps);
        let _ = terminal.draw(|f| ui::draw_snapshot(f, &snap));
    }
}

/// Create a UiActor with a pre-subscribed bus receiver.
/// Use this when the bus receiver was created before `Leader::start_with_bus()` returns,
/// so that UiActor receives initial facts like `ConfigLoaded`.
/// Call `UiActor::set_agent_handle()` after `Leader::start_with_bus()` returns.
#[allow(clippy::too_many_arguments)]
fn spawn_ui_actor_with_external_rx(
    state: AppState,
    bus_rx: runie_core::bus::Receiver<Event>,
    turn_handle: RactorTurnHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
    caps: terminal::caps::TermCaps,
) -> UiActor {
    UiActor::with_external_bus_rx(state, bus_rx, turn_handle, kb_tx, bus, shutdown_tx, caps)
}

async fn input_reader(
    input_tx: mpsc::Sender<Event>,
    mut kb_rx: watch::Receiver<HashMap<String, String>>,
) {
    let mut reader = crossterm::event::EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        let bindings = kb_rx.borrow_and_update().clone();
        if let Some(evt) = keymap::convert_event(&event, &bindings) {
            let is_quit = matches!(evt, Event::Quit | Event::Reset);
            if input_tx.send(evt).await.is_err() {
                break;
            }
            if is_quit {
                break;
            }
        }
    }
}
