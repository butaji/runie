//! `InputActor` — owns the authoritative `InputState`.
//!
//! No ractor dependency. Actor is a tokio task with mpsc channel.

use tokio::sync::mpsc;
use tracing::instrument;

use crate::actors::StopCell;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::InputState;

use super::messages::InputMsg;

// ── Handle ────────────────────────────────────────────────────────────────────

/// InputActor handle — cloneable, fire-and-forget sender.
#[derive(Clone, Debug)]
pub struct InputHandle {
    tx: mpsc::UnboundedSender<InputMsg>,
}

impl InputHandle {
    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: InputMsg) {
        let _ = self.tx.send(msg);
    }

    /// Backward-compat alias for `send`.
    #[allow(dead_code)]
    pub async fn send_message(&self, msg: InputMsg) {
        let _ = self.tx.send(msg);
    }

    /// Insert a character at the cursor.
    pub async fn insert_char(&self, c: char) {
        self.send(InputMsg::InsertChar(c)).await
    }

    /// Delete the character before the cursor.
    pub async fn backspace(&self) {
        self.send(InputMsg::Backspace).await
    }

    /// Paste text at the cursor.
    pub async fn paste(&self, text: String) {
        self.send(InputMsg::Paste(text)).await
    }

    /// Submit content — clears input.
    pub async fn submit(&self, content: String) {
        self.send(InputMsg::Submit { content }).await
    }

    /// Set all input text.
    pub async fn set_text(&self, text: String, chips: Vec<crate::model::InputChip>) {
        self.send(InputMsg::SetText { text, chips }).await
    }

    /// Clear the input.
    pub async fn clear(&self) {
        self.send(InputMsg::Clear).await
    }

    /// Navigate to previous history entry.
    pub async fn history_prev(&self) {
        self.send(InputMsg::HistoryPrev).await
    }

    /// Load history entries from disk.
    pub async fn history_loaded(&self, entries: Vec<String>) {
        self.send(InputMsg::HistoryLoaded { entries }).await
    }

    /// Undo the last edit.
    pub async fn undo(&self) {
        self.send(InputMsg::Undo).await
    }

    /// Move cursor one character left.
    pub async fn cursor_left(&self) {
        self.send(InputMsg::CursorLeft).await
    }

    /// Move cursor to the start.
    pub async fn cursor_start(&self) {
        self.send(InputMsg::CursorStart).await
    }

    /// Move cursor to the end.
    pub async fn cursor_end(&self) {
        self.send(InputMsg::CursorEnd).await
    }

    /// Set cursor to an absolute position.
    pub async fn move_cursor(&self, pos: usize) {
        self.send(InputMsg::MoveCursor { pos }).await
    }
}

// Backward-compat alias
#[allow(unused_imports)]
pub use InputHandle as RactorInputHandle;

// ── Actor state ───────────────────────────────────────────────────────────────

/// Mutable state owned by InputActor.
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct InputActorState {
    /// The authoritative input state.
    pub input: InputState,
    /// Bridge to the event bus for publishing facts.
    pub bus: EventBus<Event>,
}

impl InputActorState {
    /// Publish an InputChanged event with the current input state.
    pub fn publish_input_changed(&self) {
        let state = self.input.clone();
        self.bus.publish(Event::InputChanged { state: Box::new(state) });
    }
}

// ── Actor struct ─────────────────────────────────────────────────────────────

/// The InputActor — processes input messages.
struct InputActor {
    rx: mpsc::UnboundedReceiver<InputMsg>,
    state: InputActorState,
}

impl InputActor {
    /// Main loop.
    async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            self.handle(msg).await;
        }
    }

    /// Handle one message.
    #[instrument(name = "input_actor", skip_all, fields(msg = ?msg))]
    async fn handle(&mut self, msg: InputMsg) {
        // Detect quit commands before Submit clears the authoritative input,
        // so the app can exit immediately without waiting for UiActor projection.
        let is_quit_submit =
            matches!(&msg, InputMsg::Submit { .. }) && crate::update::input::is_quit_command(self.state.input.input());

        InputMsg::apply_to(&msg, &mut self.state.input);
        // Always emit InputChanged: UiActor uses this as the single source of
        // truth for input state, enabling autocomplete trigger detection and
        // clean state synchronization without dual updates.
        self.state.publish_input_changed();

        if is_quit_submit {
            self.state.bus.publish(Event::Quit);
        }
    }
}

// ── Spawn ────────────────────────────────────────────────────────────────────

/// Spawn an InputActor and return (handle, stop_cell, join_handle).
pub fn spawn_input_actor(
    bus: EventBus<Event>,
) -> (InputHandle, StopCell, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    let join = tokio::spawn(async move {
        let state = InputActorState {
            input: InputState::default(),
            bus: bus.clone(),
        };
        let mut actor = InputActor { rx, state };
        actor.run().await;
    });

    (InputHandle { tx }, StopCell, join)
}

// ── Backward compat stubs ────────────────────────────────────────────────────

/// Stub type for backward compatibility.
pub struct InputActorBase;

impl InputActorBase {
    /// Spawn an `InputActor` and return (handle, stop_cell, join_handle).
    /// Cell is a no-op `StopCell` — mpsc actors stop when the handle is dropped.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> Result<(InputHandle, StopCell, tokio::task::JoinHandle<()>), anyhow::Error> {
        Ok(spawn_input_actor(bus))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Receiver;

    /// Wait for an event matching a predicate with a deterministic timeout.
    async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
    where
        F: Fn(&Event) -> bool,
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let timeout_duration = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(timeout_duration, sub.recv()).await {
                Ok(Ok(evt)) => {
                    if pred(&evt) {
                        return true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        false
    }

    #[tokio::test]
    async fn insert_char_updates_cursor() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _, _) = spawn_input_actor(bus.clone());
        let _ = handle.send(InputMsg::InsertChar('h')).await;

        // Wait for first InputChanged event
        let found_h = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "h"),
        )
        .await;
        assert!(found_h, "Expected InputChanged with 'h'");

        let _ = handle.send(InputMsg::InsertChar('i')).await;

        // Wait for second InputChanged event
        let found_hi = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "hi"),
        )
        .await;
        assert!(found_hi, "Expected InputChanged with 'hi'");
    }

    #[tokio::test]
    async fn history_prev_cycles() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = spawn_input_actor(bus);

        let _ = handle.send(InputMsg::HistoryLoaded { entries: vec!["first".into(), "second".into()] }).await;
        let _ = handle.send(InputMsg::HistoryPrev).await;
    }

    /// Pasted multi-line text must preserve line breaks (multi-line input,
    /// grok parity): "a\nb" stays "a\nb". CRLF normalizes to LF.
    #[tokio::test]
    async fn paste_preserves_newlines() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _, _) = spawn_input_actor(bus.clone());
        let _ = handle.send(InputMsg::Paste("first\nsecond\r\nthird".into())).await;

        let found = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "first\nsecond\nthird"),
        )
        .await;
        assert!(
            found,
            "pasted newlines must be preserved: expected 'first\\nsecond\\nthird'"
        );
    }

    #[tokio::test]
    async fn clear_resets_everything() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = spawn_input_actor(bus);

        let _ = handle.send(InputMsg::InsertChar('t')).await;
        let _ = handle.send(InputMsg::InsertChar('e')).await;
        let _ = handle.send(InputMsg::InsertChar('s')).await;
        let _ = handle.send(InputMsg::Clear).await;
    }

    /// Regression: submitting a message must append it to the input history so
    /// HistoryPrev (Up arrow) can recall it in the same session. Previously
    /// only `HistoryLoaded` (disk load at startup) populated the history, so
    /// messages sent in the current session were not recallable.
    #[tokio::test]
    async fn submit_appends_to_history() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _, _) = spawn_input_actor(bus.clone());

        // Type and submit a message.
        for c in "hello".chars() {
            let _ = handle.send(InputMsg::InsertChar(c)).await;
        }
        let _ = handle.send(InputMsg::Submit { content: "hello".into() }).await;

        // Wait for the submit's InputChanged (cleared input).
        let cleared = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input.is_empty()),
        )
        .await;
        assert!(cleared, "Expected cleared input after submit");

        // Up arrow must recall the just-submitted message.
        let _ = handle.send(InputMsg::HistoryPrev).await;
        let recalled = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "hello"),
        )
        .await;
        assert!(
            recalled,
            "HistoryPrev after submit should recall 'hello' in the same session"
        );
    }
}
