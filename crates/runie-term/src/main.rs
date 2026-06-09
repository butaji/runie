//! Runie Terminal — Non-blocking event loop with render actor
//!
//! Architecture (impossible to block by design):
//!   1. Event loop: single-threaded async, only async ops
//!   2. State: owned by event loop, mutable borrow per event
//!   3. Snapshot: immutable frame description (the UI DSL)
//!   4. Render actor: owns Terminal, receives Snapshots via channel
//!   5. If render is slow, old Snapshots are dropped — event loop never waits

mod keymap;

use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{AgentCommand, run_agent_turn};
use runie_core::{AppState, Event as CoreEvent, Snapshot, keybindings, config_reload};
use std::{collections::HashMap, io, io::Write, time::Duration};
use tokio::sync::mpsc;

const ANIM_MS: u64 = 200;

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let _cleanup = Cleanup;
    let terminal = setup_terminal()?;
    let mut state = AppState::default();
    apply_trust_on_startup(&mut state);
    init_scoped_models(&mut state);

    let (input_tx, input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = mpsc::channel::<Snapshot>(1);

    let keybindings = keybindings::load_keybindings(&None);

    tokio::spawn(agent_loop(cmd_rx, agent_tx));
    tokio::spawn(input_reader(input_tx.clone(), keybindings));
    tokio::spawn(render_task(terminal, render_rx));
    tokio::spawn(config_reload::spawn_config_watcher(input_tx.clone(), config_reload::config_path()));

    event_loop(state, input_rx, agent_rx, cmd_tx, render_tx, input_tx).await
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableBracketedPaste,
    )?;
    Terminal::new(CrosstermBackend::new(std::io::stdout()))
}

async fn render_task(
    mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    mut render_rx: mpsc::Receiver<Snapshot>,
) {
    while let Some(snap) = render_rx.recv().await {
        let _ = terminal.draw(|f| runie_tui::ui::draw_snapshot(f, &snap));
    }
}

async fn input_reader(input_tx: mpsc::Sender<CoreEvent>, bindings: HashMap<String, String>) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        if let Some(evt) = keymap::convert_event(&event, &bindings) {
            let is_quit = matches!(&evt, CoreEvent::Quit | CoreEvent::Reset);
            if input_tx.send(evt).await.is_err() { break; }
            if is_quit { break; }
        }
    }
}

fn init_scoped_models(state: &mut AppState) {
    let config = config_reload::Config::load_from(&config_reload::config_path());
    if let Some(scoped) = config.scoped_models() {
        state.scoped_models = scoped
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() == 2 {
                    runie_core::model::ScopedModel {
                        provider: parts[0].to_string(),
                        name: parts[1].to_string(),
                        enabled: true,
                    }
                } else {
                    runie_core::model::ScopedModel {
                        provider: state.current_provider.clone(),
                        name: s.clone(),
                        enabled: true,
                    }
                }
            })
            .collect();
    } else {
        // Default: first 10 models from catalog
        let registry = runie_provider::model::ModelRegistry::default();
        state.scoped_models = registry
            .list()
            .iter()
            .take(10)
            .map(|m| runie_core::model::ScopedModel {
                provider: m.provider.clone(),
                name: m.name.clone(),
                enabled: true,
            })
            .collect();
    }
}

fn apply_trust_on_startup(state: &mut AppState) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let tm = runie_core::TrustManager::load();
    match tm.decision_for(&cwd) {
        Some(runie_core::TrustDecision::Untrusted) => {
            state.read_only = true;
        }
        Some(runie_core::TrustDecision::Trusted) => {
            state.read_only = false;
        }
        None => {
            state.read_only = false;
            state.messages.push(runie_core::ChatMessage {
                role: runie_core::Role::System,
                content: format!(
                    "Welcome to runie in {}.\n\nThis project is not yet trusted. \
                    Run /trust to enable write tools, or /untrust to enforce read-only mode.",
                    cwd.display()
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                id: "trust_welcome".to_string(),
                ..Default::default()
            });
            state.messages_changed();
        }
    }
}

async fn event_loop(
    mut state: AppState,
    mut input_rx: mpsc::Receiver<CoreEvent>,
    mut agent_rx: mpsc::Receiver<CoreEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: mpsc::Sender<Snapshot>,
    input_tx: mpsc::Sender<CoreEvent>,
) -> io::Result<()> {
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

    // Initial draw so the user sees the app immediately, without waiting
    // for the first keyboard event.
    state.ensure_fresh();
    let _ = render_tx.try_send(state.snapshot());

    loop {
        tokio::select! {
            biased;

            Some(evt) = input_rx.recv() => {
                if matches!(evt, CoreEvent::OpenExternalEditor) {
                    let text = state.input.clone();
                    let tx = input_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = spawn_external_editor_sync(text, tx);
                    });
                } else {
                    let was_submit = matches!(evt, CoreEvent::Submit);
                    let was_followup = matches!(evt, CoreEvent::FollowUp);
                    state.update(evt);
                    if state.should_quit {
                        return Ok(());
                    }
                    if was_submit || was_followup {
                        spawn_if_queued(&mut state, &cmd_tx).await;
                    }
                }
            }

            Some(evt) = agent_rx.recv() => {
                let was_done = matches!(evt, CoreEvent::AgentDone { .. } | CoreEvent::AgentError { .. });
                state.update(evt);
                if was_done {
                    spawn_if_queued(&mut state, &cmd_tx).await;
                }
            }

            _ = anim.tick() => {
                state.tick_animation();
            }
        }

        state.ensure_fresh();
        let snap = state.snapshot();
        if render_tx.try_send(snap).is_err() {
            // Render task is behind — old snapshot dropped, latest will draw
        }
    }
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        let agent_tx_clone = agent_tx.clone();
        let cmd_id = cmd.id.clone();

        let result = run_agent_turn(
            &cmd,
            |evt| {
                let tx = agent_tx_clone.clone();
                let _ = tx.try_send(evt);
            },
            5,
        ).await;

        if let Err(e) = result {
            let _ = agent_tx.send(CoreEvent::AgentError {
                id: cmd_id,
                message: format!("Agent error: {}", e),
            }).await;
        }
    }
}

fn spawn_external_editor_sync(
    text: String,
    tx: mpsc::Sender<CoreEvent>,
) -> io::Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) { "notepad" } else { "vi" }.to_string()
    });

    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(text.as_bytes())?;
    tmp.flush()?;
    let path = tmp.into_temp_path();

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()?;

    if status.success() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let _ = handle.block_on(tx.send(CoreEvent::ExternalEditorDone { content }));
        }
    }

    Ok(())
}

async fn spawn_if_queued(state: &mut AppState, cmd_tx: &mpsc::Sender<AgentCommand>) {
    if let Some((content, id)) = state.peek_queue() {
        let content = content.clone();
        let id = id.clone();
        state.pop_queue();
        state.streaming = true;
        state.turn_active = true;
        state.inflight += 1;
        let _ = cmd_tx.send(AgentCommand {
            content,
            id,
            provider: state.current_provider.clone(),
            model: state.current_model.clone(),
            thinking_level: state.thinking_level,
            read_only: state.read_only,
        }).await;
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn animation_interval_is_200ms() {
        assert_eq!(super::ANIM_MS, 200, "ANIM_MS must be 200ms for visible braille spinner, got {}", super::ANIM_MS);
    }

    #[tokio::test]
    async fn spawn_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();
        state.request_queue.push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.turn_active);
        assert_eq!(state.inflight, 0);

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(state.turn_active, "spawn_if_queued must set turn_active");
        assert_eq!(state.inflight, 1, "spawn_if_queued must increment inflight");
        assert!(state.request_queue.is_empty(), "Message should be popped from request_queue");

        let cmd = rx.try_recv().expect("Command should be sent to agent");
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.thinking_level, runie_core::model::ThinkingLevel::Off);
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(!state.turn_active);
        assert_eq!(state.inflight, 0);
    }
}
