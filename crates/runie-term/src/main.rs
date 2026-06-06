//! Runie Terminal - Async Binary Entry Point
//! Architecture: Main = async runtime, UI = background thread

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::{
    io,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::sync::mpsc;

// Shared state between threads
type SharedState = Arc<Mutex<AppState>>;

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let _cleanup = Cleanup;

    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;

    // Shared state - single source of truth
    let state: SharedState = Arc::new(Mutex::new(load_state().unwrap_or_default()));
    let ui_state = state.clone();

    // UI thread - dedicated, runs terminal.draw() in background
    // NOTE: stdout is moved here, won't be available after
    let _ui_handle = thread::spawn(move || {
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
        
        // Initial render
        if let Ok(s) = ui_state.lock() {
            let _ = terminal.draw(|f| runie_tui::ui::view(f, &s));
        }

        // Render loop - 20fps
        loop {
            thread::sleep(Duration::from_millis(50));
            
            if let Ok(s) = ui_state.lock() {
                let _ = terminal.draw(|f| runie_tui::ui::view(f, &s));
            }
        }
    });

    // Main thread = async runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Channels (created in async context)
        let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(100);
        let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(100);
        let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

        // Agent loop - async
        let agent_state = state.clone();
        tokio::spawn(async move {
            agent_loop(cmd_rx, agent_tx, agent_state).await;
        });

        // Input reader - async
        let input_tx_clone = input_tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                if let Some(evt) = convert_event(&event) {
                    if input_tx_clone.send(evt).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Event loop
        let mut anim_interval = tokio::time::interval(Duration::from_millis(200));
        
        loop {
            tokio::select! {
                biased;
                
                Some(evt) = input_rx.recv() => {
                    let mut s = state.lock().unwrap();
                    *s = runie_core::update::update(std::mem::take(&mut *s), evt.clone());

                    if matches!(evt, CoreEvent::Submit) {
                        if let Some((content, id)) = s.peek_queue() {
                            s.pop_queue();
                            s.streaming = true;
                            let _ = cmd_tx.send(AgentCommand { content, id }).await;
                        }
                    }

                    if matches!(evt, CoreEvent::Quit) {
                        break;
                    }
                }
                
                Some(evt) = agent_rx.recv() => {
                    let mut s = state.lock().unwrap();
                    *s = runie_core::update::update(std::mem::take(&mut *s), evt);
                }
                
                _ = anim_interval.tick() => {
                    let mut s = state.lock().unwrap();
                    if s.turn_active {
                        s.animation_frame = s.animation_frame.wrapping_add(1);
                    }
                }
            }
            
            // Check quit
            let should_quit = {
                let s = state.lock().unwrap();
                matches!(s.messages.last(), Some(runie_core::ChatMessage { role, .. }) if role == "quit")
            };
            if should_quit {
                break;
            }
        }

        // Save state
        let final_state = state.lock().unwrap().clone();
        drop(final_state);
    });

    Ok(())
}

/// Agent loop
async fn agent_loop(
    mut cmd_rx: mpsc::Receiver<AgentCommand>, 
    agent_tx: mpsc::Sender<CoreEvent>,
    _state: SharedState,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            run_tool_flow(&cmd, &agent_tx).await;
        } else {
            run_simple_flow(&cmd, &agent_tx).await;
        }

        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

/// Simple response flow
async fn run_simple_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;

    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let provider = MockProvider;
    let messages = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    let chunks = provider.generate(messages);

    for chunk in chunks {
        let _ = agent_tx.send(CoreEvent::AgentResponse {
            id: cmd.id.clone(),
            content: chunk.content,
        }).await;
        chunk_delay().await;
    }
}

/// Tool execution flow
async fn run_tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    use std::time::Instant;

    let turn_start = Instant::now();
    let tool_start = Instant::now();

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentToolStart {
        id: cmd.id.clone(),
        name: "list_files".to_string(),
    }).await;
    if !cfg!(test) {
        tokio::time::sleep(Duration::from_millis(500 + (rand_u32() % 1500) as u64)).await;
    }
    let _ = get_fake_file_list();
    let tool_duration = tool_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd { duration_secs: tool_duration }).await;
    chunk_delay().await;

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentResponse {
        id: cmd.id.clone(),
        content: "Here are the files in your project:\n".to_string(),
    }).await;
    chunk_delay().await;

    let duration = turn_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentTurnComplete {
        id: cmd.id.clone(),
        duration_secs: duration,
    }).await;
}

async fn thinking_delay() {
    if !cfg!(test) {
        let delay_ms = 500 + (rand_u32() % 2500);
        tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
    }
}

async fn chunk_delay() {
    if !cfg!(test) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32
}

fn convert_event(event: &Event) -> Option<CoreEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
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
                    KeyCode::Home | KeyCode::End => Some(CoreEvent::Reset),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}

fn load_state() -> Option<AppState> {
    let data = std::fs::read("/tmp/runie_state.bin").ok()?;
    let state = bincode::deserialize(&data).ok()?;
    let _ = std::fs::remove_file("/tmp/runie_state.bin");
    Some(state)
}

fn save_state(_state: &AppState) {
    // Disabled for now - can be added back with proper handling
    // if let Ok(data) = bincode::serialize(state) {
    //     let _ = std::fs::write("/tmp/runie_state.bin", &data);
    // }
}
