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

// Timing constants
const ANIM_MS: u64 = 200;
const THROTTLE_MS: u64 = 50;
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
            if input_tx.send(evt.clone()).await.is_err() { break; }
            if matches!(evt, CoreEvent::Quit | CoreEvent::Reset) { break; }
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
                    if was_submit {
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
                }

                Some(evt) = agent_rx.recv(), if events < BATCH_SIZE => {
                    state.update(evt);
                    dirty = true; events += 1;
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

        if events == 0 {
            tokio::time::sleep(Duration::from_millis(THROTTLE_MS)).await;
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
                let _ = agent_tx_clone.try_send(core_evt);
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

fn convert_event(event: &Event) -> Option<CoreEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C')
                    | KeyCode::Char('q') | KeyCode::Char('Q')
                    | KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::Quit),
                    _ => None,
                }
            } else {
                match key.code {
                    KeyCode::Esc => Some(CoreEvent::Quit),
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
