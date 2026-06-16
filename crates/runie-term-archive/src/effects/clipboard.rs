//! Clipboard effect handlers.

use crate::terminal::{caps::TerminalCapabilities, clipboard as term_clipboard};
use runie_core::model::Role;
use runie_core::ChatMessage;

/// Copy the given text to the terminal clipboard if the terminal supports it.
pub fn copy_to_clipboard(text: String, caps: TerminalCapabilities) {
    if caps.clipboard {
        let _ = term_clipboard::copy_to_clipboard(&mut std::io::stdout(), &text);
    }
}

/// Copy the last assistant response to the terminal clipboard.
pub fn copy_last_response(messages: Vec<ChatMessage>, caps: TerminalCapabilities) {
    if !caps.clipboard {
        return;
    }
    let text = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant)
        .map(|m| m.content.clone())
        .unwrap_or_default();
    if !text.is_empty() {
        let _ = term_clipboard::copy_to_clipboard(&mut std::io::stdout(), &text);
    }
}
