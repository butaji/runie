use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use libloading::{Library, Symbol};
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{engine::AgentLoop, provider::MockProvider, types::Message};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;

mod ui;

type AppStatePtr = *mut runie_app::AppState;

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
        unsafe { &mut *state }.build_time = build_time;
        Ok(Self { lib, state })
    }

    fn save_state(&self) {
        let serialize: Symbol<unsafe fn(AppStatePtr) -> *mut Vec<u8>> =
            unsafe { self.lib.get(b"app_serialize") }.unwrap();
        let bytes = unsafe { serialize(self.state) };
        if !bytes.is_null() {
            let data = unsafe { Box::from_raw(bytes) };
            let _ = std::fs::write("/tmp/runie_state.json", &*data);
            std::mem::forget(data);
        }
    }

    fn restore_state(&mut self) {
        if let Ok(data) = std::fs::read("/tmp/runie_state.json") {
            let deserialize: Symbol<unsafe fn(*mut Vec<u8>) -> AppStatePtr> =
                unsafe { self.lib.get(b"app_deserialize") }.unwrap();
            let data_box = Box::new(data);
            let data_ptr = Box::into_raw(data_box);
            let new_state = unsafe { deserialize(data_ptr) };
            let free_state: Symbol<unsafe fn(AppStatePtr)> =
                unsafe { self.lib.get(b"app_free_state") }.unwrap();
            unsafe { free_state(self.state) };
            self.state = new_state;
        }
    }

    fn reload(&mut self, build_time: String) -> Result<(), Box<dyn std::error::Error>> {
        println!(">>> Reloading app...");
        self.save_state();
        self.lib = unsafe { Library::new(Self::lib_path())? };
        self.restore_state();
        unsafe { &mut *self.state }.build_time = build_time;
        println!(">>> Reloaded!");
        Ok(())
    }

    fn push_input(&self, c: char) {
        let push: Symbol<unsafe fn(AppStatePtr, libc::c_char)> =
            unsafe { self.lib.get(b"app_input_push") }.unwrap();
        unsafe { push(self.state, c as libc::c_char) };
    }

    fn backspace(&self) {
        let backspace: Symbol<unsafe fn(AppStatePtr)> =
            unsafe { self.lib.get(b"app_input_backspace") }.unwrap();
        unsafe { backspace(self.state) };
    }

    fn push_message(&self) {
        let push: Symbol<unsafe fn(AppStatePtr)> =
            unsafe { self.lib.get(b"app_push_message") }.unwrap();
        unsafe { push(self.state) };
    }

    fn set_streaming(&self, streaming: bool) {
        let set: Symbol<unsafe fn(AppStatePtr, bool)> =
            unsafe { self.lib.get(b"app_set_streaming") }.unwrap();
        unsafe { set(self.state, streaming) };
    }

    fn handle_event(&self, event: &str) {
        let handle: Symbol<unsafe fn(AppStatePtr, *const libc::c_char)> =
            unsafe { self.lib.get(b"app_handle_event") }.unwrap();
        let c_str = std::ffi::CString::new(event).unwrap();
        unsafe { handle(self.state, c_str.as_ptr()) };
    }

    fn free(&self) {
        let free_state: Symbol<unsafe fn(AppStatePtr)> =
            unsafe { self.lib.get(b"app_free_state") }.unwrap();
        unsafe { free_state(self.state) };
    }

    fn is_terminated(&self) -> bool {
        false // Controlled by external quit flag
    }
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

fn check_reload() -> Option<std::time::SystemTime> {
    static LAST_MTIME: std::sync::OnceLock<std::time::SystemTime> = std::sync::OnceLock::new();
    
    let current = std::fs::metadata("target/debug/librunie_app.so")
        .and_then(|m| m.modified())
        .ok()?;
    
    if let Some(last) = LAST_MTIME.get() {
        if current > *last {
            return Some(current);
        }
    } else {
        LAST_MTIME.set(current).ok();
    }
    None
}

fn handle_key_events(
    hot_app: &HotApp,
    agent: &AgentLoop,
    agent_tx: &mpsc::Sender<runie_agent::types::AgentEvent>,
) -> Option<bool> {
    let _state = unsafe { &*hot_app.state };
    
    match crossterm::event::poll(Duration::from_millis(50)) {
        Ok(true) => {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(hot_app, agent, agent_tx, &key)
                    } else {
                        Some(false)
                    }
                }
                Ok(_) => Some(false),
                Err(e) => {
                    eprintln!("Event read error: {:?}", e);
                    Some(false)
                }
            }
        }
        Ok(false) => Some(false),
        Err(e) => {
            // Don't spam errors in headless mode
            Some(false)
        }
    }
}

fn handle_key(
    hot_app: &HotApp,
    agent: &AgentLoop,
    agent_tx: &mpsc::Sender<runie_agent::types::AgentEvent>,
    key: &crossterm::event::KeyEvent,
) -> Option<bool> {
    let state = unsafe { &*hot_app.state };
    
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            hot_app.save_state();
            Some(true)
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            hot_app.save_state();
            Some(true)
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            hot_app.save_state();
            Some(true)
        }
        KeyCode::Esc => {
            hot_app.save_state();
            Some(true)
        }
        KeyCode::Char(c) if !state.streaming => {
            hot_app.push_input(c);
            Some(false)
        }
        KeyCode::Backspace if !state.streaming => {
            hot_app.backspace();
            Some(false)
        }
        KeyCode::Enter if !state.streaming && !state.input.is_empty() => {
            handle_submit(hot_app, agent, agent_tx);
            Some(false)
        }
        _ => Some(false),
    }
}

fn handle_submit(
    hot_app: &HotApp,
    agent: &AgentLoop,
    agent_tx: &mpsc::Sender<runie_agent::types::AgentEvent>,
) {
    hot_app.push_message();
    hot_app.set_streaming(true);
    
    let messages = build_messages(unsafe { &*hot_app.state });
    let mut stream = agent.run(messages);
    let tx = agent_tx.clone();

    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            let _ = tx.send(event).await;
        }
    });
}

fn process_agent_events(
    hot_app: &HotApp,
    agent_rx: &mut mpsc::Receiver<runie_agent::types::AgentEvent>,
) -> bool {
    let mut changed = false;
    
    while let Ok(event) = agent_rx.try_recv() {
        let event_str = match event {
            runie_agent::types::AgentEvent::MessageStart { .. } => "START".to_string(),
            runie_agent::types::AgentEvent::MessageDelta { content } => format!("TEXT:{}", content),
            runie_agent::types::AgentEvent::MessageEnd => "END".to_string(),
            _ => continue,
        };
        hot_app.handle_event(&event_str);
        changed = true;
    }
    changed
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }
    Ok(())
}

fn should_redraw(state: &runie_app::AppState, needs_redraw: bool) -> bool {
    needs_redraw || state.streaming
}

fn check_and_reload(hot_app: &mut HotApp, last_check: &mut std::time::Instant) -> bool {
    if last_check.elapsed() > Duration::from_millis(500) {
        *last_check = std::time::Instant::now();
        if check_reload().is_some() {
            let build_time = chrono::Local::now().format("%H%M").to_string();
            if let Err(e) = hot_app.reload(build_time) {
                eprintln!("Reload failed: {}", e);
            }
            return true;
        }
    }
    false
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut hot_app = HotApp::new(chrono::Local::now().format("%H%M").to_string())?;
    let provider = Arc::new(MockProvider);
    let agent = AgentLoop::new(provider);
    let (agent_tx, mut agent_rx) = mpsc::channel(128);

    let mut needs_redraw = true;
    let mut last_reload_check = std::time::Instant::now();

    while !hot_app.is_terminated() {
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, unsafe { &*hot_app.state }))?;
            needs_redraw = false;
        }

        if check_and_reload(&mut hot_app, &mut last_reload_check) {
            needs_redraw = true;
        }

        if let Some(true) = handle_key_events(&hot_app, &agent, &agent_tx) {
            break;
        }

        if process_agent_events(&hot_app, &mut agent_rx) {
            needs_redraw = true;
        }

        let state = unsafe { &*hot_app.state };
        needs_redraw = should_redraw(state, needs_redraw);
    }

    hot_app.free();
    Ok(())
}
