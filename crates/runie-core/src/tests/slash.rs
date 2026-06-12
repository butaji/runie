//! Slash command tests — ensure all /commands work as users expect

use crate::event::Event;
use crate::model::AppState;
use crate::session::Store;
use std::sync::Mutex;

pub static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn fresh_state() -> AppState {
    AppState::default()
}

pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

/// Set input buffer directly and submit — bypasses the command palette.
/// Use for slash commands that need arguments.
pub fn exec(state: &mut AppState, text: &str) {
    state.input.input = text.into();
    state.input.cursor_pos = text.len();
    state.update(Event::Submit);
}

pub fn tmp_store() -> Store {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("runie_slash_test_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    Store::new(dir)
}

pub fn minimal_session(name: &str) -> crate::session::Session {
    crate::session::Session {
        name: name.to_string(),
        created_at: 1.0,
        updated_at: 1.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "runie".into(),
        thinking_level: crate::model::ThinkingLevel::Off,
        read_only: false,
        display_name: None,
        session_tree: None,
    }
}

pub mod misc;
pub mod model;
pub mod save_load;
pub mod session;
