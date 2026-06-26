//! `TrustActor` — owns trust decisions and the derived read-only flag.

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::trust::TrustDecision;

use super::messages::{TrustActorHandle, TrustMsg};

/// Actor that owns trust decisions and derives the read-only flag.
///
/// Trust decisions are stored in-memory and can be loaded from/committed to
/// persistence (via SessionActor). The actor emits:
/// - `Event::TrustChanged` when a decision changes
/// - `Event::ReadOnlyChanged` when the read-only flag changes
#[derive(Default)]
pub struct TrustActor {
    /// Trust decisions keyed by project path.
    decisions: HashMap<PathBuf, TrustDecision>,
    /// Current read-only flag state.
    read_only: bool,
}

impl TrustActor {
    /// Spawn a `TrustActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (TrustActorHandle, ActorHandle) {
        let actor = Self::default();
        let (tx, handle) = spawn_actor(actor, bus);
        (TrustActorHandle::new(tx), handle)
    }

    /// Get the current read-only flag.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// Get a copy of all trust decisions.
    pub fn decisions(&self) -> HashMap<PathBuf, TrustDecision> {
        self.decisions.clone()
    }

    /// Dispatch an incoming message to the appropriate handler.
    fn handle_msg(&mut self, msg: TrustMsg, bus: &EventBus<Event>) {
        match msg {
            TrustMsg::LoadTrust { decisions } => {
                self.decisions = decisions;
            }
            TrustMsg::SetTrust { path, decision } => {
                self.decisions.insert(path.clone(), decision);
                bus.publish(Event::TrustChanged { path, decision });
            }
            TrustMsg::InitReadOnly { path } => {
                let new_read_only = !matches!(
                    self.decisions.get(&path),
                    Some(TrustDecision::Trusted) | None
                );
                if new_read_only != self.read_only {
                    self.read_only = new_read_only;
                    bus.publish(Event::ReadOnlyChanged { enabled: new_read_only });
                }
            }
        }
    }
}

impl Actor for TrustActor {
    type Msg = TrustMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg, &bus);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::TrustDecision;

    async fn drain_events<E: Clone + Send + 'static>(
        sub: &mut tokio::sync::broadcast::Receiver<E>,
        count: usize,
    ) -> Vec<E> {
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            match sub.recv().await {
                Ok(e) => events.push(e),
                Err(_) => break,
            }
        }
        events
    }

    #[tokio::test]
    async fn set_trust_emits_trust_changed() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _actor) = TrustActor::spawn(bus);

        let path = PathBuf::from("/test/project");
        handle
            .send(TrustMsg::SetTrust {
                path: path.clone(),
                decision: TrustDecision::Trusted,
            })
            .await;
        drop(handle);

        let events = drain_events(&mut sub, 1).await;
        assert!(!events.is_empty());
        if let Event::TrustChanged { path: p, decision: d } = &events[0] {
            assert_eq!(p, &path);
            assert_eq!(d, &TrustDecision::Trusted);
        } else {
            panic!("Expected TrustChanged event");
        }
    }

    #[tokio::test]
    async fn init_read_only_emits_read_only_changed_when_untrusted() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _actor) = TrustActor::spawn(bus);

        let path = PathBuf::from("/test/project");
        // First set the path as untrusted
        handle
            .send(TrustMsg::SetTrust {
                path: path.clone(),
                decision: TrustDecision::Untrusted,
            })
            .await;
        // Then init read-only (should emit ReadOnlyChanged since untrusted)
        handle
            .send(TrustMsg::InitReadOnly { path: path.clone() })
            .await;
        drop(handle);

        let events = drain_events(&mut sub, 3).await; // TrustChanged + ReadOnlyChanged
        assert!(events.len() >= 2);

        // Find the ReadOnlyChanged event
        let read_only_changed = events.iter().find(|e| matches!(e, Event::ReadOnlyChanged { .. }));
        assert!(
            read_only_changed.is_some(),
            "Expected ReadOnlyChanged event"
        );
        if let Event::ReadOnlyChanged { enabled } = read_only_changed.unwrap() {
            assert!(enabled, "Untrusted project should enable read-only");
        }
    }

    #[tokio::test]
    async fn init_read_only_does_not_emit_when_trusted() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        let (handle, _actor) = TrustActor::spawn(bus);

        let path = PathBuf::from("/test/project");
        // Set as trusted
        handle
            .send(TrustMsg::SetTrust {
                path: path.clone(),
                decision: TrustDecision::Trusted,
            })
            .await;
        // Init read-only (should NOT emit ReadOnlyChanged since trusted)
        handle
            .send(TrustMsg::InitReadOnly { path: path.clone() })
            .await;
        drop(handle);

        let events = drain_events(&mut sub, 3).await;
        // Only TrustChanged, no ReadOnlyChanged for trusted project
        let read_only_changed_count = events
            .iter()
            .filter(|e| matches!(e, Event::ReadOnlyChanged { .. }))
            .count();
        assert_eq!(read_only_changed_count, 0, "Trusted project should not enable read-only");
    }

    #[tokio::test]
    async fn load_trust_sets_decisions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = TrustActor::spawn(bus);

        let mut decisions = HashMap::new();
        decisions.insert(PathBuf::from("/proj1"), TrustDecision::Trusted);
        decisions.insert(PathBuf::from("/proj2"), TrustDecision::Untrusted);

        handle
            .send(TrustMsg::LoadTrust { decisions: decisions.clone() })
            .await;
        drop(handle);
    }
}
