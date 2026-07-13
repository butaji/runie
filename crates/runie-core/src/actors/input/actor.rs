//! `InputActor` — owns the authoritative `InputState`.
//!
//! This actor uses the ractor framework for actor supervision and message handling.

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::instrument;

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::InputState;

use super::messages::InputMsg;

/// Handle type for InputActor using ractor.
pub type RactorInputHandle = ActorRef<InputMsg>;

/// InputActor's ractor State — holds the authoritative input state.
///
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct InputActorState {
    /// The authoritative input state.
    input: InputState,
    /// Bridge to the event bus for publishing facts.
    bus: EventBus<Event>,
}

impl InputActorState {
    /// Publish an InputChanged event with the current input state.
    fn publish_input_changed(&self) {
        let state = self.input.clone();
        self.bus.publish(Event::InputChanged {
            state: Box::new(state),
        });
    }
}

/// Ractor-based InputActor.
///
/// Text editing, cursor navigation, history, and undo/redo are all mutations
/// that live here. The actor processes `InputMsg` messages and emits
/// `InputChanged` facts when state changes.
pub struct InputActor;

#[async_trait]
impl Actor for InputActor {
    type Msg = InputMsg;
    type State = InputActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        bus: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(InputActorState {
            input: InputState::default(),
            bus,
        })
    }

    #[instrument(name = "input_actor", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Detect quit commands before Submit clears the authoritative input,
        // so the app can exit immediately without waiting for UiActor projection.
        let is_quit_submit = matches!(&msg, InputMsg::Submit { .. })
            && crate::update::input::is_quit_command(state.input.input());

        InputMsg::apply_to(&msg, &mut state.input);
        // Always emit InputChanged: UiActor uses this as the single source of
        // truth for input state, enabling autocomplete trigger detection and
        // clean state synchronization without dual updates.
        state.publish_input_changed();

        if is_quit_submit {
            state.bus.publish(Event::Quit);
        }
        Ok(())
    }
}

impl InputActor {
    /// Spawn an `InputActor` on the given event bus using ractor.
    ///
    /// Returns a `Result` to allow callers to handle spawn failures gracefully.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> anyhow::Result<(
        RactorInputHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        let actor = Self;
        let (handle, join, cell) = spawn_ractor(None, actor, bus)
            .await
            .map_err(|e| anyhow::anyhow!("InputActor spawn failed: {}", e))?;
        Ok((handle, cell, join))
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

        let (handle, _cell, _) = InputActor::spawn(bus.clone()).await.unwrap();
        let _ = handle.send_message(InputMsg::InsertChar('h'));

        // Wait for first InputChanged event
        let found_h = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "h"),
        )
        .await;
        assert!(found_h, "Expected InputChanged with 'h'");

        let _ = handle.send_message(InputMsg::InsertChar('i'));

        // Wait for second InputChanged event
        let found_hi = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "hi"),
        )
        .await;
        assert!(found_hi, "Expected InputChanged with 'hi'");

        drop(handle);
    }

    #[tokio::test]
    async fn history_prev_cycles() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell, _) = InputActor::spawn(bus).await.unwrap();

        let _ = handle.send_message(InputMsg::HistoryLoaded {
            entries: vec!["first".into(), "second".into()],
        });
        let _ = handle.send_message(InputMsg::HistoryPrev);
        drop(handle);
    }

    /// Pasted multi-line text must flatten line breaks to spaces, never
    /// delete them: "a\nb" must become "a b", not "ab". Regression: the
    /// actor's Paste handler stripped newlines while AppState::paste
    /// (the other entry point) flattened them to spaces.
    #[tokio::test]
    async fn paste_flattens_newlines_to_spaces() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _cell, _) = InputActor::spawn(bus.clone()).await.unwrap();
        let _ = handle.send_message(InputMsg::Paste("first\nsecond\r\nthird".into()));

        let found = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "first second third"),
        )
        .await;
        assert!(
            found,
            "pasted newlines must become spaces: expected 'first second third'"
        );

        drop(handle);
    }

    #[tokio::test]
    async fn clear_resets_everything() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell, _) = InputActor::spawn(bus).await.unwrap();

        let _ = handle.send_message(InputMsg::InsertChar('t'));
        let _ = handle.send_message(InputMsg::InsertChar('e'));
        let _ = handle.send_message(InputMsg::InsertChar('s'));
        let _ = handle.send_message(InputMsg::Clear);
        drop(handle);
    }

    /// Regression: submitting a message must append it to the input history so
    /// HistoryPrev (Up arrow) can recall it in the same session. Previously
    /// only `HistoryLoaded` (disk load at startup) populated the history, so
    /// messages sent in the current session were not recallable.
    #[tokio::test]
    async fn submit_appends_to_history() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell, _) = InputActor::spawn(bus.clone()).await.unwrap();

        // Type and submit a message.
        for c in "hello".chars() {
            let _ = handle.send_message(InputMsg::InsertChar(c));
        }
        let _ = handle.send_message(InputMsg::Submit {
            content: "hello".into(),
        });

        // Wait for the submit's InputChanged (cleared input).
        let cleared = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input.is_empty()),
        )
        .await;
        assert!(cleared, "Expected cleared input after submit");

        // Up arrow must recall the just-submitted message.
        let _ = handle.send_message(InputMsg::HistoryPrev);
        let recalled = wait_for_event(
            &mut sub,
            |e| matches!(e, Event::InputChanged { state } if state.input == "hello"),
        )
        .await;
        assert!(
            recalled,
            "HistoryPrev after submit should recall 'hello' in the same session"
        );

        drop(handle);
    }
}
