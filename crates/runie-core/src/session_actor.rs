//! SessionActor — subscribes to the event bus, filters durable events,
//! and appends them to a JSONL session file. Also maintains a session
//! metadata index for browsing.

use crate::actor::{Actor, ActorFuture};
use crate::bus::EventBus;
use crate::event::DurableCoreEvent;
use crate::session_replay::durable_to_event;
use crate::session_store::SessionStore;
use crate::Event;
use tokio::sync::mpsc;

/// Actor that persists durable events to a JSONL session file.
pub struct SessionActor {
    session_id: String,
    display_name: String,
    store: SessionStore,
    message_count: usize,
    summary: Option<String>,
    started_at: f64,
}

impl SessionActor {
    pub fn new(session_id: String, display_name: String, store: SessionStore) -> Self {
        let now = crate::message::now();
        Self {
            session_id,
            display_name,
            store,
            message_count: 0,
            summary: None,
            started_at: now,
        }
    }

    fn build_meta(&self) -> crate::session_index::SessionMetadata {
        crate::session_index::SessionMetadata {
            id: self.session_id.clone(),
            display_name: self.display_name.clone(),
            created_at: self.started_at,
            updated_at: crate::message::now(),
            message_count: self.message_count,
            summary: self.summary.clone(),
            is_starred: false,
            is_system: false,
        }
    }

    fn update_index(&self) {
        if let Err(e) = self.store.update_index(&self.build_meta()) {
            eprintln!("SessionActor: failed to update index: {}", e);
        }
    }

    /// Generate a simple summary from the first 500 characters of session
    /// message content (no LLM call needed).
    fn generate_summary(&self, events: &[DurableCoreEvent]) -> String {
        let mut content = String::new();
        for ev in events {
            if let DurableCoreEvent::MessageSent { content: msg, .. } = ev {
                if content.len() >= 500 {
                    break;
                }
                content.push_str(msg);
            }
        }
        let chars: Vec<char> = content.chars().take(500).collect();
        let truncated: String = chars.into_iter().collect();
        if truncated.len() < content.len() {
            format!("{}…", truncated)
        } else {
            truncated
        }
    }
}

fn replay_existing_events(store: &SessionStore, session_id: &str, bus: &EventBus<Event>) {
    let events = match store.load_events(session_id) {
        Ok(events) => events,
        Err(_) => return,
    };
    for event in events {
        if let Some(evt) = durable_to_event(&event) {
            bus.publish(evt);
        }
    }
}

impl Actor for SessionActor {
    type Msg = ();
    type Event = Event;

    fn run(self, _rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) -> ActorFuture {
        replay_existing_events(&self.store, &self.session_id, &bus);
        Box::pin(self.run_body(_rx, bus))
    }

    async fn run_body(mut self, _rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) {
        let mut sub = bus.subscribe();
        loop {
            match sub.recv().await {
                Ok(event) => {
                    if let Some(durable) = event.to_durable() {
                        if let Err(e) = self.store.append(&self.session_id, &durable) {
                            eprintln!("SessionActor: failed to persist event: {}", e);
                            continue;
                        }

                        if matches!(&durable, DurableCoreEvent::MessageSent { .. }) {
                            self.message_count += 1;
                            let events =
                                self.store.load_events(&self.session_id).unwrap_or_default();
                            self.summary = Some(self.generate_summary(&events));
                        }

                        if let DurableCoreEvent::SessionRenamed { name } = &durable {
                            self.display_name = name.clone();
                        }

                        self.update_index();
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    self.update_index();
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("SessionActor: lagged by {} events", n);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AgentEvent, DurableCoreEvent, InputEvent, ScrollEvent};
    use crate::Event;
    use tempfile::TempDir;

    struct TestHarness {
        bus: EventBus<Event>,
        store: SessionStore,
        _dir: TempDir,
    }

    fn make_harness() -> TestHarness {
        let bus = EventBus::new(64);
        let dir = TempDir::new().unwrap();
        let store = SessionStore::new(dir.path().to_path_buf());
        TestHarness {
            bus,
            store,
            _dir: dir,
        }
    }

    #[tokio::test]
    async fn event_bus_filters_durable_events() {
        let h = make_harness();
        let session_id = "filter_test";

        // Directly test filtering: publish events and check store directly
        let events = vec![
            AgentEvent::Response {
                id: "resp.1".into(),
                content: "hello".into(),
            },
            InputEvent::Input('x'),
            AgentEvent::ToolStart {
                id: "tool.1".into(),
                name: "bash".into(),
                input: serde_json::Value::Null,
            },
            InputEvent::Submit,
            AgentEvent::ToolEnd {
                id: "tool.1".into(),
                duration_secs: 1.0,
                output: "done".into(),
            },
            ScrollEvent::Up,
        ];

        // Manually filter and persist (simulating SessionActor logic)
        for event in &events {
            if let Some(durable) = event.to_durable() {
                h.store.append(session_id, &durable).unwrap();
            }
        }

        // Verify only durable events were persisted
        let persisted = h.store.load_events(session_id).unwrap();
        assert_eq!(persisted.len(), 3);
        for event in &persisted {
            assert!(
                matches!(
                    event,
                    DurableCoreEvent::MessageSent { .. }
                        | DurableCoreEvent::ToolCalled { .. }
                        | DurableCoreEvent::ToolResult { .. }
                ),
                "unexpected durable event variant: {:?}",
                event
            );
        }
    }

    fn replay_events_fixture() -> Vec<DurableCoreEvent> {
        vec![
            DurableCoreEvent::MessageSent {
                id: "m1".into(),
                role: "user".into(),
                content: "Hello".into(),
                timestamp: 1.0,
                provider: String::new(),
            },
            DurableCoreEvent::MessageSent {
                id: "m2".into(),
                role: "assistant".into(),
                content: "Hi!".into(),
                timestamp: 2.0,
                provider: "anthropic".into(),
            },
            DurableCoreEvent::ModelSwitched {
                provider: "anthropic".into(),
                model: "claude-3".into(),
            },
        ]
    }

    fn spawn_replay_actor(h: &TestHarness, session_id: &str) -> crate::actor::ActorHandle {
        let actor = SessionActor::new(session_id.into(), "Replay".into(), h.store.clone());
        let (_tx, handle) = crate::actor::spawn_actor(actor, h.bus.clone());
        handle
    }

    async fn collect_replayed_events(
        sub: &mut crate::bus::ReplayReceiver<Event>,
        count: usize,
    ) -> Vec<Event> {
        let mut collected = Vec::new();
        while collected.len() < count {
            if let Ok(event) = sub.recv().await {
                collected.push(event);
            } else {
                break;
            }
        }
        collected
    }

    fn assert_replayed_state(state: &crate::model::AppState) {
        assert_eq!(state.session.messages.len(), 2);
        assert_eq!(state.session.messages[0].content, "Hello");
        assert_eq!(state.session.messages[1].content, "Hi!");
        assert_eq!(state.config.current_provider, "anthropic");
        assert_eq!(state.config.current_model, "claude-3");
    }

    #[tokio::test]
    async fn session_actor_replays_to_uactor() {
        let h = make_harness();
        let session_id = "replay_test";
        h.store
            .append_batch(session_id, &replay_events_fixture())
            .unwrap();

        let mut sub = h.bus.subscribe_with_replay();
        let handle = spawn_replay_actor(&h, session_id);
        let collected = collect_replayed_events(&mut sub, 3).await;
        handle.abort();

        let mut state = crate::model::AppState::default();
        for event in collected {
            state.update(event);
        }
        assert_replayed_state(&state);
    }
}
