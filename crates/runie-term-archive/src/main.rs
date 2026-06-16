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

mod app_init;
mod effects;
mod keymap;
mod share;
mod terminal;
mod terminal_setup;

use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{build_provider_with_warning, run_agent_turn, AgentCommand};
use runie_core::actor::Actor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::event::{AgentEvent, ControlEvent, InputEvent, LoginFlowEvent, ModelConfigEvent};
use runie_core::session_store::SessionStore;
use runie_core::{config_reload, AppState, Snapshot};
use std::{collections::HashMap, io, sync::Arc, sync::Mutex, time::Duration};
use tokio::sync::{mpsc, watch};

const ANIM_MS: u64 = 200;

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
    let _cleanup = Cleanup;
    let (terminal, terminal_caps) = terminal_setup::setup_terminal()?;
    let mut state = AppState::default();
    init_terminal_state(&mut state);
    run_init_hooks(&mut state);

    // Create EventBus for cross-component communication (SessionActor subscription)
    let bus: EventBus<Event> = EventBus::new(100);

    // Spawn SessionActor to persist durable events to JSONL
    let session_id = format!("session_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let store = SessionStore::new(
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("runie")
            .join("sessions"),
    );
    let session_actor = runie_core::SessionActor::new(session_id, "main".into(), store);
    let (_session_tx, session_rx) = mpsc::channel(1);
    tokio::spawn(session_actor.run(session_rx, bus.clone()));

    let channels = spawn_background_tasks(terminal, &mut state, &terminal_caps, bus.clone());

    event_loop(
        state,
        channels.input_rx,
        channels.agent_rx,
        channels.cmd_tx,
        channels.render_tx,
        channels.input_tx,
        channels.kb_tx,
        terminal_caps,
        bus,
    )
    .await
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
        state.update(Event::LoginFlow(LoginFlowEvent::Start));
    }
}

struct BackgroundChannels {
    input_tx: mpsc::Sender<Event>,
    input_rx: mpsc::Receiver<Event>,
    agent_rx: mpsc::Receiver<Event>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: watch::Sender<Snapshot>,
    kb_tx: watch::Sender<HashMap<String, String>>,
}

fn spawn_background_tasks(
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    _caps: &crate::terminal::caps::TerminalCapabilities,
    bus: EventBus<Event>,
) -> BackgroundChannels {
    let (input_tx, input_rx) = mpsc::channel::<Event>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<Event>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = watch::channel(state.snapshot());
    let (kb_tx, kb_rx) = watch::channel(state.config.keybindings.clone());

    // Spawn actors that publish to EventBus
    tokio::spawn(agent_loop(cmd_rx, agent_tx, bus.clone()));
    tokio::spawn(input_reader(input_tx.clone(), kb_rx, bus));
    tokio::spawn(render_task(terminal, render_rx));
    tokio::spawn(config_reload::spawn_config_watcher(
        input_tx.clone(),
        config_reload::config_path(),
    ));

    BackgroundChannels {
        input_tx,
        input_rx,
        agent_rx,
        cmd_tx,
        render_tx,
        kb_tx,
    }
}

async fn render_task(
    mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
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
        let _ = terminal.draw(|f| runie_tui::ui::draw_snapshot(f, &snap));
        if render_rx.changed().await.is_err() {
            break;
        }
    }
}

/// Input reader that also publishes events to EventBus for SessionActor.
async fn input_reader(
    input_tx: mpsc::Sender<Event>,
    mut kb_rx: watch::Receiver<HashMap<String, String>>,
    bus: EventBus<Event>,
) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        let bindings = kb_rx.borrow_and_update().clone();
        if let Some(evt) = keymap::convert_event(&event, &bindings) {
            let is_quit = matches!(
                &evt,
                Event::Control(ControlEvent::Quit) | Event::Control(ControlEvent::Reset)
            );

            // Send to main loop
            if input_tx.send(evt.clone()).await.is_err() {
                break;
            }

            // Publish to EventBus for SessionActor
            bus.publish(evt);

            if is_quit {
                break;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn event_loop(
    mut state: AppState,
    mut input_rx: mpsc::Receiver<Event>,
    mut agent_rx: mpsc::Receiver<Event>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: watch::Sender<Snapshot>,
    input_tx: mpsc::Sender<Event>,
    kb_tx: watch::Sender<HashMap<String, String>>,
    terminal_caps: crate::terminal::caps::TerminalCapabilities,
    _bus: EventBus<Event>,
) -> io::Result<()> {
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

    state.ensure_fresh();
    let _ = render_tx.send(state.snapshot());

    loop {
        tokio::select! {
            biased;

            Some(evt) = input_rx.recv() => {
                if handle_input_with_effects(evt, &mut state, &input_tx, &render_tx, &kb_tx, &cmd_tx, &terminal_caps).await? {
                    return Ok(());
                }
            }

            Some(evt) = agent_rx.recv() => {
                handle_agent_event(evt, &mut state, &cmd_tx).await;
            }

            _ = anim.tick() => {
                state.tick_animation();
            }
        }

        state.ensure_fresh();
        let _ = render_tx.send(state.snapshot());
    }
}

async fn handle_input_with_effects(
    evt: Event,
    state: &mut AppState,
    input_tx: &mpsc::Sender<Event>,
    render_tx: &watch::Sender<Snapshot>,
    kb_tx: &watch::Sender<HashMap<String, String>>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    terminal_caps: &crate::terminal::caps::TerminalCapabilities,
) -> io::Result<bool> {
    if let Some(cmd) = effects::EffectCommand::try_from_event(&evt, state, terminal_caps) {
        state.update(evt);
        cmd.dispatch(input_tx.clone(), render_tx.clone(), state, *terminal_caps);
        return Ok(false);
    }
    handle_input_event(evt, state, kb_tx, cmd_tx).await
}

async fn handle_input_event(
    evt: Event,
    state: &mut AppState,
    kb_tx: &watch::Sender<HashMap<String, String>>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) -> io::Result<bool> {
    let was_submit = matches!(evt, Event::Input(InputEvent::Submit));
    let was_followup = matches!(evt, Event::Control(ControlEvent::FollowUp));
    let was_reload = matches!(evt, Event::ModelConfig(ModelConfigEvent::ReloadAll));
    state.update(evt);
    if state.should_quit {
        return Ok(true);
    }
    if was_reload {
        let _ = kb_tx.send(state.config.keybindings.clone());
    }
    if was_submit || was_followup {
        spawn_if_queued(state, cmd_tx).await;
    }
    Ok(false)
}

async fn handle_agent_event(
    evt: Event,
    state: &mut AppState,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) {
    let was_done = matches!(
        evt,
        Event::Agent(AgentEvent::Done { .. }) | Event::Agent(AgentEvent::Error { .. })
    );
    state.update(evt);
    if was_done {
        spawn_if_queued(state, cmd_tx).await;
    }
}

/// Agent loop that also publishes events to EventBus for SessionActor.
async fn agent_loop(
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    agent_tx: mpsc::Sender<Event>,
    bus: EventBus<Event>,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        let agent_tx_clone = agent_tx.clone();
        let bus_clone = bus.clone();
        let cmd_id = cmd.id.clone();

        let provider = match build_provider_with_warning(&cmd.provider, &cmd.model) {
            Ok(p) => p,
            Err(e) => {
                let evt = Event::Agent(AgentEvent::Error {
                    id: cmd_id,
                    message: format!("Provider error: {}", e),
                });
                let _ = agent_tx.send(evt.clone()).await;
                bus_clone.publish(evt);
                continue;
            }
        };

        let result = run_agent_turn(
            &provider,
            &cmd,
            Arc::new(Mutex::new(move |evt: Event| {
                let tx = agent_tx_clone.clone();
                let b = bus_clone.clone();
                let _ = tx.try_send(evt.clone());
                b.publish(evt);
            })),
            5,
        )
        .await;

        if let Err(e) = result {
            let evt = Event::Agent(AgentEvent::Error {
                id: cmd_id,
                message: format!("Agent error: {}", e),
            });
            let _ = agent_tx.send(evt.clone()).await;
            bus.publish(evt);
        }
    }
}

async fn spawn_if_queued(state: &mut AppState, cmd_tx: &mpsc::Sender<AgentCommand>) {
    if let Some((content, id)) = state.peek_queue() {
        let content = content.clone();
        let id = id.clone();
        state.pop_queue();
        state.agent.streaming = true;
        state.agent.turn_active = true;
        state.agent.inflight += 1;
        let skills_context = runie_core::skills::build_skills_context(&state.skills);
        let system_prompt = state
            .prompts
            .iter()
            .find(|p| p.name == state.input.current_prompt)
            .map(|p| p.content.clone())
            .unwrap_or_default();
        let _ = cmd_tx
            .send(AgentCommand {
                content,
                id,
                provider: state.config.current_provider.clone(),
                model: state.config.current_model.clone(),
                thinking_level: state.config.thinking_level,
                read_only: state.config.read_only,
                skills_context,
                system_prompt,
                truncation: runie_agent::truncate::policy_from_section(
                    state.config.truncation.max_lines,
                    state.config.truncation.max_bytes,
                ),
            })
            .await;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn animation_interval_is_200ms() {
        assert_eq!(
            super::ANIM_MS,
            200,
            "ANIM_MS must be 200ms for visible braille spinner, got {}",
            super::ANIM_MS
        );
    }

    #[tokio::test]
    async fn spawn_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(
            state.agent.turn_active,
            "spawn_if_queued must set turn_active"
        );
        assert_eq!(
            state.agent.inflight, 1,
            "spawn_if_queued must increment inflight"
        );
        assert!(
            state.agent.request_queue.is_empty(),
            "Message should be popped from request_queue"
        );

        let cmd = rx.try_recv().expect("Command should be sent to agent");
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.thinking_level, runie_core::model::ThinkingLevel::Off);
        assert_eq!(cmd.system_prompt, "");
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);
    }
}
