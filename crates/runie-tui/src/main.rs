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
use runie_agent::{build_provider_with_warning, run_agent_turn, AgentCommand};
use runie_core::actor::{spawn_actor, Actor};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::event::{AgentEvent, LoginFlowEvent};
use runie_core::orchestrator_actor::{OrchestratorActor, OrchestratorEvent};
use runie_core::session_store::SessionStore;
use runie_core::{config_reload, AppState, Snapshot};
use runie_tui::{app_init, keymap, terminal, terminal_setup, theme, ui, ui_actor::UiActor};
use std::{collections::HashMap, io, sync::Arc, sync::Mutex};
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

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(report) = runie_tui::dry_run::run_from_args(&args) {
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;
    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;

    // Wire terminal capabilities into the theme system before first render.
    theme::set_current_theme_with_caps(theme::DEFAULT_THEME_NAME, terminal_caps);

    let mut state = AppState::default();
    init_terminal_state(&mut state);
    run_init_hooks(&mut state);

    // Create EventBus for cross-component communication (SessionActor subscription)
    let bus: EventBus<Event> = EventBus::new(100);

    // Spawn SessionActor to persist durable events to JSONL
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
    let (_session_tx, session_rx) = mpsc::channel(1);
    tokio::spawn(session_actor.run(session_rx, bus.clone()));

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    spawn_background_tasks(terminal, state, terminal_caps, bus.clone(), shutdown_tx);

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

fn run_init_hooks(state: &mut AppState) {
    app_init::apply_trust_on_startup(state);
    app_init::init_scoped_models(state);
    app_init::init_skills(state);
    app_init::init_prompts(state);
    app_init::init_telemetry(state);
    app_init::init_truncation(state);
    app_init::init_ui_config(state);

    if state.config.current_provider.is_empty() && !runie_core::provider_registry::is_mock_enabled()
    {
        state.update(LoginFlowEvent::Start);
    }
}

fn spawn_background_tasks(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut state: AppState,
    caps: terminal::caps::TerminalCapabilities,
    bus: EventBus<Event>,
    shutdown_tx: oneshot::Sender<()>,
) {
    let (input_tx, mut input_rx) = mpsc::channel::<Event>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = watch::channel(state.snapshot());
    let (kb_tx, kb_rx) = watch::channel(state.config.keybindings.clone());

    // Forward all input events (keyboard + config watcher) into the shared bus.
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        while let Some(evt) = input_rx.recv().await {
            bus_clone.publish(evt);
        }
    });

    // Spawn agents that publish to EventBus
    tokio::spawn(agent_loop(cmd_rx, bus.clone()));
    tokio::spawn(input_reader(input_tx.clone(), kb_rx));
    tokio::spawn(render_task(terminal, render_rx));
    tokio::spawn(config_reload::spawn_config_watcher(
        input_tx.clone(),
        config_reload::config_path(),
    ));

    // Spawn OrchestratorActor with its own EventBus, forwarding to main bus
    if state.config.execution_mode.uses_orchestrator() {
        let orch_bus: EventBus<OrchestratorEvent> = EventBus::new(100);
        let main_bus = bus.clone();
        tokio::spawn(forward_orchestrator_events(orch_bus.subscribe(), main_bus));
        let orchestrator = OrchestratorActor::new();
        let (_tx, handle) = spawn_actor(orchestrator, orch_bus);
        tokio::spawn(handle);
    }

    // UiActor is the sole owner of AppState and the only runtime mutator.
    let ui_sub = bus.subscribe();
    tokio::spawn(UiActor::new(state, render_tx, cmd_tx, kb_tx, bus, shutdown_tx, caps).run(ui_sub));
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

/// Forward `OrchestratorEvent` into the main `EventBus<Event>`.
async fn forward_orchestrator_events(
    mut orch_sub: runie_core::bus::ReplayReceiver<OrchestratorEvent>,
    main_bus: EventBus<Event>,
) {
    while let Ok(evt) = orch_sub.recv().await {
        let _ = main_bus.publish(evt);
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

/// Agent loop that publishes events to EventBus for SessionActor and UiActor.
async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, bus: EventBus<Event>) {
    while let Some(cmd) = cmd_rx.recv().await {
        let bus_clone = bus.clone();
        let cmd_id = cmd.id.clone();

        let provider = match build_provider_with_warning(&cmd.provider, &cmd.model) {
            Ok(p) => p,
            Err(e) => {
                let evt = AgentEvent::Error {
                    id: cmd_id,
                    message: format!("Provider error: {}", e),
                };
                bus_clone.publish(evt);
                continue;
            }
        };

        let result = run_agent_turn(
            &provider,
            &cmd,
            Arc::new(Mutex::new(move |evt: Event| {
                bus_clone.publish(evt);
            })),
            5,
        )
        .await;

        if let Err(e) = result {
            let evt = AgentEvent::Error {
                id: cmd_id,
                message: format!("Agent error: {}", e),
            };
            bus.publish(evt);
        }
    }
}
