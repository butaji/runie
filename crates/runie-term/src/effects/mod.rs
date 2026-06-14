use runie_core::event::{ControlEvent, DialogEvent, EditEvent, InputEvent, SystemEvent};
use runie_core::model::AppState;
use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

use crate::terminal::caps::TerminalCapabilities;

pub enum EffectCommand {
    LoginFlowSubmitKey { provider: String, key: String },
}

impl EffectCommand {
    pub fn try_from_event(
        _evt: &CoreEvent,
        _state: &AppState,
        _caps: &TerminalCapabilities,
    ) -> Option<Self> {
        None
    }

    pub fn dispatch(self, _tx: mpsc::Sender<CoreEvent>) {
    }
}
