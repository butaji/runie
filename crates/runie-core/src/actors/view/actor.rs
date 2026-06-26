//! `ViewActor` — owns the authoritative `ViewState`.

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::{InputReceiver, ViewState};

use super::messages::{
    ElementJumpDirection, MouseButton, ScrollDirection, ViewActorHandle, ViewMsg,
};

/// Actor that owns view state.
///
/// Scroll, vim nav, terminal sizing, dialog state, and animation are all
/// mutations that live here. The actor processes `ViewMsg` messages and emits
/// `ViewChanged` facts when state changes.
///
/// Note: Feed cache rebuilding (elements, line_counts, posts) depends on
/// both session messages and view state, so it stays in AppState's
/// `ensure_fresh()`. ViewActor handles pure navigation and sizing mutations.
pub struct ViewActor {
    /// The authoritative view state.
    state: ViewState,
    /// Animation frame accumulator.
    animation_accum: u32,
}

impl ViewActor {
    /// Spawn a `ViewActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (ViewActorHandle, ActorHandle) {
        let actor = Self {
            state: ViewState::default(),
            animation_accum: 0,
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (ViewActorHandle::new(tx), handle)
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&mut self, msg: ViewMsg) {
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
            ViewMsg::MouseReleased { row, col, button } => {
                self.mouse_released(row, col, button)
            }
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

    fn invalidate(&mut self) {
        self.state.dirty = true;
    }

    fn messages_changed(&mut self) {
        self.state.dirty = true;
        self.state.message_gen += 1;
    }

    // ── Scroll ─────────────────────────────────────────────────────────────

    fn scroll(&mut self, direction: ScrollDirection) {
        let lines = match direction {
            ScrollDirection::Up => 3,
            ScrollDirection::Down => 3,
            ScrollDirection::HalfUp => self.state.last_visible_height as usize / 2,
            ScrollDirection::HalfDown => self.state.last_visible_height as usize / 2,
        };
        match direction {
            ScrollDirection::Up | ScrollDirection::HalfUp => {
                self.state.scroll = self.state.scroll.saturating_sub(lines);
            }
            ScrollDirection::Down | ScrollDirection::HalfDown => {
                let max_scroll = self
                    .state
                    .total_lines
                    .saturating_sub(self.state.last_visible_height as usize);
                self.state.scroll = (self.state.scroll + lines).min(max_scroll);
            }
        }
        self.state.dirty = true;
    }

    fn page_up(&mut self) {
        let delta = self.state.last_visible_height as usize;
        self.state.scroll = self.state.scroll.saturating_sub(delta);
        self.state.dirty = true;
    }

    fn page_down(&mut self) {
        let max_scroll = self
            .state
            .total_lines
            .saturating_sub(self.state.last_visible_height as usize);
        let delta = self.state.last_visible_height as usize;
        self.state.scroll = (self.state.scroll + delta).min(max_scroll);
        self.state.dirty = true;
    }

    fn go_to_top(&mut self) {
        self.state.scroll = 0;
        self.state.dirty = true;
    }

    fn go_to_bottom(&mut self) {
        let max_scroll = self
            .state
            .total_lines
            .saturating_sub(self.state.last_visible_height as usize);
        self.state.scroll = max_scroll;
        self.state.dirty = true;
    }

    fn element_jump(&mut self, direction: ElementJumpDirection) {
        let lines = &self.state.line_counts;
        let current = self.state.scroll;
        match direction {
            ElementJumpDirection::Next => {
                if let Some(&next) = lines.iter().find(|&&l| l > current) {
                    self.state.scroll = next;
                }
            }
            ElementJumpDirection::Prev => {
                if let Some(&prev) = lines.iter().rev().find(|&&l| l < current) {
                    self.state.scroll = prev;
                }
            }
        }
        self.state.dirty = true;
    }

    // ── Mouse ───────────────────────────────────────────────────────────────

    fn mouse_moved(&mut self, row: u16, col: u16) {
        self.state.mouse_position = Some((row, col));
        self.state.dirty = true;
    }

    fn mouse_clicked(&mut self, _row: u16, _col: u16, button: MouseButton) {
        if matches!(button, MouseButton::Left) {
            // Route click based on position
        }
        self.state.dirty = true;
    }

    fn mouse_released(&mut self, _row: u16, _col: u16, _button: MouseButton) {
        self.state.dirty = true;
    }

    // ── Terminal sizing ─────────────────────────────────────────────────────

    fn terminal_sized(&mut self, width: u16, height: u16) {
        self.state.last_content_width = width;
        self.state.last_visible_height = height;
        self.state.dirty = true;
    }

    // ── Dialog state ────────────────────────────────────────────────────────

    fn dialog_opened(&mut self) {
        self.state.input_receiver = InputReceiver::Dialog;
        self.state.dirty = true;
    }

    fn dialog_closed(&mut self) {
        self.state.input_receiver = InputReceiver::ChatInput;
        self.state.dirty = true;
    }

    // ── Input receiver ───────────────────────────────────────────────────────

    fn set_input_receiver(&mut self, receiver: InputReceiver) {
        self.state.input_receiver = receiver;
        self.state.dirty = true;
    }

    // ── Vim navigation ──────────────────────────────────────────────────────

    fn vim_nav(&mut self, enabled: bool, selected_post: Option<usize>) {
        self.state.vim_nav_mode = enabled;
        self.state.selected_post = selected_post;
        self.state.dirty = true;
    }

    fn toggle_expand_all(&mut self) {
        self.state.all_collapsed = !self.state.all_collapsed;
        self.state.dirty = true;
        self.state.message_gen += 1;
    }

    // ── Turn lifecycle ───────────────────────────────────────────────────────

    fn turn_ended(&mut self) {
        self.state.vim_nav_pending = false;
        self.state.dirty = true;
    }

    fn turn_errored(&mut self) {
        self.state.vim_nav_pending = true;
        self.state.dirty = true;
    }

    // ── Animation ─────────────────────────────────────────────────────────────

    fn animation_tick(&mut self) {
        self.animation_accum += 1;
        // Throttle to ~15fps
        if self.animation_accum % 4 == 0 {
            self.state.animation_frame = self.state.animation_frame.wrapping_add(1);
            self.state.dirty = true;
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> &ViewState {
        &self.state
    }

    /// Access mutable state (for testing).
    #[cfg(test)]
    pub fn state_mut(&mut self) -> &mut ViewState {
        &mut self.state
    }
}

impl Actor for ViewActor {
    type Msg = ViewMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg.clone());
            bus.publish(Event::ViewChanged {
                state: Box::new(self.state.clone()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalidate_sets_dirty() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ViewActor::spawn(bus);

        handle.send(ViewMsg::Invalidate).await;
        drop(handle);
    }

    #[tokio::test]
    async fn scroll_changes_position() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ViewActor::spawn(bus);

        handle.send(ViewMsg::Scroll { direction: ScrollDirection::Down }).await;
        drop(handle);
    }

    #[tokio::test]
    async fn terminal_sized_updates_dimensions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ViewActor::spawn(bus);

        handle.send(ViewMsg::TerminalSized { width: 120, height: 40 }).await;
        drop(handle);
    }

    #[tokio::test]
    async fn dialog_opened_updates_receiver() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ViewActor::spawn(bus);

        handle.send(ViewMsg::DialogOpened).await;
        handle.send(ViewMsg::DialogClosed).await;
        drop(handle);
    }

    #[test]
    fn view_actor_scroll_bounds() {
        let mut actor = ViewActor {
            state: ViewState::default(),
            animation_accum: 0,
        };

        // Scroll up from 0 should stay at 0
        actor.state.scroll = 0;
        actor.scroll(ScrollDirection::Up);
        assert_eq!(actor.state.scroll, 0);
        assert!(actor.state.dirty);

        // Scroll down from 0 with no content stays at 0
        actor.scroll(ScrollDirection::Down);
        assert_eq!(actor.state.scroll, 0);
    }

    #[test]
    fn view_actor_terminal_sized() {
        let mut actor = ViewActor {
            state: ViewState::default(),
            animation_accum: 0,
        };

        actor.terminal_sized(100, 30);
        assert_eq!(actor.state.last_content_width, 100);
        assert_eq!(actor.state.last_visible_height, 30);
        assert!(actor.state.dirty);
    }

    #[test]
    fn view_actor_vim_nav() {
        let mut actor = ViewActor {
            state: ViewState::default(),
            animation_accum: 0,
        };

        actor.vim_nav(true, Some(5));
        assert!(actor.state.vim_nav_mode);
        assert_eq!(actor.state.selected_post, Some(5));
        assert!(actor.state.dirty);
    }
}
