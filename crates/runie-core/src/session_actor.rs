//! SessionActor — subscribes to the event bus, filters durable events,
//! and appends them to a JSONL session file. Also maintains a session
//! metadata index for browsing.

use crate::actor::{Actor, ActorFuture};
use crate::bus::EventBus;
use crate::event::DurableCoreEvent;
use crate::session_store::{SessionMeta, SessionStore};
use crate::Event;
use tokio::sync::{mpsc, oneshot};

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

impl Actor for SessionActor {
    type Msg = ();
    type Event = Event;

    fn run(self, _rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) -> ActorFuture {
        Box::pin(self.run_body(_rx, bus))
    }

    async fn run_body(
        mut self,
        _rx: mpsc::Receiver<Self::Msg>,
        bus: EventBus<Self::Event>,
    ) {
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
                            let events = self.store.load_events(&self.session_id).unwrap_or_default();
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
        TestHarness { bus, store, _dir: dir }
    }

    #[tokio::test]
    async fn event_bus_filters_durable_events() {
        let h = make_harness();
        let session_id = "filter_test";

        // Directly test filtering: publish events and check store directly
        let events = vec![
            Event::Agent(AgentEvent::Response { id: "resp.1".into(), content: "hello".into() }),
            Event::Input(InputEvent::Input('x')),
            Event::Agent(AgentEvent::ToolStart { id: "tool.1".into(), name: "bash".into() }),
            Event::Input(InputEvent::Submit),
            Event::Agent(AgentEvent::ToolEnd { duration_secs: 1.0, output: "done".into() }),
            Event::Scroll(ScrollEvent::Up),
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
}
