//! Runie Terminal — Single-threaded async event loop with batched events
//!
//! Architecture:
//!   1. tokio::main runs everything on one thread (input + render + agent)
//!   2. Events are batched: process up to BATCH_SIZE per frame, then draw once
//!   3. Cache rebuild (ensure_fresh) happens in render path, not per-event
//!   4. Dirty flag: only redraw when state actually changes

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{AgentCommand, run_agent_turn};
use runie_core::{AppState, Event as CoreEvent};
use std::{io, time::Duration};
use tokio::sync::mpsc;




const ANIM_MS: u64 = 16;
const BATCH_SIZE: usize = 10;

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
    let mut terminal = setup_terminal()?;
    let state = AppState::default();

    let (input_tx, input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

    tokio::spawn(agent_loop(cmd_rx, agent_tx));
    tokio::spawn(input_reader(input_tx));

    event_loop(&mut terminal, state, input_rx, agent_rx, cmd_tx).await
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout()))
}

async fn input_reader(input_tx: mpsc::Sender<CoreEvent>) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        if let Some(evt) = convert_event(&event) {
            let is_quit = matches!(&evt, CoreEvent::Quit | CoreEvent::Reset);
            if input_tx.send(evt).await.is_err() { break; }
            if is_quit { break; }
        }
    }
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut state: AppState,
    mut input_rx: mpsc::Receiver<CoreEvent>,
    mut agent_rx: mpsc::Receiver<CoreEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
) -> io::Result<()> {
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));
    let mut dirty = true;

    loop {
        let mut events = 0usize;
        loop {
            tokio::select! {
                biased;

                Some(evt) = input_rx.recv(), if events < BATCH_SIZE => {
                    let was_submit = matches!(evt, CoreEvent::Submit);
                    state.update(evt.clone());
                    dirty = true; events += 1;
                    if matches!(evt, CoreEvent::Quit) { return Ok(()); }
                    if was_submit || matches!(evt, CoreEvent::FollowUp) {
                        spawn_if_queued(&mut state, &cmd_tx).await;
                    }
                }

                Some(evt) = agent_rx.recv(), if events < BATCH_SIZE => {
                    let was_done = matches!(evt, CoreEvent::AgentDone { .. } | CoreEvent::AgentError { .. });
                    state.update(evt);
                    dirty = true; events += 1;
                    if was_done {
                        spawn_if_queued(&mut state, &cmd_tx).await;
                    }
                }

                _ = anim.tick(), if events < BATCH_SIZE => {
                    if state.turn_active {
                        state.tick_animation();
                        dirty = true;
                    }
                    break;
                }

                else => break,
            }
        }

        if dirty {
            terminal.draw(|f| runie_tui::ui::view(f, &mut state))?;
            dirty = false;
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
                let core_evt = evt.to_core_event();
                let tx = agent_tx_clone.clone();
                tokio::spawn(async move { let _ = tx.send(core_evt).await; });
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
    #[test]
    fn animation_interval_supports_60fps() {
        assert!(super::ANIM_MS <= 17, "ANIM_MS must be <= 17ms for 60fps, got {}", super::ANIM_MS);
    }
}

fn convert_event(event: &Event) -> Option<CoreEvent> {
    if let Event::Key(key) = event {
        if std::env::var("RUNIE_DEBUG").is_ok() {
            use std::io::Write;
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/runie_keys.log")
                .and_then(|mut f| writeln!(f, "{:?}", key));
        }
    }
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C')
                    | KeyCode::Char('q') | KeyCode::Char('Q')
                    | KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::Quit),
                    KeyCode::Char('s') | KeyCode::Char('S') => Some(CoreEvent::Abort),
                    _ => None,
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    KeyCode::Enter => Some(CoreEvent::FollowUp),
                    _ => None,
                }
            } else {
                match key.code {
                    KeyCode::Esc => Some(CoreEvent::Abort),
                    KeyCode::Char('\t') | KeyCode::Tab | KeyCode::BackTab => Some(CoreEvent::Input('\t')),
                    KeyCode::Char(c) => Some(CoreEvent::Input(c)),
                    KeyCode::Backspace => Some(CoreEvent::Backspace),
                    KeyCode::Enter => Some(CoreEvent::Submit),
                    KeyCode::Up => Some(CoreEvent::ScrollUp),
                    KeyCode::Down => Some(CoreEvent::ScrollDown),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}
