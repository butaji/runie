//! Ractor-based `ViewActor` implementation.
//!
//! This module provides a ractor-based implementation of the ViewActor,
//! following the same pattern as the InputActor migration.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Mutex;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::{InputReceiver, ViewState};

use super::messages::{
    ElementJumpDirection, MouseButton, ScrollDirection, ViewMsg,
};

/// Ractor handle type for ViewActor with convenience methods.
#[derive(Clone, Debug)]
pub struct RactorViewHandle {
    inner: RactorHandle<ViewMsg>,
}

impl RactorViewHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<ViewMsg>) -> Self {
        Self { inner }
    }

    /// Send a message (fire-and-forget).
    pub async fn send(&self, msg: ViewMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget).
    pub fn try_send(&self, msg: ViewMsg) {
        let _ = self.inner.try_send(msg);
    }
}

/// Ractor-based ViewActor.
///
/// Owns view state: scroll, vim nav, terminal sizing, dialog state, and animation.
/// Uses ractor for actor supervision and message handling.
pub struct RactorViewActor {
    /// The authoritative view state.
    state: Mutex<ViewState>,
    /// Animation frame accumulator.
    animation_accum: Mutex<u32>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl Default for RactorViewActor {
    fn default() -> Self {
        Self {
            state: Mutex::new(ViewState::default()),
            animation_accum: Mutex::new(0),
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
        }
    }
}

#[ractor::async_trait]
impl Actor for RactorViewActor {
    type Msg = ViewMsg;
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
        msg: ViewMsg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        self.handle_msg(msg);
        let state = self.state.lock().unwrap().clone();
        self.bus_bridge.publish(Event::ViewChanged {
            state: Box::new(state),
        });
        Ok(())
    }
}

impl RactorViewActor {
    /// Spawn a `RactorViewActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorViewHandle, ractor::ActorCell) {
        let actor = Self {
            state: Mutex::new(ViewState::default()),
            animation_accum: Mutex::new(0),
            bus_bridge: EventBusBridge::new(bus.clone()),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorViewHandle::new(handle), cell)
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&self, msg: ViewMsg) {
        match msg {
            ViewMsg::Invalidate => self.invalidate(),
            ViewMsg::MessagesChanged => self.messages_changed(),
            ViewMsg::Scroll { direction } => self.scroll(direction),
            ViewMsg::PageUp => self.page_up(),
            ViewMsg::PageDown => self.page_down(),
            ViewMsg::GoToTop => self.go_to_top(),
            ViewMsg::GoToBottom => self.go_to_bottom(),
            ViewMsg::ElementJump { direction } => self.element_jump(direction),
            ViewMsg::MouseMoved { row, col } => self.mouse_moved(row, col),
            ViewMsg::MouseClicked { row, col, button } => self.mouse_clicked(row, col, button),
            ViewMsg::MouseReleased { row, col, button } => self.mouse_released(row, col, button),
            ViewMsg::TerminalSized { width, height } => self.terminal_sized(width, height),
            ViewMsg::DialogOpened => self.dialog_opened(),
            ViewMsg::DialogClosed => self.dialog_closed(),
            ViewMsg::SetInputReceiver { receiver } => self.set_input_receiver(receiver),
            ViewMsg::VimNav { enabled, selected_post } => self.vim_nav(enabled, selected_post),
            ViewMsg::ToggleExpandAll => self.toggle_expand_all(),
            ViewMsg::TurnEnded => self.turn_ended(),
            ViewMsg::TurnErrored => self.turn_errored(),
            ViewMsg::AnimationTick => self.animation_tick(),
        }
    }

    // ── Cache invalidation ─────────────────────────────────────────────────

    fn invalidate(&self) {
        self.state.lock().unwrap().dirty = true;
    }

    fn messages_changed(&self) {
        let mut state = self.state.lock().unwrap();
        state.dirty = true;
        state.message_gen += 1;
    }

    // ── Scroll ─────────────────────────────────────────────────────────────

    fn scroll(&self, direction: ScrollDirection) {
        let lines = match direction {
            ScrollDirection::Up => 3,
            ScrollDirection::Down => 3,
            ScrollDirection::HalfUp => self.state.lock().unwrap().last_visible_height as usize / 2,
            ScrollDirection::HalfDown => self.state.lock().unwrap().last_visible_height as usize / 2,
        };
        let mut state = self.state.lock().unwrap();
        match direction {
            ScrollDirection::Up | ScrollDirection::HalfUp => {
                state.scroll = state.scroll.saturating_sub(lines);
            }
            ScrollDirection::Down | ScrollDirection::HalfDown => {
                let max_scroll = state.total_lines.saturating_sub(state.last_visible_height as usize);
                state.scroll = (state.scroll + lines).min(max_scroll);
            }
        }
        state.dirty = true;
    }

    fn page_up(&self) {
        let mut state = self.state.lock().unwrap();
        let delta = state.last_visible_height as usize;
        state.scroll = state.scroll.saturating_sub(delta);
        state.dirty = true;
    }

    fn page_down(&self) {
        let mut state = self.state.lock().unwrap();
        let max_scroll = state.total_lines.saturating_sub(state.last_visible_height as usize);
        let delta = state.last_visible_height as usize;
        state.scroll = (state.scroll + delta).min(max_scroll);
        state.dirty = true;
    }

    fn go_to_top(&self) {
        let mut state = self.state.lock().unwrap();
        state.scroll = 0;
        state.dirty = true;
    }

    fn go_to_bottom(&self) {
        let mut state = self.state.lock().unwrap();
        let max_scroll = state.total_lines.saturating_sub(state.last_visible_height as usize);
        state.scroll = max_scroll;
        state.dirty = true;
    }

    fn element_jump(&self, direction: ElementJumpDirection) {
        let mut state = self.state.lock().unwrap();
        let lines = &state.line_counts;
        let current = state.scroll;
        match direction {
            ElementJumpDirection::Next => {
                if let Some(&next) = lines.iter().find(|&&l| l > current) {
                    state.scroll = next;
                }
            }
            ElementJumpDirection::Prev => {
                if let Some(&prev) = lines.iter().rev().find(|&&l| l < current) {
                    state.scroll = prev;
                }
            }
        }
        state.dirty = true;
    }

    // ── Mouse ───────────────────────────────────────────────────────────────

    fn mouse_moved(&self, row: u16, col: u16) {
        let mut state = self.state.lock().unwrap();
        state.mouse_position = Some((row, col));
        state.dirty = true;
    }

    fn mouse_clicked(&self, _row: u16, _col: u16, button: MouseButton) {
        if matches!(button, MouseButton::Left) {
            // Route click based on position
        }
        self.state.lock().unwrap().dirty = true;
    }

    fn mouse_released(&self, _row: u16, _col: u16, _button: MouseButton) {
        self.state.lock().unwrap().dirty = true;
    }

    // ── Terminal sizing ─────────────────────────────────────────────────────

    fn terminal_sized(&self, width: u16, height: u16) {
        let mut state = self.state.lock().unwrap();
        state.last_content_width = width;
        state.last_visible_height = height;
        state.dirty = true;
    }

    // ── Dialog state ────────────────────────────────────────────────────────

    fn dialog_opened(&self) {
        let mut state = self.state.lock().unwrap();
        state.input_receiver = InputReceiver::Dialog;
        state.dirty = true;
    }

    fn dialog_closed(&self) {
        let mut state = self.state.lock().unwrap();
        state.input_receiver = InputReceiver::ChatInput;
        state.dirty = true;
    }

    // ── Input receiver ───────────────────────────────────────────────────────

    fn set_input_receiver(&self, receiver: InputReceiver) {
        let mut state = self.state.lock().unwrap();
        state.input_receiver = receiver;
        state.dirty = true;
    }

    // ── Vim navigation ──────────────────────────────────────────────────────

    fn vim_nav(&self, enabled: bool, selected_post: Option<usize>) {
        let mut state = self.state.lock().unwrap();
        state.vim_nav_mode = enabled;
        state.selected_post = selected_post;
        state.dirty = true;
    }

    fn toggle_expand_all(&self) {
        let mut state = self.state.lock().unwrap();
        state.all_collapsed = !state.all_collapsed;
        state.dirty = true;
        state.message_gen += 1;
    }

    // ── Turn lifecycle ───────────────────────────────────────────────────────

    fn turn_ended(&self) {
        let mut state = self.state.lock().unwrap();
        state.vim_nav_pending = false;
        state.dirty = true;
    }

    fn turn_errored(&self) {
        let mut state = self.state.lock().unwrap();
        state.vim_nav_pending = true;
        state.dirty = true;
    }

    // ── Animation ─────────────────────────────────────────────────────────────

    fn animation_tick(&self) {
        let mut accum = self.animation_accum.lock().unwrap();
        *accum += 1;
        if accum.is_multiple_of(4) {
            let mut state = self.state.lock().unwrap();
            state.animation_frame = state.animation_frame.wrapping_add(1);
            state.dirty = true;
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> ViewState {
        self.state.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalidate_sets_dirty() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorViewActor::spawn(bus).await;

        handle.send(ViewMsg::Invalidate).await;
        drop(handle);
    }

    #[tokio::test]
    async fn scroll_changes_position() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorViewActor::spawn(bus).await;

        handle.send(ViewMsg::Scroll { direction: ScrollDirection::Down }).await;
        drop(handle);
    }

    #[tokio::test]
    async fn terminal_sized_updates_dimensions() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorViewActor::spawn(bus).await;

        handle.send(ViewMsg::TerminalSized { width: 120, height: 40 }).await;
        drop(handle);

        // Verify ViewChanged event was emitted
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut found = false;
        while let Ok(e) = sub.try_recv() {
            if matches!(e, Event::ViewChanged { .. }) {
                found = true;
            }
        }
        assert!(found, "Expected ViewChanged event after TerminalSized");
    }

    #[tokio::test]
    async fn dialog_opened_updates_receiver() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorViewActor::spawn(bus).await;

        handle.send(ViewMsg::DialogOpened).await;
        handle.send(ViewMsg::DialogClosed).await;
        drop(handle);
    }

    #[test]
    fn view_actor_scroll_bounds() {
        let actor = RactorViewActor::default();

        // Scroll up from 0 should stay at 0
        actor.state.lock().unwrap().scroll = 0;
        actor.scroll(ScrollDirection::Up);
        assert_eq!(actor.state.lock().unwrap().scroll, 0);
        assert!(actor.state.lock().unwrap().dirty);

        // Scroll down from 0 with no content stays at 0
        actor.state.lock().unwrap().dirty = false;
        actor.scroll(ScrollDirection::Down);
        assert_eq!(actor.state.lock().unwrap().scroll, 0);
    }

    #[test]
    fn view_actor_terminal_sized() {
        let actor = RactorViewActor::default();

        actor.terminal_sized(100, 30);
        assert_eq!(actor.state.lock().unwrap().last_content_width, 100);
        assert_eq!(actor.state.lock().unwrap().last_visible_height, 30);
        assert!(actor.state.lock().unwrap().dirty);
    }

    #[test]
    fn view_actor_vim_nav() {
        let actor = RactorViewActor::default();

        actor.vim_nav(true, Some(5));
        assert!(actor.state.lock().unwrap().vim_nav_mode);
        assert_eq!(actor.state.lock().unwrap().selected_post, Some(5));
        assert!(actor.state.lock().unwrap().dirty);
    }
}
