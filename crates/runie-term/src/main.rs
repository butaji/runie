//! Runie Terminal — Single-threaded event loop (ratatui best practice)
//!
//! Architecture:
//!   - Main thread: event loop + terminal.draw() (synchronous)
//!   - Agent: tokio task on worker thread
//!   - Input: async EventStream
//!
//! Key perf choices:
//!   - No UI thread (no state cloning)
//!   - No RwLock (no contention)
//!   - Batch process events, draw between batches
//!   - Only render visible viewport (not all messages)
//!   - Skip draw when nothing changed

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::{
    io,
    time::Duration,
};
use tokio::sync::mpsc;

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

    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::default();

    // Async channels (bounded to prevent unbounded growth)
    let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

    // Agent on worker thread
    tokio::spawn(agent_loop(cmd_rx, agent_tx));

    // Input reader — async EventStream, non-blocking
    let input_tx_clone = input_tx.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if let Some(evt) = convert_event(&event) {
                let is_quit = matches!(evt, CoreEvent::Quit);
                if input_tx_clone.send(evt).await.is_err() { break; }
                if is_quit { break; }
            }
        }
    });

    let mut anim_interval = tokio::time::interval(Duration::from_millis(200));
    let mut dirty = true; // force first draw

    loop {
        // ── Phase 1: collect events (with timeout) ──
        let mut events_processed = 0usize;
        const BATCH_SIZE: usize = 10;

        // Process up to BATCH_SIZE events before drawing
        // This prevents blocking on a flood of agent chunks
        loop {
            tokio::select! {
                biased;

                Some(evt) = input_rx.recv(), if events_processed < BATCH_SIZE => {
                    state = runie_core::update::update(state, evt.clone());
                    dirty = true;
                    events_processed += 1;
                    if matches!(evt, CoreEvent::Quit) {
                        return Ok(());
                    }
                    if matches!(evt, CoreEvent::Submit) {
                        if let Some((content, id)) = state.peek_queue() {
                            state.pop_queue();
                            state.streaming = true;
                            let _ = cmd_tx.send(AgentCommand { content, id }).await;
                        }
                    }
                }

                Some(evt) = agent_rx.recv(), if events_processed < BATCH_SIZE => {
                    state = runie_core::update::update(state, evt);
                    dirty = true;
                    events_processed += 1;
                }

                _ = anim_interval.tick(), if events_processed < BATCH_SIZE => {
                    if state.turn_active {
                        state.animation_frame = state.animation_frame.wrapping_add(1);
                        dirty = true;
                    }
                    break; // animation tick = natural break point
                }

                else => break, // no more events, go to draw
            }
        }

        // ── Phase 2: draw if anything changed ──
        if dirty {
            terminal.draw(|f| runie_tui::ui::view(f, &state))?;
            dirty = false;
        }

        // ── Phase 3: throttle when truly idle ──
        // Only sleep if we processed nothing AND nothing is pending
        if events_processed == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            run_tool_flow(&cmd, &agent_tx).await;
        } else {
            run_simple_flow(&cmd, &agent_tx).await;
        }
        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

async fn run_simple_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let provider = MockProvider;
    let messages = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    for chunk in provider.generate(messages) {
        let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: chunk.content }).await;
        if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }
    }
}

async fn run_tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    use std::time::Instant;
    let turn_start = Instant::now();

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentToolStart { id: cmd.id.clone(), name: "list_files".to_string() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(1000)).await; }
    let _ = get_fake_file_list();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd { duration_secs: 1.0 }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: "Files:\n".to_string() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }

    let _ = agent_tx.send(CoreEvent::AgentTurnComplete { id: cmd.id.clone(), duration_secs: turn_start.elapsed().as_secs_f64() }).await;
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
