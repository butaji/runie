//! Typed effect commands dispatched from the main event loop.

use runie_core::{AppState, ChatMessage, Event as CoreEvent, Snapshot};
use tokio::sync::{mpsc, watch};

use crate::terminal::caps::TerminalCapabilities;

mod clipboard;
mod editor;
pub(crate) mod login;
mod share;
mod suspend;

pub enum EffectCommand {
    OpenExternalEditor {
        text: String,
    },
    CopyToClipboard {
        text: String,
    },
    CopyLastResponse {
        messages: Vec<ChatMessage>,
    },
    CopySelectedBlock {
        text: String,
    },
    CopyBlockMetadata {
        text: String,
    },
    ShareSession {
        messages: Vec<ChatMessage>,
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
        state: &AppState,
        caps: &TerminalCapabilities,
    ) -> Option<Self> {
        match evt {
            CoreEvent::OpenExternalEditor => Some(Self::OpenExternalEditor {
                text: state.input.input.clone(),
            }),
            CoreEvent::CopyToClipboard(text) => Some(Self::CopyToClipboard { text: text.clone() }),
            CoreEvent::CopyLastResponse => Some(Self::CopyLastResponse {
                messages: state.session.messages.clone(),
            }),
            CoreEvent::CopySelectedBlock => state
                .copy_selected_post_text()
                .map(|text| Self::CopySelectedBlock { text }),
            CoreEvent::CopyBlockMetadata => state
                .copy_selected_post_metadata()
                .map(|text| Self::CopyBlockMetadata { text }),
            CoreEvent::ShareSession => Some(Self::ShareSession {
                messages: state.session.messages.clone(),
                display_name: state.session.session_display_name.clone(),
            }),
            CoreEvent::Suspend => Some(Self::Suspend {
                terminal_caps: *caps,
            }),
            CoreEvent::SubmitKey { .. } => login_command(evt),
            _ => None,
        }
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
            Self::CopyLastResponse { messages } => {
                clipboard::copy_last_response(messages, caps);
            }
            Self::CopySelectedBlock { text } => clipboard::copy_to_clipboard(text, caps),
            Self::CopyBlockMetadata { text } => clipboard::copy_to_clipboard(text, caps),
            Self::ShareSession {
                messages,
                display_name,
            } => share::run(messages, display_name, tx),
            Self::Suspend { terminal_caps } => suspend::run(terminal_caps, render_tx, state),
            Self::LoginFlowSubmitKey { provider, key } => {
                if let Some(ref provider_tx) = state.provider_tx {
                    login::run(provider, key, tx, provider_tx.clone());
                }
            }
        }
    }
}

fn login_command(evt: &CoreEvent) -> Option<EffectCommand> {
    if let CoreEvent::SubmitKey { provider, key } = evt {
        Some(EffectCommand::LoginFlowSubmitKey {
            provider: provider.clone(),
            key: key.clone(),
        })
    } else {
        None
    }
}
