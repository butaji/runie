//! Runie TUI - MVU Architecture
//! 
//! Model: AppState - immutable application state
//! View: view() - renders state to terminal
//! Update: update() - processes events and returns new state

mod ui;
mod model;
mod update;

pub use model::{AppState, ChatMessage, Msg};
pub use update::Event;

// Re-export for convenience
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use std::time::Duration;

const STATE_FILE: &str = "/tmp/runie_state.bin";

/// Main entry point
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Panic handler to restore terminal
    std::panic::set_hook(Box::new(|_| {
        cleanup_terminal();
    }));

    // Load saved state or use default
    let state = load_state().unwrap_or_default();
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(&mut stdout, EnterAlternateScreen).ok();

    // Setup signal handler
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    // Run the MVU loop
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let final_state = run_loop(&mut terminal, state, &running);

    // Cleanup and save
    cleanup_terminal();
    if let Ok(state) = final_state {
        save_state(&state);
    }
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: AppState,
    running: &Arc<std::sync::atomic::AtomicBool>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let mut state = state;
    
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // View: render current state
        terminal.draw(|f| ui::view(f, &state))?;
        
        // Process events
        if let Ok(true) = event::poll(Duration::from_millis(50)) {
            if let Ok(event) = event::read() {
                // Convert crossterm event to our event
                if let Some(evt) = convert_event(&event) {
                    // Quit events break the loop directly
                    if matches!(evt, Event::Quit) {
                        break;
                    }
                    // Update: process event and get new state
                    state = update::update(state, evt);
                }
            }
        }
    }
    
    Ok(state)
}

fn convert_event(event: &CrosstermEvent) -> Option<Event> {
    match event {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            // IMPORTANT: Check Ctrl modifiers BEFORE checking Char
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C') => Some(Event::Quit),
                    KeyCode::Char('q') | KeyCode::Char('Q') => Some(Event::Quit),
                    KeyCode::Char('d') | KeyCode::Char('D') => Some(Event::Quit),
                    KeyCode::Char('r') | KeyCode::Char('R') => Some(Event::Reset),
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
                KeyCode::Home => Some(Event::Reset),
                KeyCode::End => Some(Event::Reset),
                _ => None,
            }
        }
        _ => None,
    }
}

fn cleanup_terminal() {
    disable_raw_mode().ok();
    execute!(io::stdout(), LeaveAlternateScreen).ok();
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
