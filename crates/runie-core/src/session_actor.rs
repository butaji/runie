//! SessionActor — subscribes to the event bus, filters durable events,
//! and appends them to a JSONL session file. Also maintains a session
//! metadata index for browsing.

use crate::actor::Actor;
use crate::bus::EventBus;
use crate::event::DurableCoreEvent;
use crate::session_store::{SessionMeta, SessionStore};
use crate::Event;
use tokio::sync::mpsc;

/// Actor that persists durable events to a JSONL session file.
pub struct SessionActor {
    session_id: String,
    display_name: String,
    store: SessionStore,
    message_count: usize,
    summary: String,
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
            summary: String::new(),
            started_at: now,
        }
    }

    fn build_meta(&self) -> SessionMeta {
        SessionMeta {
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

    async fn run(
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
                            self.summary = self.generate_summary(&events);
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

        let actor = SessionActor::new(session_id.to_string(), "test".into(), h.store.clone());
        let (_tx, rx) = mpsc::channel(64);
        tokio::spawn(actor.run(rx, h.bus.clone()));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        h.bus.publish(Event::Agent(AgentEvent::AgentResponse {
            id: "resp.1".into(),
            content: "hello".into(),
        }));
        h.bus.publish(Event::Input(InputEvent::Input(InputEvent::Input('x'))));
        h.bus.publish(Event::Agent(AgentEvent::AgentToolStart {
            id: "tool.1".into(),
            name: "bash".into(),
        }));
        h.bus.publish(Event::Input(InputEvent::Input(InputEvent::Submit)));
        h.bus.publish(Event::Agent(AgentEvent::AgentToolEnd {
            duration_secs: 1.0,
            output: "done".into(),
        }));
        h.bus.publish(Event::Scroll(ScrollEvent::ScrollUp));

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let events = h.store.load_events(session_id).unwrap();
        assert_eq!(events.len(), 3, "only durable events should be persisted");

        for event in &events {
            match event {
                DurableCoreEvent::MessageSent { .. }
                | DurableCoreEvent::ToolCalled { .. }
                | DurableCoreEvent::ToolResult { .. } => {}
                _ => panic!("unexpected durable event variant: {:?}", event),
            }
        }
    }
}
