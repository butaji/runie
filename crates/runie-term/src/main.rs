//! Runie Terminal - Async Binary Entry Point
//! 
//! Uses tokio runtime with EventStream for clean async event handling.

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::io;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

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

    // Channels
    let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(32);
    let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(32);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

    // Spawn agent actor
    let provider = MockProvider;
    tokio::spawn(agent_loop(cmd_rx, agent_tx, provider));

    // Spawn input reader
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

    // Main state
    let mut state = load_state().unwrap_or_default();
    let mut render_interval = interval(Duration::from_millis(50));
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        tokio::select! {
            // Render at ~20fps
            _ = render_interval.tick() => {
                terminal.draw(|f| runie_tui::ui::view(f, &state))?;
            }
            
            // Process input events
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
            }
            
            // Process agent events
            Some(evt) = agent_rx.recv() => {
                state = runie_core::update::update(state, evt);
            }
        }

        // Check for quit message
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
        // Send thinking
        let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;

        // Delay for manual UI testing (this is the "thinking" time)
        let delay_ms = 500 + (rand_u32() % 2500);
        tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;

        // Get response chunks
        let messages = vec![runie_agent::Message::User { content: cmd.content }];
        let chunks = provider.generate(messages);

        // Send each chunk
        for chunk in chunks {
            let _ = agent_tx.send(CoreEvent::AgentResponse {
                id: cmd.id.clone(),
                content: chunk.content,
            }).await;

            // Small delay between chunks
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Done
        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
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
