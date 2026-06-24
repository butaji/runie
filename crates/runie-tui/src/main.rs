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
use runie_core::actors::{ConfigActor, IoActor, PersistenceActor, ProviderActor, SessionStoreActor};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::session_store::SessionStore;
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

fn generate_session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_else(|e| e.duration().as_nanos());
    format!("session_{}", nanos)
}

fn spawn_session_persistence(bus: &EventBus<Event>) {
    let session_id = generate_session_id();
    let store = SessionStore::new(
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("runie")
            .join("sessions"),
    );
    let actor = runie_core::SessionActor::new(session_id, "main".into(), store);
    tokio::spawn(actor.run_loop(bus.clone()));
}

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
    let provider_handle = bootstrap.1;


    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
    init_terminal_state(&mut state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    spawn_background_tasks(
        terminal,
        state,
        terminal_caps,
        bus,
        shutdown_tx,
        provider_handle,
    );

    shutdown_rx
        .await
        .map_err(|_| io::Error::other("shutdown signal dropped"))?;
    Ok(())
}

async fn bootstrap_app(
    bus: EventBus<Event>,
) -> (AppState, runie_core::actors::ProviderActorHandle) {
    let (config_handle, config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, provider_actor) = spawn_provider_actor(&bus, &config_handle);
    let (persistence_handle, persistence_actor) = PersistenceActor::spawn(bus.clone());
    let (session_store_handle, session_store_actor) = SessionStoreActor::spawn(bus.clone());
    let (io_handle, io_actor) = IoActor::spawn(bus.clone());
    let mut state = AppState {
        config_tx: Some(config_handle.tx().clone()),
        provider_tx: Some(provider_handle.tx().clone()),
        persistence_tx: Some(persistence_handle.clone()),
        session_store_tx: Some(session_store_handle.clone()),
        io_tx: Some(io_handle.clone()),
        ..Default::default()
    };
    app_init::bootstrap(&mut state).await;
    (state, provider_handle)
}

fn spawn_provider_actor(
    bus: &EventBus<Event>,
    config_handle: &runie_core::actors::ConfigActorHandle,
) -> (
    runie_core::actors::ProviderActorHandle,
    runie_core::actor::ActorHandle,
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
    provider_handle: runie_core::actors::ProviderActorHandle,
) {
    let persistence_handle = state
        .persistence_tx
        .clone()
        .expect("PersistenceActor must be spawned before UI");
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (agent_handle, agent_actor) = AgentActor::spawn(
        bus.clone(),
        provider_handle,
        state.approval_registry.clone(),
    );
    let (render_tx, render_rx) = watch::channel(state.snapshot());
    let (kb_tx, kb_rx) = watch::channel(state.config.keybindings.clone());
    spawn_input_forwarder(input_rx, bus.clone());
    spawn_agent_tasks(input_tx, kb_rx, terminal, render_rx, bus.clone(), caps);
    spawn_ui_actor(
        state,
        render_tx,
        agent_handle,
        persistence_handle,
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    spawn_session_persistence(&bus);

    // Keep the agent actor alive until shutdown; its handle aborts on Drop.
    tokio::spawn(async move {
        let _ = agent_actor.await;
    });
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

fn render_forwarder(
    mut render_rx: watch::Receiver<Snapshot>,
    tx: std::sync::mpsc::SyncSender<Snapshot>,
) -> impl std::future::Future<Output = ()> {
    async move {
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

        let new_size = terminal.size().map(|r| (r.width, r.height)).unwrap_or((0, 0));
        if last_size != Some(new_size) {
            let _ = terminal.clear();
            last_size = Some(new_size);
        }
        theme::set_current_theme_with_caps(&snap.theme_name, caps);
        let _ = terminal.draw(|f| ui::draw_snapshot(f, &snap));
    }
}

fn spawn_ui_actor(
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: runie_agent::AgentActorHandle,
    persistence_handle: runie_core::actors::PersistenceActorHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
    caps: terminal::caps::TerminalCapabilities,
) {
    // UiActor is the sole owner of AppState and the only runtime mutator.
    // Subscribe with replay so resuming a session restores prior messages.
    // UiActor MUST subscribe before SessionActor replays durable events.
    let ui_sub = bus.subscribe_with_replay();
    tokio::spawn(
        UiActor::new(
            state,
            render_tx,
            agent_handle,
            persistence_handle,
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


