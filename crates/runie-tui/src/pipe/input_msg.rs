//! InputMsg for terminal input events.
//!
//! Used by InputActor to send input events through the system.

use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum InputMsg {
    Key(KeyEvent),
    Paste(String),
    Resize(u16, u16),
}