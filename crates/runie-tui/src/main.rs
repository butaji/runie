use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use libloading::{Library, Symbol};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use runie_agent::{
    engine::AgentLoop,
    provider::MockProvider,
    types::Message,
};
use std::{
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;

mod ui;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to set up terminal, but don't fail hard
    let _ = enable_raw_mode();
    let _ = execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture);

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create terminal: {}", e);
            return Ok(());
        }
    };

    let res = run_app(&mut terminal).await;

    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    let _ = io::stdout().flush();
    
    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn isatty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) != 0 }
}

type AppStatePtr = *mut runie_app::AppState;
type BytesPtr = *mut Vec<u8>;

struct HotApp {
    lib: Library,
    state: AppStatePtr,
}

impl HotApp {
    fn lib_path() -> String {
        if cfg!(target_os = "macos") {
            "target/debug/librunie_app.dylib".to_string()
        } else {
            "target/debug/librunie_app.so".to_string()
        }
    }

    fn new(build_time: String) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = unsafe { Library::new(Self::lib_path()) }?;
        
        let init: Symbol<unsafe fn() -> AppStatePtr> = unsafe { lib.get(b"app_init")? };
        let state = unsafe { init() };
        
        // Set build time
        let state_ref = unsafe { &mut *state };
        state_ref.build_time = build_time;
        
        Ok(Self { lib, state })
    }
    
    fn save_state(&self) {
        let serialize: Symbol<unsafe fn(AppStatePtr) -> BytesPtr> = 
            unsafe { self.lib.get(b"app_serialize") }.unwrap();
        
        let bytes = unsafe { serialize(self.state) };
        if !bytes.is_null() {
            let data = unsafe { Box::from_raw(bytes) };
            let _ = std::fs::write("/tmp/runie_state.json", &*data);
            // Leak the box intentionally to transfer ownership to C/other side
            std::mem::forget(data);
        }
    }
    
    fn restore_state(&mut self) {
        if let Ok(data) = std::fs::read("/tmp/runie_state.json") {
            let deserialize: Symbol<unsafe fn(BytesPtr) -> AppStatePtr> = 
                unsafe { self.lib.get(b"app_deserialize") }.unwrap();
            
            let data_box = Box::new(data);
            let data_ptr = Box::into_raw(data_box);
            let new_state = unsafe { deserialize(data_ptr) };
            
            // Free old state, use new one
            let free_state: Symbol<unsafe fn(AppStatePtr)> = 
                unsafe { self.lib.get(b"app_free_state") }.unwrap();
            unsafe { free_state(self.state) };
            
            self.state = new_state;
        }
    }
    
    fn reload(&mut self, build_time: String) -> Result<(), Box<dyn std::error::Error>> {
        println!(">>> Reloading app...");
        self.save_state();
        
        // Load new lib (old one will be dropped when reassigned)
        self.lib = unsafe { Library::new(Self::lib_path())? };
        self.restore_state();
        
        // Update build time
        let state_ref = unsafe { &mut *self.state };
        state_ref.build_time = build_time;
        
        println!(">>> Reloaded!");
        Ok(())
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let build_time = chrono::Local::now().format("%H%M").to_string();
    let mut hot_app = HotApp::new(build_time.clone())?;
    let provider = Arc::new(MockProvider);
    let agent = AgentLoop::new(provider);

    let (agent_tx, mut agent_rx) = mpsc::channel::<runie_agent::types::AgentEvent>(128);

    let mut needs_redraw = true;
    let mut last_reload_check = std::time::Instant::now();

    loop {
        if needs_redraw {
            terminal.draw(|f| {
                let state = unsafe { &*hot_app.state };
                ui::draw(f, state);
            })?;
            needs_redraw = false;
        }

        // Check for reload every 500ms
        if last_reload_check.elapsed() > Duration::from_millis(500) {
            last_reload_check = std::time::Instant::now();
            
            let current_mtime = std::fs::metadata("target/debug/librunie_app.so")
                .and_then(|m| m.modified())
                .ok();
            
            static LAST_MTIME: std::sync::OnceLock<std::time::SystemTime> = std::sync::OnceLock::new();
            
            if let Some(current) = current_mtime {
                if let Some(last) = LAST_MTIME.get() {
                    if current > *last {
                        let new_build_time = chrono::Local::now().format("%H%M").to_string();
                        if let Err(e) = hot_app.reload(new_build_time) {
                            eprintln!("Reload failed: {}", e);
                        }
                        needs_redraw = true;
                    }
                } else {
                    LAST_MTIME.set(current).ok();
                }
            }
        }

        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) 
                            || !key.modifiers.is_empty() => {
                            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                                hot_app.save_state();
                                break;
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            hot_app.save_state();
                            break;
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            hot_app.save_state();
                            break;
                        }
                        KeyCode::Esc => {
                            hot_app.save_state();
                            break;
                        }
                        KeyCode::Char(c) => {
                            let state = unsafe { &*hot_app.state };
                            if !state.streaming {
                                let push: Symbol<unsafe fn(AppStatePtr, libc::c_char)> = 
                                    unsafe { hot_app.lib.get(b"app_input_push") }.unwrap();
                                unsafe { push(hot_app.state, c as libc::c_char) };
                                needs_redraw = true;
                            }
                        }
                        KeyCode::Backspace => {
                            let state = unsafe { &*hot_app.state };
                            if !state.streaming {
                                let backspace: Symbol<unsafe fn(AppStatePtr)> = 
                                    unsafe { hot_app.lib.get(b"app_input_backspace") }.unwrap();
                                unsafe { backspace(hot_app.state) };
                                needs_redraw = true;
                            }
                        }
                        KeyCode::Enter => {
                            let state = unsafe { &*hot_app.state };
                            if !state.streaming && !state.input.is_empty() {
                                let push: Symbol<unsafe fn(AppStatePtr)> = 
                                    unsafe { hot_app.lib.get(b"app_push_message") }.unwrap();
                                unsafe { push(hot_app.state) };
                                
                                let set_streaming: Symbol<unsafe fn(AppStatePtr, bool)> = 
                                    unsafe { hot_app.lib.get(b"app_set_streaming") }.unwrap();
                                unsafe { set_streaming(hot_app.state, true) };
                                
                                // Build messages from state
                                let messages = build_messages(unsafe { &*hot_app.state });
                                let mut stream = agent.run(messages);
                                let tx = agent_tx.clone();

                                tokio::spawn(async move {
                                    while let Some(event) = stream.next().await {
                                        let _ = tx.send(event).await;
                                    }
                                });
                                
                                needs_redraw = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        while let Ok(event) = agent_rx.try_recv() {
            let event_str = match event {
                runie_agent::types::AgentEvent::MessageStart { .. } => "START".to_string(),
                runie_agent::types::AgentEvent::MessageDelta { content } => format!("TEXT:{}", content),
                runie_agent::types::AgentEvent::MessageEnd => "END".to_string(),
                _ => continue,
            };
            
            let handle: Symbol<unsafe fn(AppStatePtr, *const libc::c_char)> = 
                unsafe { hot_app.lib.get(b"app_handle_event") }.unwrap();
            let c_str = std::ffi::CString::new(event_str).unwrap();
            unsafe { handle(hot_app.state, c_str.as_ptr()) };
            
            needs_redraw = true;
        }

        // Keep redrawing during streaming
        let state = unsafe { &*hot_app.state };
        if state.streaming {
            needs_redraw = true;
        }
    }

    // Safety: free the state
    let free_state: Symbol<unsafe fn(AppStatePtr)> = 
        unsafe { hot_app.lib.get(b"app_free_state") }.unwrap();
    unsafe { free_state(hot_app.state) };

    Ok(())
}

fn build_messages(state: &runie_app::AppState) -> Vec<Message> {
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
