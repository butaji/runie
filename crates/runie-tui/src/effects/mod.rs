//! Typed effect commands dispatched from the main event loop.
//!
//! Effect payload computation is inlined here to keep effect logic localized
//! to the TUI layer.

use runie_core::{AppState, Event as CoreEvent, Snapshot};
use tokio::sync::{mpsc, watch};

use crate::terminal::caps::TerminalCapabilities;

mod clipboard;
mod editor;
pub(crate) mod login;
mod share;
mod suspend;

// ---------------------------------------------------------------------------
// Effect payload (formerly in runie-core::effect_payload)
// ---------------------------------------------------------------------------

/// Self-contained description of a user-initiated side effect.
#[derive(Debug, Clone)]
pub enum EffectPayload {
    /// Open the system editor with the given text.
    OpenExternalEditor { text: String },
    /// Copy the given text to the clipboard.
    CopyToClipboard { text: String },
    /// Share the given session messages.
    ShareSession {
        messages: Vec<runie_core::ChatMessage>,
        display_name: Option<String>,
    },
    /// Validate a provider API key.
    LoginValidateKey { provider: String, key: String },
    /// Suspend the terminal process.
    Suspend,
}

/// Extract an effect payload from an event and the current state.
fn extract(event: &CoreEvent, state: &mut AppState) -> Option<EffectPayload> {
    match event {
        CoreEvent::OpenExternalEditor => Some(EffectPayload::OpenExternalEditor {
            text: state.input().input.clone(),
        }),
        CoreEvent::CopyToClipboard(text) => {
            Some(EffectPayload::CopyToClipboard { text: text.clone() })
        }
        CoreEvent::CopyLastResponse => {
            let text = last_assistant_text(&state.session().messages);
            if text.is_empty() {
                return None;
            }
            Some(EffectPayload::CopyToClipboard { text })
        }
        CoreEvent::CopySelectedBlock => state
            .copy_selected_post_text()
            .map(|text| EffectPayload::CopyToClipboard { text }),
        CoreEvent::CopyBlockMetadata => state
            .copy_selected_post_metadata()
            .map(|text| EffectPayload::CopyToClipboard { text }),
        CoreEvent::ShareSession => Some(EffectPayload::ShareSession {
            messages: state.session().messages.clone(),
            display_name: state.session().session_display_name.clone(),
        }),
        CoreEvent::Suspend => Some(EffectPayload::Suspend),
        CoreEvent::SubmitKey { provider, key } => Some(EffectPayload::LoginValidateKey {
            provider: provider.clone(),
            key: key.clone(),
        }),
        _ => None,
    }
}

fn last_assistant_text(messages: &[runie_core::ChatMessage]) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == runie_core::Role::Assistant)
        .map(|m| m.content())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Effect command
// ---------------------------------------------------------------------------

pub enum EffectCommand {
    OpenExternalEditor {
        text: String,
    },
    CopyToClipboard {
        text: String,
    },
    ShareSession {
        messages: Vec<runie_core::ChatMessage>,
        display_name: Option<String>,
    },
    Suspend {
        terminal_caps: TerminalCapabilities,
    },
    LoginFlowSubmitKey {
        provider: String,
        key: String,
    },
}

impl EffectCommand {
    /// Build an effect command from a core event, if the event is an effect.
    pub fn try_from_event(
        evt: &CoreEvent,
        state: &mut AppState,
        caps: &TerminalCapabilities,
    ) -> Option<Self> {
        let payload = extract(evt, state)?;
        Some(match payload {
            EffectPayload::OpenExternalEditor { text } => Self::OpenExternalEditor { text },
            EffectPayload::CopyToClipboard { text } => Self::CopyToClipboard { text },
            EffectPayload::ShareSession {
                messages,
                display_name,
            } => Self::ShareSession {
                messages,
                display_name,
            },
            EffectPayload::LoginValidateKey { provider, key } => {
                Self::LoginFlowSubmitKey { provider, key }
            }
            EffectPayload::Suspend => Self::Suspend {
                terminal_caps: *caps,
            },
        })
    }

    /// Run the side effect asynchronously, feeding results back via `tx`.
    pub fn dispatch(
        self,
        tx: mpsc::Sender<CoreEvent>,
        render_tx: watch::Sender<Snapshot>,
        state: &mut AppState,
        caps: TerminalCapabilities,
    ) {
        match self {
            Self::OpenExternalEditor { text } => editor::run(text, tx),
            Self::CopyToClipboard { text } => clipboard::copy_to_clipboard(text, caps),
            Self::ShareSession {
                messages,
                display_name,
            } => share::run(messages, display_name, tx),
            Self::Suspend { terminal_caps } => suspend::run(terminal_caps, render_tx, state),
            Self::LoginFlowSubmitKey { provider, key } => {
                if let Some(ref handles) = state.actor_handles_mut() {
                    if let Some(ref provider_handle) = handles.provider {
                        login::run(provider, key, tx, provider_handle.tx().clone());
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (moved from runie-core::effect_payload)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::message::ChatMessage;

    #[test]
    fn copy_last_response_extracts_assistant_text() {
        let mut state = AppState::default();
        state
            .session_mut()
            .messages
            .push(ChatMessage::system("sys".to_string()));
        state
            .session
            .messages
            .push(ChatMessage::assistant("the answer".to_string()));
        let payload = extract(&CoreEvent::CopyLastResponse, &mut state);
        assert!(
            matches!(payload, Some(EffectPayload::CopyToClipboard { text }) if text == "the answer")
        );
    }

    #[test]
    fn copy_last_response_empty_when_no_assistant() {
        let mut state = AppState::default();
        assert!(extract(&CoreEvent::CopyLastResponse, &mut state).is_none());
    }
}
