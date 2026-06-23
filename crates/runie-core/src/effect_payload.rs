//! Pure domain helper for deriving TUI effect payloads from events and state.
//!
//! Keeping this logic in `runie-core` means the TUI layer only translates a
//! payload into a terminal-side effect, without knowing how to compute copy
//! text, last responses, or session data.

use crate::event::Event;
use crate::message::ChatMessage;
use crate::model::Role;
use crate::AppState;

/// Self-contained description of a user-initiated side effect.
#[derive(Debug, Clone)]
pub enum EffectPayload {
    /// Open the system editor with the given text.
    OpenExternalEditor { text: String },
    /// Copy the given text to the clipboard.
    CopyToClipboard { text: String },
    /// Share the given session messages.
    ShareSession {
        messages: Vec<ChatMessage>,
        display_name: Option<String>,
    },
    /// Validate a provider API key.
    LoginValidateKey { provider: String, key: String },
    /// Suspend the terminal process.
    Suspend,
}

/// Extract an effect payload from an event and the current state.
pub fn extract(event: &Event, state: &AppState) -> Option<EffectPayload> {
    match event {
        Event::OpenExternalEditor => Some(EffectPayload::OpenExternalEditor {
            text: state.input.input.clone(),
        }),
        Event::CopyToClipboard(text) => Some(EffectPayload::CopyToClipboard {
            text: text.clone(),
        }),
        Event::CopyLastResponse => {
            let text = last_assistant_text(&state.session.messages);
            if text.is_empty() {
                return None;
            }
            Some(EffectPayload::CopyToClipboard { text })
        }
        Event::CopySelectedBlock => state
            .copy_selected_post_text()
            .map(|text| EffectPayload::CopyToClipboard { text }),
        Event::CopyBlockMetadata => state
            .copy_selected_post_metadata()
            .map(|text| EffectPayload::CopyToClipboard { text }),
        Event::ShareSession => Some(EffectPayload::ShareSession {
            messages: state.session.messages.clone(),
            display_name: state.session.session_display_name.clone(),
        }),
        Event::Suspend => Some(EffectPayload::Suspend),
        Event::SubmitKey { provider, key } => Some(EffectPayload::LoginValidateKey {
            provider: provider.clone(),
            key: key.clone(),
        }),
        _ => None,
    }
}

fn last_assistant_text(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant)
        .map(|m| m.content().clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ChatMessage;

    #[test]
    fn copy_last_response_extracts_assistant_text() {
        let mut state = AppState::default();
        state.session.messages.push(ChatMessage::system("sys".to_string()));
        state
            .session
            .messages
            .push(ChatMessage::assistant("the answer".to_string()));
        let payload = extract(&Event::CopyLastResponse, &state);
        assert!(
            matches!(payload, Some(EffectPayload::CopyToClipboard { text }) if text == "the answer")
        );
    }

    #[test]
    fn copy_last_response_empty_when_no_assistant() {
        let state = AppState::default();
        assert!(extract(&Event::CopyLastResponse, &state).is_none());
    }
}
