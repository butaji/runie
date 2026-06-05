//! Runie Terminal - Binary Entry Point
use std::sync::Arc;

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_core::{AppState, Event as AppEvent};
use runie_tui::ui;
use std::io;
use std::time::Duration;

const STATE_FILE: &str = "/tmp/runie_state.bin";

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Panic handler to restore terminal
    std::panic::set_hook(Box::new(|_| {
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();
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

    // Run the event loop
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let final_state = run_loop(&mut terminal, state, &running);

    // Cleanup and save
    disable_raw_mode().ok();
    execute!(io::stdout(), LeaveAlternateScreen).ok();
    
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
                    if matches!(evt, AppEvent::Quit) {
                        break;
                    }
                    // Update: process event and get new state
                    state = runie_core::update::update(state, evt);
                }
            }
        }
    }
    
    Ok(state)
}

fn convert_event(event: &CrosstermEvent) -> Option<AppEvent> {
    match event {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            // Check Ctrl modifiers first
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C') => Some(AppEvent::Quit),
                    KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppEvent::Quit),
                    KeyCode::Char('d') | KeyCode::Char('D') => Some(AppEvent::Quit),
                    _ => None,
                };
            }
            // Non-Ctrl keys
            match key.code {
                KeyCode::Esc => Some(AppEvent::Quit),
                KeyCode::Char(c) => Some(AppEvent::Input(c)),
                KeyCode::Backspace => Some(AppEvent::Backspace),
                KeyCode::Enter => Some(AppEvent::Submit),
                KeyCode::Up => Some(AppEvent::ScrollUp),
                KeyCode::Down => Some(AppEvent::ScrollDown),
                KeyCode::Home => Some(AppEvent::Reset),
                KeyCode::End => Some(AppEvent::Reset),
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
