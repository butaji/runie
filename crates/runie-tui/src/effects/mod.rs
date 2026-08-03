//! Typed effect commands dispatched from the main event loop.
//!
//! Effect commands are translated into IoActor messages, keeping all IO in
//! the async actor layer per the architecture rules.

use runie_core::{AppState, Event as CoreEvent};

pub(crate) mod login;

// ---------------------------------------------------------------------------
// Effect command
// ---------------------------------------------------------------------------

/// User-initiated side effects dispatched from the event loop to IoActor.
#[derive(Debug)]
pub enum EffectCommand {
    OpenExternalEditor { text: String },
    CopyToClipboard { text: String },
    ShareSession { messages: Vec<runie_core::ChatMessage>, display_name: Option<String> },
    Suspend,
    LoginFlowSubmitKey { provider: String, key: String },
}

impl EffectCommand {
    /// Build an effect command from a core event, if the event is an effect.
    /// Returns `None` for events that are not side effects.
    pub fn try_from_event(
        evt: &CoreEvent,
        state: &mut AppState,
        _caps: &crate::terminal::caps::TermCaps,
    ) -> Option<Self> {
        match evt {
            CoreEvent::OpenExternalEditor => Some(Self::OpenExternalEditor { text: state.input().input().to_string() }),
            CoreEvent::CopyToClipboard(text) => Some(Self::CopyToClipboard { text: text.clone() }),
            CoreEvent::CopyLastResponse => {
                let text = last_assistant_text(state.session().messages());
                if text.is_empty() {
                    return None;
                }
                Some(Self::CopyToClipboard { text })
            }
            CoreEvent::CopySelectedBlock => state
                .copy_selected_post_text()
                .map(|text| Self::CopyToClipboard { text }),
            CoreEvent::CopyBlockMetadata => state
                .copy_selected_post_metadata()
                .map(|text| Self::CopyToClipboard { text }),
            CoreEvent::ShareSession => Some(Self::ShareSession {
                messages: state.session().messages().to_vec(),
                display_name: state.session().session_display_name().map(String::from),
            }),
            CoreEvent::Suspend => Some(Self::Suspend),
            CoreEvent::SubmitKey { provider, key } => {
                Some(Self::LoginFlowSubmitKey { provider: provider.clone(), key: key.clone() })
            }
            _ => None,
        }
    }

    /// Dispatch the effect via IoActor (async).
    pub async fn dispatch_async(self, state: &AppState) {
        let io_handle = state.actor_handles().as_ref().map(|h| h.io.clone());
        let Some(handle) = io_handle else {
            return;
        };

        match self {
            Self::OpenExternalEditor { text } => {
                handle.open_external_editor(text).await;
            }
            Self::CopyToClipboard { text } => {
                handle.write_clipboard(text).await;
            }
            Self::ShareSession { messages, display_name } => {
                handle.share_session(messages, display_name).await;
            }
            Self::Suspend => {
                handle.suspend_process().await;
            }
            Self::LoginFlowSubmitKey { .. } => {
                // Login validation uses ProviderActor, handled separately
            }
        }
    }
}

/// Extract assistant message text from a message list.
fn last_assistant_text(messages: &[runie_core::ChatMessage]) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == runie_core::Role::Assistant)
        .map(|m| m.content())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::message::ChatMessage;

    #[test]
    fn effect_command_copy_last_response_extracts_assistant_text() {
        let mut state = AppState::default();
        state
            .session_mut()
            .messages
            .push(ChatMessage::system("sys".to_string()));
        state
            .session
            .messages
            .push(ChatMessage::assistant("the answer".to_string()));
        let caps = crate::terminal::caps::TermCaps::default();
        let cmd = EffectCommand::try_from_event(&CoreEvent::CopyLastResponse, &mut state, &caps);
        assert!(matches!(cmd, Some(EffectCommand::CopyToClipboard { text }) if text == "the answer"));
    }

    #[test]
    fn effect_command_copy_last_response_empty_when_no_assistant() {
        let mut state = AppState::default();
        let caps = crate::terminal::caps::TermCaps::default();
        assert!(EffectCommand::try_from_event(&CoreEvent::CopyLastResponse, &mut state, &caps).is_none());
    }

    #[test]
    fn effect_command_submit_key_roundtrips() {
        let mut state = AppState::default();
        let caps = crate::terminal::caps::TermCaps::default();
        let cmd = EffectCommand::try_from_event(
            &CoreEvent::SubmitKey { provider: "anthropic".into(), key: "sk-123".into() },
            &mut state,
            &caps,
        );
        match cmd {
            Some(EffectCommand::LoginFlowSubmitKey { provider, key }) => {
                assert_eq!(provider, "anthropic");
                assert_eq!(key, "sk-123");
            }
            other => panic!("expected LoginFlowSubmitKey, got {other:?}"),
        }
    }
}
