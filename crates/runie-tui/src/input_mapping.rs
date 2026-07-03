//! Canonical event → `InputMsg` router for input actors.
//!
//! Both the `input_forwarder_task` (raw terminal events) and
//! `UiActor::handle_input_event` (bus-published events) map the same
//! key/command events to `InputMsg` variants.  This module provides the
//! single canonical function so there is one place to maintain the mapping.

use runie_core::actors::{InputMsg, RactorInputHandle};

/// Route a raw input event to `InputActor` via the appropriate `InputMsg`.
///
/// Returns `true` if the event was forwarded to `InputActor`.
/// Returns `false` if the event should be handled by `UiActor` itself
/// (e.g. `Submit`, `Quit`, `ForceQuit`, `Abort`).
///
/// This is the single source of truth for the event → `InputMsg` mapping.
/// Both the forwarder and `UiActor` must use this function.
pub async fn route_to_input_actor(handle: &RactorInputHandle, evt: &runie_core::Event) -> bool {
    use runie_core::Event as E;
    match evt {
        E::Input(c) => {
            handle_input(handle, InputMsg::InsertChar(*c));
            true
        }
        E::Backspace => {
            handle_input(handle, InputMsg::Backspace);
            true
        }
        E::Newline => {
            handle_input(handle, InputMsg::Newline);
            true
        }
        E::DeleteWord => {
            handle_input(handle, InputMsg::DeleteWord);
            true
        }
        E::DeleteToEnd => {
            handle_input(handle, InputMsg::DeleteToEnd);
            true
        }
        E::DeleteToStart => {
            handle_input(handle, InputMsg::DeleteToStart);
            true
        }
        E::CursorLeft => {
            handle_input(handle, InputMsg::CursorLeft);
            true
        }
        E::CursorRight => {
            handle_input(handle, InputMsg::CursorRight);
            true
        }
        E::CursorStart => {
            handle_input(handle, InputMsg::CursorStart);
            true
        }
        E::CursorEnd => {
            handle_input(handle, InputMsg::CursorEnd);
            true
        }
        E::CursorWordLeft => {
            handle_input(handle, InputMsg::CursorWordLeft);
            true
        }
        E::CursorWordRight => {
            handle_input(handle, InputMsg::CursorWordRight);
            true
        }
        E::HistoryPrev => {
            handle_input(handle, InputMsg::HistoryPrev);
            true
        }
        E::HistoryNext => {
            handle_input(handle, InputMsg::HistoryNext);
            true
        }
        E::Undo => {
            handle_input(handle, InputMsg::Undo);
            true
        }
        E::Redo => {
            handle_input(handle, InputMsg::Redo);
            true
        }
        E::Paste(s) => {
            handle_input(handle, InputMsg::Paste(s.clone()));
            true
        }
        E::PasteImage => {
            handle_input(handle, InputMsg::PasteImage);
            true
        }
        // These are not routed to InputActor; UiActor handles them.
        E::Submit | E::Quit | E::ForceQuit | E::Abort => false,
        _ => false,
    }
}

/// Fire-and-forget send to InputActor (synchronous, no await).
fn handle_input(handle: &RactorInputHandle, msg: InputMsg) {
    let _ = handle.send_message(msg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::event::Event;

    /// Layer 1: verify the canonical mapping returns the correct bool for each event.
    #[tokio::test]
    async fn route_to_input_actor_returns_true_for_routable_events() {
        // Use the test leader to get a real InputActor handle.
        let leader = runie_core::actors::leader::test_leader_handle().await;

        // Verify that all known-routable events return true.
        let events = [
            Event::Input('x'),
            Event::Backspace,
            Event::Newline,
            Event::DeleteWord,
            Event::DeleteToEnd,
            Event::DeleteToStart,
            Event::CursorLeft,
            Event::CursorRight,
            Event::CursorStart,
            Event::CursorEnd,
            Event::CursorWordLeft,
            Event::CursorWordRight,
            Event::HistoryPrev,
            Event::HistoryNext,
            Event::Undo,
            Event::Redo,
            Event::Paste("test".into()),
            Event::PasteImage,
        ];

        for evt in &events {
            let result = route_to_input_actor(&leader.input, evt).await;
            assert!(result, "Event {:?} should be routed to InputActor", evt);
        }

        // Verify that non-routable events return false.
        let not_routed = [Event::Submit, Event::Quit, Event::ForceQuit, Event::Abort];
        for evt in &not_routed {
            let result = route_to_input_actor(&leader.input, evt).await;
            assert!(
                !result,
                "Event {:?} should NOT be routed to InputActor",
                evt
            );
        }

        leader.shutdown().await;
    }
}
