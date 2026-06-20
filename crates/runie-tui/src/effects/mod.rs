//! Typed effect commands dispatched from the main event loop.
//!
//! Effect payload computation lives in `runie_core::effect_payload`; this module
//! only translates self-contained payloads into terminal-side side effects.

use runie_core::effect_payload::EffectPayload;
use runie_core::{AppState, Event as CoreEvent, Snapshot};
use tokio::sync::{mpsc, watch};

use crate::terminal::caps::TerminalCapabilities;

mod clipboard;
mod editor;
pub(crate) mod login;
mod share;
mod suspend;

pub enum EffectCommand {
    OpenExternalEditor { text: String },
    CopyToClipboard { text: String },
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
        state: &AppState,
        caps: &TerminalCapabilities,
    ) -> Option<Self> {
        let payload = runie_core::effect_payload::extract(evt, state)?;
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
            EffectPayload::Suspend => Self::Suspend { terminal_caps: *caps },
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
                if let Some(ref provider_tx) = state.provider_tx {
                    login::run(provider, key, tx, provider_tx.clone());
                }
            }
        }
    }
}
