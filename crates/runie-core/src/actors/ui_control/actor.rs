//! `UiControlActor` — owns dialogs, login flow, and quit state.

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::commands::DialogState;
use crate::event::{DialogKind, Event};
use crate::login_flow::LoginFlowState;

use super::messages::{UiControlActorHandle, UiControlMsg};

/// Actor that owns UI control state.
///
/// Manages:
/// - Dialog stack (`dialog`, `back_stack`)
/// - Login flow lifecycle
/// - Quit flag (`quit_requested`)
///
/// This actor ensures these shared mutable fields are updated through
/// a single authoritative owner rather than scattered direct mutations.
pub struct UiControlActor {
    /// Currently open dialog (if any).
    dialog: Option<DialogState>,
    /// Stack of dialogs pushed beneath the current one.
    back_stack: Vec<DialogState>,
    /// Login flow state machine (if active).
    flow: Option<LoginFlowState>,
    /// Whether the application should quit.
    quit_requested: bool,
}

impl UiControlActor {
    /// Spawn a `UiControlActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (UiControlActorHandle, ActorHandle) {
        let actor = Self {
            dialog: None,
            back_stack: Vec::new(),
            flow: None,
            quit_requested: false,
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (UiControlActorHandle::new(tx), handle)
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&mut self, msg: UiControlMsg) {
        match msg {
            UiControlMsg::OpenDialog(d) => self.open_dialog(d),
            UiControlMsg::PushDialog(d) => self.push_dialog(d),
            UiControlMsg::PopDialog => self.pop_dialog(),
            UiControlMsg::CloseAllDialogs => self.close_all(),
            UiControlMsg::StartLoginFlow => self.start_login_flow(),
            UiControlMsg::LoginFlowStep(s) => self.login_flow_step(s),
            UiControlMsg::CancelLoginFlow => self.cancel_login_flow(),
            UiControlMsg::RequestQuit => self.request_quit(false),
            UiControlMsg::ForceQuit => self.request_quit(true),
        }
    }

    // ── Dialog lifecycle ───────────────────────────────────────────────────

    fn open_dialog(&mut self, d: DialogState) {
        if let Some(current) = self.dialog.take() {
            self.back_stack.push(current);
        }
        self.dialog = Some(d);
    }

    fn push_dialog(&mut self, d: DialogState) {
        if let Some(current) = self.dialog.take() {
            self.back_stack.push(current);
        }
        self.dialog = Some(d);
    }

    fn pop_dialog(&mut self) {
        self.dialog = self.back_stack.pop();
    }

    fn close_all(&mut self) {
        self.dialog = None;
        self.back_stack.clear();
    }

    // ── Login flow ─────────────────────────────────────────────────────────

    fn start_login_flow(&mut self) {
        self.dialog = None;
        self.back_stack.clear();
        self.flow = Some(LoginFlowState::new());
    }

    fn login_flow_step(&mut self, state: LoginFlowState) {
        self.flow = Some(state);
    }

    fn cancel_login_flow(&mut self) {
        self.flow = None;
    }

    // ── Quit ───────────────────────────────────────────────────────────────

    fn request_quit(&mut self, _force: bool) {
        self.quit_requested = true;
    }

    // ── Fact emission helpers ─────────────────────────────────────────────

    fn emit_dialog_changed(&self, bus: &EventBus<Event>, was_open: bool) {
        if was_open && !self.dialog.is_some() {
            bus.publish(Event::DialogClosed);
        } else if !was_open && self.dialog.is_some() {
            if let Some(ref d) = self.dialog {
                let kind = dialog_kind(d);
                bus.publish(Event::DialogOpened { kind });
            }
        }
    }

    fn emit_login_flow_changed(&self, bus: &EventBus<Event>, msg: &UiControlMsg) {
        if let Some(ref f) = self.flow {
            bus.publish(Event::LoginFlowStepChanged {
                step: f.step.clone(),
                provider: f.provider.clone(),
            });
        } else if self.flow.is_none() && !matches!(msg, UiControlMsg::LoginFlowStep(_)) {
            if matches!(msg, UiControlMsg::CancelLoginFlow) {
                bus.publish(Event::LoginFlowClosed);
            }
        }
    }

    fn is_force_quit(msg: &UiControlMsg) -> bool {
        matches!(msg, UiControlMsg::ForceQuit)
    }

    #[cfg(test)]
    pub fn state(&self) -> (&Option<DialogState>, &[DialogState], &Option<LoginFlowState>, bool) {
        (&self.dialog, &self.back_stack, &self.flow, self.quit_requested)
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
        DialogState::PanelStack(_) => DialogKind::Settings, // Fallback
    }
}

impl Actor for UiControlActor {
    type Msg = UiControlMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            let was_open = self.dialog.is_some();
            self.handle_msg(msg.clone());
            self.emit_dialog_changed(&bus, was_open);
            self.emit_login_flow_changed(&bus, &msg);
            if self.quit_requested {
                bus.publish(Event::QuitRequested { forced: Self::is_force_quit(&msg) });
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_dialog_pushes_to_back_stack() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = UiControlActor::spawn(bus);
        handle
            .send(UiControlMsg::OpenDialog(DialogState::CommandPalette(
                crate::dialog::stack::PanelStack {
                    panels: vec![],
                },
            )))
            .await;
        handle
            .send(UiControlMsg::OpenDialog(DialogState::Settings(
                crate::dialog::stack::PanelStack {
                    panels: vec![],
                },
            )))
            .await;
        drop(handle);
    }

    #[tokio::test]
    async fn start_login_flow_clears_dialogs() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = UiControlActor::spawn(bus);
        handle
            .send(UiControlMsg::OpenDialog(DialogState::Settings(
                crate::dialog::stack::PanelStack {
                    panels: vec![],
                },
            )))
            .await;
        handle.send(UiControlMsg::StartLoginFlow).await;
        drop(handle);
    }

    #[tokio::test]
    async fn pop_restores_previous_dialog() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = UiControlActor::spawn(bus);
        handle
            .send(UiControlMsg::OpenDialog(DialogState::Settings(
                crate::dialog::stack::PanelStack {
                    panels: vec![],
                },
            )))
            .await;
        handle
            .send(UiControlMsg::OpenDialog(DialogState::Welcome))
            .await;
        handle.send(UiControlMsg::PopDialog).await;
        drop(handle);
    }

    #[tokio::test]
    async fn quit_actor_stops_after_request() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _actor) = UiControlActor::spawn(bus);
        handle.send(UiControlMsg::RequestQuit).await;
        drop(handle);
        let events: Vec<Event> = drain_events(&mut sub, 10);
        assert!(events.iter().any(|e| matches!(e, Event::QuitRequested { .. })));
    }

    fn drain_events<E: Clone + Send + 'static>(
        sub: &mut tokio::sync::broadcast::Receiver<E>,
        count: usize,
    ) -> Vec<E> {
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            match sub.try_recv() {
                Ok(e) => events.push(e),
                Err(_) => break,
            }
        }
        events
    }
}
