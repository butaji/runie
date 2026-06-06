use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::{io, time::Duration};
use tokio::sync::mpsc;

// Constants
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
    let mut state = AppState::default();

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
    Terminal::new(CrosstermBackend::new(std::io::stdout())).map_err(|e| e.into())
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
                    state.update(evt.clone());
                    if matches!(evt, CoreEvent::Quit) { return Ok(()); }
                    if matches!(evt, CoreEvent::Submit) {
                        if let Some(content) = state.peek_queue() {
                            let id = state.next_id();
                            state.pop_queue();
                            state.streaming = true;
                            let _ = cmd_tx.send(AgentCommand { content, id }).await;
                        }
                    }
                    dirty = true; events += 1;
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
        if dirty { terminal.draw(|f| runie_tui::ui::view(f, &state)).expect("draw"); dirty = false; }
        if events == 0 { tokio::time::sleep(Duration::from_millis(THROTTLE_MS)).await; }
    }
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            tool_flow(&cmd, &agent_tx).await;
        } else {
            simple_flow(&cmd, &agent_tx).await;
        }
        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

async fn simple_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    thinking(&cmd.id, agent_tx).await;
    let msgs = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    for chunk in MockProvider.generate(msgs) {
        let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: chunk.content }).await;
        sleep(50).await;
    }
}

async fn tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    let start = std::time::Instant::now();
    thinking(&cmd.id, agent_tx).await;
    tool_exec(&cmd.id, agent_tx).await;
    thinking(&cmd.id, agent_tx).await;
    let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: "Files:\n".to_string() }).await;
    sleep(50).await;
    let _ = agent_tx.send(CoreEvent::AgentTurnComplete { id: cmd.id.clone(), duration_secs: start.elapsed().as_secs_f64() }).await;
}

async fn thinking(id: &str, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: id.to_string() }).await;
    sleep(500).await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: id.to_string() }).await;
}

async fn tool_exec(id: &str, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentToolStart { id: id.to_string(), name: "list_files".to_string() }).await;
    sleep(1000).await;
    let _ = get_fake_file_list();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd { duration_secs: 1.0 }).await;
    sleep(50).await;
}

async fn sleep(ms: u64) {
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(ms)).await; }
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
