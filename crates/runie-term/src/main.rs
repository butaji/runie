//! Runie Terminal - Binary Entry Point
//! 
//! UI on main thread, agent in background thread with channels.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const STATE_FILE: &str = "/tmp/runie_state.bin";

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Panic handler
    std::panic::set_hook(Box::new(|_| {
        disable_raw_mode().ok();
        execute!(std::io::stdout(), LeaveAlternateScreen).ok();
    }));

    // Load saved state
    let mut state = load_state().unwrap_or_default();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(&mut stdout, EnterAlternateScreen).ok();

    // Create channels
    let (app_tx, app_rx) = mpsc::channel::<Event>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>();

    // Spawn agent thread
    let provider = MockProvider;
    thread::spawn(move || {
        run_agent_thread(cmd_rx, app_tx, provider);
    });

    // Setup terminal backend
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Flag for running
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    // Setup signal handler
    let _ = thread::spawn(move || {
        ctrlc::set_handler(move || {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        })
        .ok();
    });

    // Main UI loop (runs on main thread)
    loop {
        // === 1. Process UI events (blocking with timeout) ===
        if let Ok(true) = event::poll(Duration::from_millis(50)) {
            if let Ok(crossterm_event) = event::read() {
                if let Some(evt) = convert_ui_event(&crossterm_event) {
                    state = runie_core::update::update(state, evt.clone());

                    if matches!(evt, Event::Submit) {
                        if let Some((content, id)) = state.peek_queue() {
                            state.pop_queue();
                            state.streaming = true;
                            // Send command to agent thread
                            let _ = cmd_tx.send(AgentCommand { content, id });
                        }
                    }
                }
            }
        }

        // === 2. Process agent events (non-blocking) ===
        while let Ok(evt) = app_rx.try_recv() {
            state = runie_core::update::update(state, evt);
        }

        // === 3. Render ===
        terminal.draw(|f| runie_tui::ui::view(f, &state))?;

        // === 4. Check for quit ===
        if matches!(state.messages.last(), Some(runie_core::ChatMessage { role, .. }) if role == "quit") {
            break;
        }
        
        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
    }

    // Cleanup
    disable_raw_mode().ok();
    execute!(std::io::stdout(), LeaveAlternateScreen).ok();
    save_state(&state);

    Ok(())
}

/// Agent thread: processes commands sequentially
fn run_agent_thread(cmd_rx: mpsc::Receiver<AgentCommand>, app_tx: mpsc::Sender<Event>, provider: MockProvider) {
    loop {
        // Receive command (blocking)
        let cmd = match cmd_rx.recv() {
            Ok(cmd) => cmd,
            Err(_) => break, // Channel closed
        };

        // Run agent synchronously
        run_agent_sync(&provider, cmd, &app_tx);
    }
}

/// Run agent synchronously and send events
fn run_agent_sync(provider: &MockProvider, cmd: AgentCommand, app_tx: &mpsc::Sender<Event>) {
    // Send thinking FIRST (so UI starts timing)
    let _ = app_tx.send(Event::AgentThinking { id: cmd.id.clone() });

    // THEN delay for manual UI testing (this is the "thinking" time), skip in tests
    if std::env::var("RUNIE_TEST").is_err() {
        let delay_ms = 500 + (rand_u32() % 2500);
        thread::sleep(Duration::from_millis(delay_ms as u64));
    }

    // Get response chunks
    let messages = vec![runie_agent::Message::User { content: cmd.content }];
    let chunks = provider.generate(messages);

    // Send each chunk
    for chunk in chunks {
        let _ = app_tx.send(Event::AgentResponse {
            id: cmd.id.clone(),
            content: chunk.content,
        });
        
        // Small delay for streaming effect (skip in tests)
        if std::env::var("RUNIE_TEST").is_err() {
            thread::sleep(Duration::from_millis(50));
        }
    }

    // Send done
    let _ = app_tx.send(Event::AgentDone { id: cmd.id });
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32
}

fn convert_ui_event(event: &CrosstermEvent) -> Option<Event> {
    match event {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C')
                    | KeyCode::Char('q') | KeyCode::Char('Q')
                    | KeyCode::Char('d') | KeyCode::Char('D') => Some(Event::Quit),
                    _ => None,
                };
            }
            match key.code {
                KeyCode::Esc => Some(Event::Quit),
                KeyCode::Char(c) => Some(Event::Input(c)),
                KeyCode::Backspace => Some(Event::Backspace),
                KeyCode::Enter => Some(Event::Submit),
                KeyCode::Up => Some(Event::ScrollUp),
                KeyCode::Down => Some(Event::ScrollDown),
                KeyCode::Home | KeyCode::End => Some(Event::Reset),
                _ => None,
            }
        }
        _ => None,
    }
}

fn load_state() -> Option<AppState> {
    let data = std::fs::read(STATE_FILE).ok()?;
    let state = bincode::deserialize(&data).ok()?;
    let _ = std::fs::remove_file(STATE_FILE);
    Some(state)
}

fn save_state(state: &AppState) {
    if let Ok(data) = bincode::serialize(state) {
        let _ = std::fs::write(STATE_FILE, &data);
    }
}
