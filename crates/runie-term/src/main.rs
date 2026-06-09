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
use std::{collections::HashMap, io, time::Duration};
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
    let state = AppState::default();

    let (input_tx, input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = mpsc::channel::<Snapshot>(1);

    let keybindings = keybindings::load_keybindings(&None);

    tokio::spawn(agent_loop(cmd_rx, agent_tx));
    tokio::spawn(input_reader(input_tx.clone(), keybindings));
    tokio::spawn(render_task(terminal, render_rx));
    tokio::spawn(config_reload::spawn_config_watcher(input_tx, config_reload::config_path()));

    event_loop(state, input_rx, agent_rx, cmd_tx, render_tx).await
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

async fn event_loop(
    mut state: AppState,
    mut input_rx: mpsc::Receiver<CoreEvent>,
    mut agent_rx: mpsc::Receiver<CoreEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: mpsc::Sender<Snapshot>,
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
        }).await;
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

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
