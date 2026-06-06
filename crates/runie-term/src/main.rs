//! Runie Terminal - Async Binary Entry Point

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::io;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

const FAST_MODE: bool = cfg!(test);

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let _cleanup = Cleanup;

    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;

    let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

    let provider = MockProvider;
    tokio::spawn(agent_loop(cmd_rx, agent_tx, provider));

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

    let mut state = load_state().unwrap_or_default();
    // Initialize cache
    state.formatted_cache = runie_core::format_messages(&state);
    let mut anim_interval = interval(Duration::from_millis(50));
    let mut draw_interval = interval(Duration::from_millis(16)); // 60fps max
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut pending_draw = false;
    
    // Initial render
    terminal.draw(|f| runie_tui::ui::view(f, &state))?;
    
    loop {
        tokio::select! {
            Some(evt) = input_rx.recv() => {
                state = runie_core::update::update(state, evt.clone());

                if matches!(evt, CoreEvent::Submit) {
                    if let Some((content, id)) = state.peek_queue() {
                        state.pop_queue();
                        state.streaming = true;
                        let _ = cmd_tx.send(AgentCommand { content, id }).await;
                    }
                }

                if matches!(evt, CoreEvent::Quit) {
                    break;
                }
                
                pending_draw = true;
            }
            
            Some(evt) = agent_rx.recv() => {
                state = runie_core::update::update(state, evt);
                pending_draw = true;
            }
            
            _ = draw_interval.tick() => {
                if pending_draw || state.turn_active {
                    terminal.draw(|f| runie_tui::ui::view(f, &state))?;
                    pending_draw = false;
                }
            }
            
            _ = anim_interval.tick() => {
                // Animation only if turn is active
                if state.turn_active {
                    state.animation_frame = state.animation_frame.wrapping_add(1);
                    pending_draw = true; // Will be drawn on next draw_interval tick
                }
            }
        }
        
        if matches!(state.messages.last(), Some(runie_core::ChatMessage { role, .. }) if role == "quit") {
            break;
        }
    }

    save_state(&state);
    Ok(())
}

/// Agent loop: processes commands sequentially
async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>, provider: MockProvider) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            run_tool_flow(&cmd, &agent_tx).await;
        } else {
            run_simple_flow(&cmd, &provider, &agent_tx).await;
        }

        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

/// Simple response flow: Thinking -> Though -> Response
async fn run_simple_flow(cmd: &AgentCommand, provider: &MockProvider, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;

    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

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

/// Tool execution flow: Though -> Ran -> Though -> Response -> Turn complete
async fn run_tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    use std::time::Instant;

    let turn_start = Instant::now();
    let tool_start = Instant::now();

    // 1. First thinking
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    // 2. Tool execution
    let _ = agent_tx.send(CoreEvent::AgentToolStart {
        id: cmd.id.clone(),
        name: "list_files".to_string(),
    }).await;
    // Simulate tool execution time
    if !FAST_MODE {
        tokio::time::sleep(Duration::from_millis(500 + (rand_u32() % 1500) as u64)).await;
    }
    let _ = get_fake_file_list(); // Trigger tool
    let tool_duration = tool_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd {
        duration_secs: tool_duration,
    }).await;
    chunk_delay().await;

    // 3. Second thinking
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    // 4. Response
    let _ = agent_tx.send(CoreEvent::AgentResponse {
        id: cmd.id.clone(),
        content: "Here are the files in your project:\n".to_string(),
    }).await;
    chunk_delay().await;

    // 5. Turn complete
    let duration = turn_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentTurnComplete {
        id: cmd.id.clone(),
        duration_secs: duration,
    }).await;
}

async fn thinking_delay() {
    if !FAST_MODE {
        let delay_ms = 500 + (rand_u32() % 2500);
        tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
    }
}

async fn chunk_delay() {
    if !FAST_MODE {
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

fn save_state(state: &AppState) {
    if let Ok(data) = bincode::serialize(state) {
        let _ = std::fs::write("/tmp/runie_state.bin", &data);
    }
}
