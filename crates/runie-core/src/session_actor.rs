//! SessionActor — subscribes to the event bus, filters durable events,
//! and appends them to a JSONL session file. Also maintains a session
//! metadata index for browsing.
//!
//! Unlike other actors, this is a pure event-bus subscriber with no incoming
//! messages. A plain `tokio::spawn` subscriber loop replaces the `Actor` trait
//! machinery (no unit channel needed).

use crate::bus::EventBus;
use crate::event::DurableCoreEvent;
use crate::session_replay::durable_to_event;
use crate::session_store::SessionStore;
use crate::Event;

/// Session persistence actor state.
pub struct SessionActor {
    session_id: String,
    display_name: String,
    store: SessionStore,
    message_count: usize,
    summary: Option<String>,
    summary_buffer: String,
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
            summary_buffer: String::new(),
            started_at: now,
        }
    }

    /// Spawn the subscriber loop. Call with `tokio::spawn`.
    pub fn run_loop(self, bus: EventBus<Event>) -> impl std::future::Future<Output = ()> + Send + 'static {
        async move {
            self.replay_existing_events(&bus).await;
            self.subscriber_loop(bus).await;
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

    /// Generate a simple summary from the first 500 characters of session
    /// message content (no LLM call needed).
    fn generate_summary(&self) -> String {
        let chars: Vec<char> = self.summary_buffer.chars().take(500).collect();
        let truncated: String = chars.into_iter().collect();
        if truncated.len() < self.summary_buffer.len() {
            format!("{}…", truncated)
        } else {
            truncated
        }
    }

    fn append_to_summary(&mut self, msg: &str) {
        if self.summary_buffer.len() < 500 {
            self.summary_buffer.push_str(msg);
        }
    }

    async fn replay_existing_events(&self, bus: &EventBus<Event>) {
        let events = match self.load_events().await {
            Ok(events) => events,
            Err(_) => return,
        };
        for event in events {
            if let Some(evt) = durable_to_event(&event) {
                bus.publish(evt);
            }
        }
    }

    async fn subscriber_loop(mut self, bus: EventBus<Event>) {
        let mut sub = bus.subscribe();
        loop {
            match sub.recv().await {
                Ok(event) => {
                    if let Some(durable) = event.to_durable() {
                        if let Err(e) = self.persist(&durable).await {
                            eprintln!("SessionActor: failed to persist event: {}", e);
                            continue;
                        }

                        if let DurableCoreEvent::MessageSent { content: msg, .. } = &durable {
                            self.message_count += 1;
                            self.append_to_summary(msg);
                            self.summary = Some(self.generate_summary());
                        }

                        if let DurableCoreEvent::SessionRenamed { name } = &durable {
                            self.display_name = name.clone();
                        }

                        self.update_index_async().await;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    self.update_index_async().await;
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("SessionActor: lagged by {} events", n);
                }
            }
        }
    }

    async fn persist(&self, durable: &DurableCoreEvent) -> anyhow::Result<()> {
        let store = self.store.clone();
        let session_id = self.session_id.clone();
        let event = durable.clone();
        tokio::task::spawn_blocking(move || store.append(&session_id, &event))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))?
    }

    async fn load_events(&self) -> anyhow::Result<Vec<DurableCoreEvent>> {
        let store = self.store.clone();
        let session_id = self.session_id.clone();
        tokio::task::spawn_blocking(move || store.load_events(&session_id))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))?
    }

    async fn update_index_async(&self) {
        let store = self.store.clone();
        let meta = self.build_meta();
        if let Err(e) = tokio::task::spawn_blocking(move || store.update_index(&meta))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {}", e))
            .and_then(|r| r)
        {
            eprintln!("SessionActor: failed to update index: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AgentEvent, DurableCoreEvent, InputEvent, ScrollEvent};
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

        for event in &events {
            if let Some(durable) = event.to_durable() {
                h.store.append(session_id, &durable).unwrap();
            }
        }

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

    fn spawn_replay_actor(h: &TestHarness, session_id: &str) -> tokio::task::JoinHandle<()> {
        let actor = SessionActor::new(session_id.into(), "Replay".into(), h.store.clone());
        tokio::spawn(actor.run_loop(h.bus.clone()))
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

    #[tokio::test]
    async fn session_actor_replays_available_to_late_subscriber() {
        let h = make_harness();
        let session_id = "replay_late_test";
        h.store
            .append_batch(session_id, &replay_events_fixture())
            .unwrap();

        let handle = spawn_replay_actor(&h, session_id);
        let mut sub = h.bus.subscribe_with_replay();
        let collected = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            collect_replayed_events(&mut sub, 3),
        )
        .await
        .expect("timed out waiting for replay");
        handle.abort();

        let mut state = crate::model::AppState::default();
        for event in collected {
            state.update(event);
        }
        assert_replayed_state(&state);
    }

    #[tokio::test]
    async fn session_actor_replays_after_subscriber_ready() {
        let h = make_harness();
        let session_id = "replay_order_test";
        h.store
            .append_batch(session_id, &replay_events_fixture())
            .unwrap();

        let mut sub = h.bus.subscribe();
        let handle = spawn_replay_actor(&h, session_id);
        let collected = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            collect_replayed_events(&mut sub, 3),
        )
        .await
        .expect("timed out waiting for replay");
        handle.abort();

        let mut state = crate::model::AppState::default();
        for event in collected {
            state.update(event);
        }
        assert_replayed_state(&state);
    }
}
