use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn default_bindings() -> std::collections::HashMap<String, String> {
    runie_core::keybindings::default_keybindings()
}

mod basic;
mod combos;
mod merge;
mod special;
