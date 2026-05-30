//! Clipboard utilities: copy last response, base64 encoding.

use crate::components::MessageItem;
use crate::tui::state::AppState;

/// Copy the last assistant message to clipboard using OSC 52 escape sequence.
pub fn handle_copy_last_response(state: &mut AppState) -> Vec<crate::tui::update::ui::UiCmd> {
    // Find last assistant message
    if let Some(last_assistant) = state.messages.iter().rev().find(|m| {
        matches!(m, MessageItem::Assistant { .. })
    }) {
        let text = match last_assistant {
            MessageItem::Assistant { text, .. } => text.clone(),
            _ => String::new(),
        };

        if !text.is_empty() {
            // Use OSC 52 escape sequence for clipboard (works in most modern terminals)
            let encoded_bytes = base64_encode(&text);
            let encoded = std::str::from_utf8(&encoded_bytes).unwrap_or_default();
            let osc52 = format!("\x1b]52;c;{}\x07", encoded);
            print!("{}", osc52);
            state.messages.push(MessageItem::System {
                text: "Copied last response to clipboard".to_string(),
            });
        } else {
            state.messages.push(MessageItem::System {
                text: "No assistant response to copy".to_string(),
            });
        }
    } else {
        state.messages.push(MessageItem::System {
            text: "No assistant response to copy".to_string(),
        });
    }
    vec![]
}

/// Base64 encode a string (minimal implementation avoiding external dependency).
fn base64_encode(input: &str) -> Vec<u8> {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::with_capacity((input.len() + 2) / 3 * 4);
    
    for chunk in input.as_bytes().chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };
        
        result.push(BASE64_CHARS[(b[0] >> 2) as usize]);
        result.push(BASE64_CHARS[((b[0] & 0x03) << 4 | b[1] >> 4) as usize]);
        
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize]);
        } else {
            result.push(b'=');
        }
        
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(b[2] & 0x3f) as usize]);
        } else {
            result.push(b'=');
        }
    }
    
    result
}
