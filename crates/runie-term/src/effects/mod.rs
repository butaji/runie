//! Typed effect commands dispatched from the main event loop.

use runie_core::model::ThinkingLevel;
use runie_core::{AppState, ChatMessage, Event as CoreEvent, Snapshot};
use tokio::sync::{mpsc, watch};

use crate::terminal::caps::TerminalCapabilities;

mod clipboard;
mod editor;
mod login;
mod share;
mod subagent;
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
    SpawnAgent {
        prompt: String,
        provider: String,
        model: String,
        thinking: ThinkingLevel,
        read_only: bool,
        skills_context: String,
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
            CoreEvent::ShareSession => Some(Self::ShareSession {
                messages: state.session.messages.clone(),
                display_name: state.session.session_display_name.clone(),
            }),
            CoreEvent::Suspend => Some(Self::Suspend {
                terminal_caps: *caps,
            }),
            CoreEvent::LoginFlowSubmitKey { .. } => login_command(evt),
            CoreEvent::SpawnAgent { prompt } => Some(Self::SpawnAgent {
                prompt: prompt.clone(),
                provider: state.config.current_provider.clone(),
                model: state.config.current_model.clone(),
                thinking: state.config.thinking_level,
                read_only: state.config.read_only,
                skills_context: runie_core::skills::build_skills_context(&state.skills),
            }),
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
            Self::ShareSession {
                messages,
                display_name,
            } => share::run(messages, display_name, tx),
            Self::Suspend { terminal_caps } => suspend::run(terminal_caps, render_tx, state),
            Self::LoginFlowSubmitKey { provider, key } => login::run(provider, key, tx),
            Self::SpawnAgent {
                prompt,
                provider,
                model,
                thinking,
                read_only,
                skills_context,
            } => subagent::run(
                prompt,
                provider,
                model,
                thinking,
                read_only,
                skills_context,
                tx,
            ),
        }
    }
}

fn login_command(evt: &CoreEvent) -> Option<EffectCommand> {
    if let CoreEvent::LoginFlowSubmitKey { provider, key } = evt {
        Some(EffectCommand::LoginFlowSubmitKey {
            provider: provider.clone(),
            key: key.clone(),
        })
    } else {
        None
    }
}
