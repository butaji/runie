//! Runie Terminal - Binary Entry Point
//! 
//! Main event loop that:
//! 1. Receives UI events (keyboard)
//! 2. Receives agent events (from channel)
//! 3. Updates state
//! 4. Renders view

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};

use runie_agent::{Message, MockProvider, run_agent};
use runie_core::{AppState, ChatMessage, Event};

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

    // Setup signal handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    // Create channel for agent events
    let (tx, rx) = mpsc::channel::<Event>();

    // Setup terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Helper to spawn agent with specific content
    let spawn_agent = |content: String, tx: mpsc::Sender<Event>| {
        let agent_tx = tx.clone();
        thread::spawn(move || {
            let provider = MockProvider;
            // Create single user message
            let messages = vec![Message::User { content }];
            run_agent(&provider, messages, |e| {
                let _ = agent_tx.send(e);
            });
        });
    };

    // Main event loop
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // === 1. Process UI events ===
        if let Ok(true) = event::poll(Duration::from_millis(10)) {
            if let Ok(crossterm_event) = event::read() {
                if let Some(evt) = convert_ui_event(&crossterm_event) {
                    let was_streaming = state.streaming;
                    state = runie_core::update::update(state, evt);
                    
                    // If submit added to queue and we weren't streaming, spawn agent
                    if !was_streaming && !state.request_queue.is_empty() {
                        if let Some(request) = state.pop_queue() {
                            spawn_agent(request, tx.clone());
                        }
                    }
                }
            }
        }

        // === 3. Process agent events ===
        let was_streaming = state.streaming;
        while let Ok(evt) = rx.try_recv() {
            state = runie_core::update::update(state, evt);
        }
        
        // === 4. Spawn next agent if one finished ===
        if was_streaming && !state.streaming && !state.request_queue.is_empty() {
            if let Some(request) = state.pop_queue() {
                spawn_agent(request, tx.clone());
            }
        }

        // === 4. Render ===
        terminal.draw(|f| runie_tui::ui::view(f, &state))?;
        
        // === 5. Check for quit ===
        if matches!(state.messages.last(), Some(ChatMessage { role, .. }) if role == "quit") {
            break;
        }
    }

    // Cleanup
    disable_raw_mode().ok();
    execute!(std::io::stdout(), LeaveAlternateScreen).ok();
    save_state(&state);
    
    Ok(())
}

fn convert_ui_event(event: &CrosstermEvent) -> Option<Event> {
    match event {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            // Check Ctrl modifiers first
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C')
                    | KeyCode::Char('q') | KeyCode::Char('Q')
                    | KeyCode::Char('d') | KeyCode::Char('D') => Some(Event::Quit),
                    _ => None,
                };
            }
            // Non-Ctrl keys
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
