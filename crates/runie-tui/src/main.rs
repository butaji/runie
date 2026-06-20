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
use runie_core::actor::Actor;
use runie_core::actors::{ConfigActor, ProviderActor};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::session_store::SessionStore;
use runie_core::{AppState, Snapshot};
use runie_provider::DynProviderFactory;
use runie_tui::{app_init, keymap, terminal, terminal_setup, theme, ui, ui_actor::UiActor};
use std::{collections::HashMap, io};
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

fn spawn_session_persistence(bus: &EventBus<Event>) -> mpsc::Sender<()> {
    let session_id = format!(
        "session_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let store = SessionStore::new(
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("runie")
            .join("sessions"),
    );
    let session_actor = runie_core::SessionActor::new(session_id, "main".into(), store);
    let (session_tx, session_rx) = mpsc::channel(1);
    tokio::spawn(session_actor.run(session_rx, bus.clone()));
    session_tx
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(report) = runie_tui::dry_run::run_from_args(&args) {
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;
    let bus = EventBus::<Event>::new(100);
    let (mut state, _config_handle, provider_handle, config_actor, provider_actor) =
        bootstrap_app(bus.clone()).await;

    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
    theme::set_current_theme_with_caps_async(&state.config.theme_name, terminal_caps).await;
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

    // Keep actors alive until shutdown. Dropping the handle aborts the actor.
    let _config_actor = config_actor;
    let _provider_actor = provider_actor;

    shutdown_rx
        .await
        .map_err(|_| io::Error::other("shutdown signal dropped"))?;
    Ok(())
}

async fn bootstrap_app(
    bus: EventBus<Event>,
) -> (
    AppState,
    runie_core::actors::ConfigActorHandle,
    runie_core::actors::ProviderActorHandle,
    runie_core::actor::ActorHandle,
    runie_core::actor::ActorHandle,
) {
    let (config_handle, config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, provider_actor) = spawn_provider_actor(&bus, &config_handle);
    let mut state = AppState {
        config_tx: Some(config_handle.tx().clone()),
        provider_tx: Some(provider_handle.tx().clone()),
        ..Default::default()
    };
    app_init::bootstrap(&mut state).await;
    (state, config_handle, provider_handle, config_actor, provider_actor)
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
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (agent_handle, agent_actor) = AgentActor::spawn(
        bus.clone(),
        provider_handle,
        state.approval_registry.clone(),
    );
    let (render_tx, render_rx) = watch::channel(state.snapshot());
    let (kb_tx, kb_rx) = watch::channel(state.config.keybindings.clone());

    spawn_input_forwarder(input_rx, bus.clone());
    spawn_agent_tasks(
        input_tx,
        kb_rx,
        terminal,
        render_rx,
        bus.clone(),
    );
    spawn_ui_actor(
        state,
        render_tx,
        agent_handle,
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    spawn_session_persistence(&bus);

    // Keep the agent actor alive until shutdown.
    let _agent_actor = agent_actor;
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
) {
    tokio::spawn(input_reader(input_tx, kb_rx));
    tokio::spawn(render_task(terminal, render_rx));
}

fn spawn_ui_actor(
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: runie_agent::AgentActorHandle,
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
        UiActor::new(state, render_tx, agent_handle, kb_tx, bus, shutdown_tx, caps).run(ui_sub),
    );
}

async fn render_task(
    mut terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut render_rx: watch::Receiver<Snapshot>,
) {
    let mut last_size: Option<(u16, u16)> = None;
    loop {
        let snap = render_rx.borrow_and_update().clone();
        let new_size = terminal
            .size()
            .map(|r| (r.width, r.height))
            .unwrap_or((0, 0));
        if last_size != Some(new_size) {
            let _ = terminal.clear();
            last_size = Some(new_size);
        }
        let _ = terminal.draw(|f| ui::draw_snapshot(f, &snap));
        if render_rx.changed().await.is_err() {
            break;
        }
    }
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


