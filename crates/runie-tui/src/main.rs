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

use futures::StreamExt;
use runie_agent::AgentActor;
use runie_core::actors::{
    ActorHandles, ConfigActor, FffIndexerActor, FffIndexerHandle, IoActor,
    ProviderActor, SessionActor, TurnActor,
};
use runie_core::actors::permission::RactorPermissionActor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::{AppState, Snapshot};
use runie_provider::DynProviderFactory;
use runie_tui::{app_init, keymap, terminal, terminal_setup, theme, ui, ui_actor::UiActor};
use std::{collections::HashMap, io, time::Duration};
use tokio::sync::{mpsc, oneshot, watch};

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

// Note: Durable event append is now handled by the unified SessionActor
// spawned in bootstrap_app(). No separate persistence actor needed.

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(report) = runie_tui::dry_run::run_from_args(&args) {
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;
    let bus = EventBus::<Event>::new(100);
    let bootstrap = bootstrap_app(bus.clone()).await;
    let mut state = bootstrap.0;
    let actor_handles = bootstrap.1;

    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
    init_terminal_state(&mut state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    spawn_background_tasks(
        terminal,
        state,
        terminal_caps,
        bus,
        shutdown_tx,
        actor_handles,
    );

    shutdown_rx
        .await
        .map_err(|_| io::Error::other("shutdown signal dropped"))?;
    Ok(())
}

async fn bootstrap_app(bus: EventBus<Event>) -> (AppState, ActorHandles) {
    let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, _provider_actor) = spawn_provider_actor(&bus, &config_handle);
    // Unified SessionActor: owns trust, history, session CRUD, and durable event append
    let (session_handle, _session_actor) = SessionActor::spawn(bus.clone());
    let (io_handle, _io_actor) = IoActor::spawn(bus.clone());
    let (permission_handle, _permission_actor) = RactorPermissionActor::spawn(bus.clone()).await;
    // InputActor owns the input buffer, cursor, history, undo/redo.
    let (input_handle, _input_actor) = runie_core::actors::InputActor::spawn(bus.clone()).await;
    // TurnActor owns turn lifecycle, queues, and token tracking.
    let (turn_handle, _turn_actor) = TurnActor::spawn(bus.clone());
    let mut state = AppState::default();
    // Build the ActorHandles registry — this is the single source of truth
    // for all actor senders. It replaces the old loose config_tx/provider_tx/... fields.
    let mut handles = ActorHandles {
        config: Some(config_handle),
        provider: Some(provider_handle),
        session: Some(session_handle.clone()),
        io: Some(io_handle),
        permission: Some(permission_handle),
        input: Some(input_handle),
        turn: Some(turn_handle.clone()),
        ..Default::default()
    };
    state.set_actor_handles(handles.clone());
    app_init::bootstrap(&mut state).await;
    // Spawn FffIndexerActor with the current working directory as the project root.
    let project_root = std::env::current_dir().unwrap_or_default();
    let data_dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    if let Ok((tx, _actor_handle)) =
        FffIndexerActor::spawn(project_root, data_dir, bus.clone())
    {
        handles.fff_indexer = Some(FffIndexerHandle::new(tx));
        state.set_actor_handles(handles.clone());
    }
    (state, handles)
}

fn spawn_provider_actor(
    bus: &EventBus<Event>,
    config_handle: &runie_core::actors::ConfigActorHandle,
) -> (
    runie_core::actors::ProviderActorHandle,
    runie_core::actors::ActorHandle,
) {
    ProviderActor::spawn(
        bus.clone(),
        config_handle.clone(),
        std::sync::Arc::new(DynProviderFactory),
    )
}

fn init_terminal_state(state: &mut AppState) {
    if let Ok((width, height)) = crossterm::terminal::size() {
        state.set_last_content_width(width);
        state.set_last_visible_height(height);
    }
}

fn spawn_background_tasks(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut state: AppState,
    caps: terminal::caps::TerminalCapabilities,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
    handles: ActorHandles,
) {
    let handles = setup_actor_channels(&handles, &mut state, &bus);
    let (render_tx, render_rx) = watch::channel(state.snapshot());
    let (kb_tx, kb_rx) = watch::channel(state.config().keybindings().clone());
    spawn_input_forwarder(handles.input_rx, bus.clone());
    spawn_agent_tasks(handles.input_tx, kb_rx, terminal, render_rx, bus.clone(), caps);
    spawn_ui_actor(state, render_tx, handles.agent_handle, handles.persistence_handle, handles.turn_handle, kb_tx, bus.clone(), shutdown_tx, caps);
    tokio::spawn(handles.agent_actor);
    tokio::spawn(handles.turn_actor);
}

struct ActorChannels {
    input_tx: mpsc::Sender<Event>,
    input_rx: mpsc::Receiver<Event>,
    agent_handle: runie_agent::AgentActorHandle,
    agent_actor: runie_core::actors::ActorHandle,
    persistence_handle: runie_core::actors::SessionActorHandle,
    turn_handle: runie_core::actors::TurnActorHandle,
    turn_actor: runie_core::actors::ActorHandle,
}

fn setup_actor_channels(
    handles: &ActorHandles,
    _state: &mut AppState,
    bus: &EventBus<Event>,
) -> ActorChannels {
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (agent_handle, agent_actor) = AgentActor::spawn(
        bus.clone(),
        handles.provider.clone().expect("ProviderActor must be spawned"),
        handles.permission.clone().expect("PermissionActor must be spawned"),
    );
    let (turn_handle, turn_actor) = TurnActor::spawn(bus.clone());
    ActorChannels {
        input_tx,
        input_rx,
        agent_handle,
        agent_actor,
        persistence_handle: handles.session.clone().expect("SessionActor must be spawned"),
        turn_handle,
        turn_actor,
    }
}

fn spawn_input_forwarder(mut input_rx: mpsc::Receiver<Event>, bus: EventBus<Event>) {
    tokio::spawn(async move {
        while let Some(evt) = input_rx.recv().await {
            bus.publish(evt);
        }
    });
}

fn spawn_agent_tasks(
    input_tx: mpsc::Sender<Event>,
    kb_rx: watch::Receiver<HashMap<String, String>>,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    render_rx: watch::Receiver<Snapshot>,
    _bus: EventBus<Event>,
    caps: terminal::caps::TerminalCapabilities,
) {
    tokio::spawn(input_reader(input_tx, kb_rx));
    // Move terminal IO off the Tokio runtime — use a dedicated OS thread.
    // Use sync_channel with buffer size 1 for non-blocking sends.
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
        // Use try_send to avoid blocking the async event loop.
        // If the render thread is busy, skip this frame and process the next one.
        if tx.try_send(snap).is_err() {
            // Render thread is backed up — skip this frame, let it catch up.
            // This prevents input latency when the render is slow.
        }
        if render_rx.changed().await.is_err() {
            break;
        }
    }
}

fn render_loop(
    mut terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    rx: std::sync::mpsc::Receiver<Snapshot>,
    caps: terminal::caps::TerminalCapabilities,
) {
    const FRAME_TIME: Duration = Duration::from_millis(16);
    let mut last_size: Option<(u16, u16)> = None;

    loop {
        // Wait for a snapshot, but no more than one frame period.
        // This caps the render thread at ~60 FPS and prevents it from
        // burning CPU redrawing on every tiny event burst.
        let mut snap = match rx.recv_timeout(FRAME_TIME) {
            Ok(s) => s,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        // Drain any newer snapshots that arrived while we were waiting and
        // keep only the latest one. Older intermediate frames are dropped.
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

#[allow(clippy::too_many_arguments)]
fn spawn_ui_actor(
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: runie_agent::AgentActorHandle,
    persistence_handle: runie_core::actors::SessionActorHandle,
    turn_handle: runie_core::actors::TurnActorHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
    caps: terminal::caps::TerminalCapabilities,
) {
    // UiActor is the sole owner of AppState and the only runtime mutator.
    // Late subscriber catch-up is handled by SessionActor disk-replay at startup.
    let ui_sub = bus.subscribe();
    tokio::spawn(
        UiActor::new(
            state,
            render_tx,
            agent_handle,
            persistence_handle,
            turn_handle,
            kb_tx,
            bus,
            shutdown_tx,
            caps,
        )
        .run(ui_sub),
    );
}

/// Input reader that sends mapped events to the shared bus via `input_tx`.
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
