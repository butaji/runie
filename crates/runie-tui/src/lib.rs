//! Runie TUI library - all logic goes here.
mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{engine::AgentLoop, provider::MockProvider, types::Message};
use serde::{Deserialize, Serialize};
use bincode;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const STATE_FILE: &str = "/tmp/runie_state.bin";

#[derive(Serialize, Deserialize, Default)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic handler to restore terminal
    std::panic::set_hook(Box::new(|_| {
        cleanup_terminal();
    }));

    // Load state if exists
    let saved_state = load_state();
    
    // Enable raw mode
    if let Err(e) = enable_raw_mode() {
        eprintln!("No terminal: {:?}", e);
        return Ok(());
    }

    let mut stdout = io::stdout();
    execute!(&mut stdout, EnterAlternateScreen, EnableMouseCapture).ok();

    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Use saved state or default
    let state = if let Some(s) = saved_state {
        s
    } else {
        AppState::default()
    };

    let result = run_app(&mut terminal, state, running);

    cleanup_terminal();

    match result {
        Ok(state) => {
            save_state(&state);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn cleanup_terminal() {
    disable_raw_mode().ok();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();
}

fn load_state() -> Option<AppState> {
    let data = std::fs::read(STATE_FILE).ok()?;
    bincode::deserialize(&data).ok().map(|s| {
        let _ = std::fs::remove_file(STATE_FILE);
        s
    })
}

fn save_state(state: &AppState) {
    if let Ok(data) = bincode::serialize(state) {
        let _ = std::fs::write(STATE_FILE, &data);
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: AppState,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider);
    let agent = AgentLoop::new(provider);
    let (agent_tx, mut agent_rx) = mpsc::channel(128);

    let mut needs_redraw = true;

    while running.load(std::sync::atomic::Ordering::SeqCst) 
        && (!state.streaming || !state.input.is_empty() || !state.messages.is_empty()) 
    {
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, &state))?;
            needs_redraw = false;
        }

        match event::poll(Duration::from_millis(50)) {
            Ok(true) => {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        match handle_key(&key) {
                            KeyAction::Quit => break,
                            KeyAction::Input(c) => {
                                state.input.push(c);
                                needs_redraw = true;
                            }
                            KeyAction::Backspace => {
                                state.input.pop();
                                needs_redraw = true;
                            }
                            KeyAction::Submit => {
                                let msg = ChatMessage {
                                    role: "user".into(),
                                    content: state.input.clone(),
                                };
                                state.messages.push(msg);
                                state.input.clear();
                                state.streaming = true;
                                needs_redraw = true;
                                
                                let messages = build_messages(&state);
                                let mut stream = agent.run(messages);
                                let tx = agent_tx.clone();
                                tokio::spawn(async move {
                                    while let Some(evt) = stream.next().await {
                                        let _ = tx.send(evt).await;
                                    }
                                });
                            }
                            KeyAction::None => {}
                        }
                    }
                }
            }
            Ok(false) | Err(_) => {}
        }

        // Process agent events
        while let Ok(evt) = agent_rx.try_recv() {
            match evt {
                runie_agent::types::AgentEvent::MessageDelta { content } => {
                    if let Some(last) = state.messages.last_mut() {
                        if last.role == "assistant" {
                            last.content.push_str(&content);
                        }
                    }
                    needs_redraw = true;
                }
                runie_agent::types::AgentEvent::MessageStart { .. } => {
                    state.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: String::new(),
                    });
                    needs_redraw = true;
                }
                runie_agent::types::AgentEvent::MessageEnd => {
                    state.streaming = false;
                    needs_redraw = true;
                }
                _ => {}
            }
        }
    }

    Ok(state)
}

fn build_messages(state: &AppState) -> Vec<Message> {
    let mut msgs = vec![Message::System {
        content: "You are a helpful assistant.".into(),
    }];
    for m in &state.messages {
        let msg = match m.role.as_str() {
            "user" => Message::User { content: m.content.clone() },
            "assistant" => Message::Assistant { content: m.content.clone(), tool_calls: vec![] },
            _ => continue,
        };
        msgs.push(msg);
    }
    msgs
}

enum KeyAction {
    Quit,
    Input(char),
    Backspace,
    Submit,
    None,
}

fn handle_key(key: &crossterm::event::KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Quit,
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Char(c) => KeyAction::Input(c),
        KeyCode::Backspace => KeyAction::Backspace,
        KeyCode::Enter => KeyAction::Submit,
        _ => KeyAction::None,
    }
}
