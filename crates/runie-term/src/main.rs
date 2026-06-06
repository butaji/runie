use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::{io, time::Duration};
use tokio::sync::{mpsc, watch};

// ═══════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════
const ANIM_MS: u64 = 200; // Spinner frame change interval

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

// ═══════════════════════════════════════════════════════════════════
// MAIN — Spawns actors, each owns its state
// ═══════════════════════════════════════════════════════════════════
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let _cleanup = Cleanup;
    let terminal = setup_terminal()?;

    // Actor mailboxes
    let (ui_tx, ui_rx) = mpsc::channel::<CoreEvent>(100);
    let (render_tx, render_rx) = watch::channel(AppState::default());
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

    // Spawn actors — each owns its state, no shared references
    tokio::spawn(input_actor(ui_tx.clone()));
    tokio::spawn(agent_actor(cmd_rx, ui_tx));
    tokio::spawn(render_actor(render_rx, terminal));

    // UI actor owns AppState — runs until Quit
    ui_actor(ui_rx, render_tx, cmd_tx).await;
    Ok(())
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout())).map_err(|e| e.into())
}

// ═══════════════════════════════════════════════════════════════════
// INPUT ACTOR — Reads crossterm events, forwards as CoreEvent
// CONTRACT: Never blocks — drops events if UI channel full
// ═══════════════════════════════════════════════════════════════════
async fn input_actor(ui_tx: mpsc::Sender<CoreEvent>) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        if let Some(evt) = convert_event(&event) {
            let should_break = matches!(evt, CoreEvent::Quit | CoreEvent::Reset);
            if ui_tx.try_send(evt).is_err() { break; } // Channel full or closed
            if should_break { break; }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// UI ACTOR — Owns AppState. Single source of truth.
// Receives all events, updates state, sends snapshots to render actor.
// CONTRACT: Never blocks outbound — uses try_send + watch::send
// ═══════════════════════════════════════════════════════════════════
async fn ui_actor(
    mut rx: mpsc::Receiver<CoreEvent>,
    render_tx: watch::Sender<AppState>,
    cmd_tx: mpsc::Sender<AgentCommand>,
) {
    let mut state = AppState::default();
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

    loop {
        tokio::select! {
            Some(evt) = rx.recv() => {
                if matches!(evt, CoreEvent::Quit) { break; }
                process_event(&mut state, evt, &cmd_tx);
                maybe_send_snapshot(&mut state, &render_tx);
            }
            _ = anim.tick() => {
                if state.turn_active {
                    state.tick_animation();
                    maybe_send_snapshot(&mut state, &render_tx);
                }
            }
        }
    }
}

#[inline]
fn process_event(state: &mut AppState, evt: CoreEvent, cmd_tx: &mpsc::Sender<AgentCommand>) {
    let is_submit = matches!(evt, CoreEvent::Submit);
    state.update(evt);
    if is_submit {
        if let Some((content, id)) = state.peek_queue() {
            let content = content.clone();
            let id = id.clone();
            state.pop_queue();
            state.streaming = true;
            state.inflight += 1;
            let _ = cmd_tx.try_send(AgentCommand { content, id });
        }
    }
}

#[inline]
fn maybe_send_snapshot(state: &mut AppState, render_tx: &watch::Sender<AppState>) {
    if state.is_dirty() {
        state.ensure_fresh(); // O(n) here, not in render actor
        // watch::send is sync and overwrites — never blocks render actor.
        let _ = render_tx.send(state.clone());
    }
}

// ═══════════════════════════════════════════════════════════════════
// RENDER ACTOR — Owns Terminal. Draws ONLY when state changes.
// Uses watch::changed() to sleep until UI sends a new snapshot.
// Zero CPU waste on idle. Instant response to input.
// CONTRACT: Never mutates state — pure drawing
// ═══════════════════════════════════════════════════════════════════
async fn render_actor(
    mut render_rx: watch::Receiver<AppState>,
    mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
) {
    // Draw initial state immediately — watch::changed() does NOT fire for the
    // initial value, it waits for the NEXT send. Without this draw, the screen
    // stays blank until the first event.
    {
        let mut state = render_rx.borrow().clone();
        let _ = terminal.draw(|f| runie_tui::ui::view(f, &mut state));
    }

    loop {
        // Sleep until UI sends a new snapshot. No polling, no timer drift.
        if render_rx.changed().await.is_err() { break; }

        let mut state = render_rx.borrow().clone();
        let _ = terminal.draw(|f| runie_tui::ui::view(f, &mut state));
    }
}

// ═══════════════════════════════════════════════════════════════════
// AGENT ACTOR — Owns LLM / mock provider. Processes commands.
// CONTRACT: Never blocks UI — sends events via try_send
// ═══════════════════════════════════════════════════════════════════
async fn agent_actor(mut cmd_rx: mpsc::Receiver<AgentCommand>, ui_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            tool_flow(&cmd, &ui_tx).await;
        } else {
            simple_flow(&cmd, &ui_tx).await;
        }
        let _ = ui_tx.try_send(CoreEvent::AgentDone { id: cmd.id });
    }
}

async fn simple_flow(cmd: &AgentCommand, ui_tx: &mpsc::Sender<CoreEvent>) {
    thinking(&cmd.id, ui_tx).await;
    let msgs = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    for chunk in MockProvider.generate(msgs) {
        let _ = ui_tx.try_send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: chunk.content });
        sleep(50).await;
    }
}

async fn tool_flow(cmd: &AgentCommand, ui_tx: &mpsc::Sender<CoreEvent>) {
    let start = std::time::Instant::now();
    thinking(&cmd.id, ui_tx).await;
    tool_exec(&cmd.id, ui_tx).await;
    thinking(&cmd.id, ui_tx).await;
    let _ = ui_tx.try_send(CoreEvent::AgentTurnComplete {
        id: cmd.id.clone(),
        duration_secs: start.elapsed().as_secs_f64(),
    });
}

async fn thinking(id: &str, ui_tx: &mpsc::Sender<CoreEvent>) {
    let _ = ui_tx.try_send(CoreEvent::AgentThinking { id: id.to_string() });
    sleep(500).await;
    let _ = ui_tx.try_send(CoreEvent::AgentThoughtDone { id: id.to_string() });
}

async fn tool_exec(id: &str, ui_tx: &mpsc::Sender<CoreEvent>) {
    let _ = ui_tx.try_send(CoreEvent::AgentToolStart { id: id.to_string(), name: "list_files".to_string() });
    sleep(1000).await;
    let files = get_fake_file_list();
    let _ = ui_tx.try_send(CoreEvent::AgentToolEnd { duration_secs: 1.0 });
    let _ = ui_tx.try_send(CoreEvent::AgentResponse { id: id.to_string(), content: format!("\n{}", files) });
    sleep(50).await;
}

async fn sleep(ms: u64) {
    if !cfg!(test) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
}

// ═══════════════════════════════════════════════════════════════════
// EVENT CONVERSION — Crossterm → CoreEvent
// ═══════════════════════════════════════════════════════════════════
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
