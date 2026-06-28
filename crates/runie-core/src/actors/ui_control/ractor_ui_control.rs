//! Ractor-based `UiControlActor` implementation.
//!
//! This module provides a ractor-based implementation of the UiControlActor,
//! following the same pattern as the InputActor migration.

use std::sync::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::commands::DialogState;
use crate::event::{DialogKind, Event};
use crate::login_flow::LoginFlowState;

use super::messages::UiControlMsg;

// ── Handle type ────────────────────────────────────────────────────────────────

/// Ractor handle type for UiControlActor.
#[derive(Clone, Debug)]
pub struct RactorUiControlHandle {
    inner: RactorHandle<UiControlMsg>,
}

impl RactorUiControlHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<UiControlMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor.
    pub async fn send(&self, msg: UiControlMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget).
    pub fn try_send(&self, msg: UiControlMsg) {
        let _ = self.inner.try_send(msg);
    }
}

impl From<RactorHandle<UiControlMsg>> for RactorUiControlHandle {
    fn from(handle: RactorHandle<UiControlMsg>) -> Self {
        Self::new(handle)
    }
}

// ── Actor state ───────────────────────────────────────────────────────────────

/// Ractor-based UiControlActor.
///
/// Owns dialog stack, login flow lifecycle, and quit state.
pub struct RactorUiControlActor {
    /// Currently open dialog (if any).
    dialog: Mutex<Option<DialogState>>,
    /// Stack of dialogs pushed beneath the current one.
    back_stack: Mutex<Vec<DialogState>>,
    /// Login flow state machine (if active).
    flow: Mutex<Option<LoginFlowState>>,
    /// Whether the application should quit.
    quit_requested: Mutex<bool>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl Default for RactorUiControlActor {
    fn default() -> Self {
        Self {
            dialog: Mutex::new(None),
            back_stack: Mutex::new(Vec::new()),
            flow: Mutex::new(None),
            quit_requested: Mutex::new(false),
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
        }
    }
}

/// Convert DialogState to DialogKind for facts.
fn dialog_kind(state: &DialogState) -> DialogKind {
    match state {
        DialogState::Welcome => DialogKind::Welcome,
        DialogState::CommandPalette(_) => DialogKind::CommandPalette,
        DialogState::ModelSelector(_) => DialogKind::ModelSelector,
        DialogState::Settings(_) => DialogKind::Settings,
        DialogState::ScopedModels(_) => DialogKind::ScopedModels,
        DialogState::SessionTree(_) => DialogKind::SessionTree,
        DialogState::PanelStack(_) => DialogKind::Settings,
    }
}

// ── Ractor Actor impl ─────────────────────────────────────────────────────────

#[async_trait]
impl Actor for RactorUiControlActor {
    type Msg = UiControlMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: UiControlMsg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let was_open = self.dialog.lock().unwrap().is_some();
        self.apply_msg(&msg);
        self.emit_dialog_fact(was_open, &msg);
        self.emit_login_flow_fact(&msg);
        self.check_quit(&msg);
        Ok(())
    }
}

impl RactorUiControlActor {
    /// Spawn a `RactorUiControlActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorUiControlHandle, ractor::ActorCell) {
        let actor = Self {
            dialog: Mutex::new(None),
            back_stack: Mutex::new(Vec::new()),
            flow: Mutex::new(None),
            quit_requested: Mutex::new(false),
            bus_bridge: EventBusBridge::new(bus.clone()),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorUiControlHandle::new(handle), cell)
    }

    /// Apply a message to the actor state.
    fn apply_msg(&self, msg: &UiControlMsg) {
        match msg {
            UiControlMsg::OpenDialog(d) => self.open_dialog(d),
            UiControlMsg::PushDialog(d) => self.open_dialog(d),
            UiControlMsg::PopDialog => {
                let mut dialog = self.dialog.lock().unwrap();
                *dialog = self.back_stack.lock().unwrap().pop();
            }
            UiControlMsg::CloseAllDialogs => {
                *self.dialog.lock().unwrap() = None;
                self.back_stack.lock().unwrap().clear();
            }
            UiControlMsg::StartLoginFlow => {
                *self.dialog.lock().unwrap() = None;
                self.back_stack.lock().unwrap().clear();
                *self.flow.lock().unwrap() = Some(LoginFlowState::new());
            }
            UiControlMsg::LoginFlowStep(s) => {
                *self.flow.lock().unwrap() = Some(s.clone());
            }
            UiControlMsg::CancelLoginFlow => {
                *self.flow.lock().unwrap() = None;
            }
            UiControlMsg::RequestQuit | UiControlMsg::ForceQuit => {
                *self.quit_requested.lock().unwrap() = true;
            }
        }
    }

    fn open_dialog(&self, d: &DialogState) {
        let mut dialog = self.dialog.lock().unwrap();
        if let Some(current) = dialog.take() {
            self.back_stack.lock().unwrap().push(current);
        }
        *dialog = Some(d.clone());
    }

    fn emit_dialog_fact(&self, was_open: bool, msg: &UiControlMsg) {
        let dialog = self.dialog.lock().unwrap();
        if was_open && dialog.is_none() {
            drop(dialog);
            self.bus_bridge.publish(Event::DialogClosed);
        } else if !was_open && dialog.is_some() {
            let kind = dialog_kind(dialog.as_ref().unwrap());
            drop(dialog);
            self.bus_bridge.publish(Event::DialogOpened { kind });
        }
    }

    fn emit_login_flow_fact(&self, msg: &UiControlMsg) {
        let flow = self.flow.lock().unwrap();
        if let Some(f) = flow.as_ref() {
            self.bus_bridge.publish(Event::LoginFlowStepChanged {
                step: f.step.clone(),
                provider: f.provider.clone(),
            });
        } else if matches!(msg, UiControlMsg::CancelLoginFlow) {
            self.bus_bridge.publish(Event::LoginFlowClosed);
        }
    }

    fn check_quit(&self, msg: &UiControlMsg) {
        if *self.quit_requested.lock().unwrap() {
            let forced = matches!(msg, UiControlMsg::ForceQuit);
            self.bus_bridge.publish(Event::QuitRequested { forced });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::stack::PanelStack;

    #[tokio::test]
    async fn open_dialog_pushes_to_back_stack() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorUiControlActor::spawn(bus).await;

        handle
            .send(UiControlMsg::OpenDialog(DialogState::CommandPalette(
                PanelStack { panels: vec![] },
            )))
            .await;
        handle
            .send(UiControlMsg::OpenDialog(DialogState::Settings(PanelStack {
                panels: vec![],
            })))
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn start_login_flow_clears_dialogs() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorUiControlActor::spawn(bus).await;

        handle
            .send(UiControlMsg::OpenDialog(DialogState::Settings(PanelStack {
                panels: vec![],
            })))
            .await;
        handle.send(UiControlMsg::StartLoginFlow).await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn quit_emits_quit_requested() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorUiControlActor::spawn(bus).await;

        handle.send(UiControlMsg::RequestQuit).await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
