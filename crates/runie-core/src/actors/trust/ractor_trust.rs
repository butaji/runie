//! Ractor-based `TrustActor` implementation.
//!
//! This module provides a ractor-based implementation of the TrustActor,
//! following the same pattern as the InputActor migration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::trust::TrustDecision;

use super::messages::TrustMsg;

/// Ractor handle type for TrustActor.
#[derive(Clone, Debug)]
pub struct RactorTrustHandle {
    inner: RactorHandle<TrustMsg>,
}

impl RactorTrustHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<TrustMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor.
    pub async fn send(&self, msg: TrustMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget).
    pub fn try_send(&self, msg: TrustMsg) {
        let _ = self.inner.try_send(msg);
    }
}

impl From<RactorHandle<TrustMsg>> for RactorTrustHandle {
    fn from(handle: RactorHandle<TrustMsg>) -> Self {
        Self::new(handle)
    }
}

/// Ractor-based TrustActor.
///
/// Owns trust decisions and derives the read-only flag.
pub struct RactorTrustActor {
    /// Trust decisions keyed by project path.
    decisions: Mutex<HashMap<PathBuf, TrustDecision>>,
    /// Current read-only flag state.
    read_only: Mutex<bool>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl Default for RactorTrustActor {
    fn default() -> Self {
        Self {
            decisions: Mutex::new(HashMap::new()),
            read_only: Mutex::new(false),
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
        }
    }
}

#[async_trait]
impl Actor for RactorTrustActor {
    type Msg = TrustMsg;
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
        msg: TrustMsg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            TrustMsg::LoadTrust { decisions } => {
                *self.decisions.lock().unwrap() = decisions;
            }
            TrustMsg::SetTrust { path, decision } => {
                self.decisions.lock().unwrap().insert(path.clone(), decision);
                self.bus_bridge.publish(Event::TrustChanged { path, decision });
            }
            TrustMsg::InitReadOnly { path } => {
                let new_read_only = !matches!(
                    self.decisions.lock().unwrap().get(&path),
                    Some(TrustDecision::Trusted) | None
                );
                let mut read_only = self.read_only.lock().unwrap();
                if new_read_only != *read_only {
                    *read_only = new_read_only;
                    self.bus_bridge
                        .publish(Event::ReadOnlyChanged { enabled: new_read_only });
                }
            }
        }
        Ok(())
    }
}

impl RactorTrustActor {
    /// Spawn a `RactorTrustActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorTrustHandle, ractor::ActorCell) {
        let actor = Self {
            decisions: Mutex::new(HashMap::new()),
            read_only: Mutex::new(false),
            bus_bridge: EventBusBridge::new(bus.clone()),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorTrustHandle::new(handle), cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::TrustDecision;

    #[tokio::test]
    async fn set_trust_emits_trust_changed() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorTrustActor::spawn(bus).await;

        let path = PathBuf::from("/test/project");
        handle
            .send(TrustMsg::SetTrust {
                path: path.clone(),
                decision: TrustDecision::Trusted,
            })
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut found = false;
        for _ in 0..10 {
            if let Ok(e) = sub.try_recv() {
                if matches!(e, Event::TrustChanged { .. }) {
                    found = true;
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(found, "Expected TrustChanged event");
    }

    #[tokio::test]
    async fn init_read_only_emits_when_untrusted() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorTrustActor::spawn(bus).await;

        let path = PathBuf::from("/test/project");
        handle
            .send(TrustMsg::SetTrust {
                path: path.clone(),
                decision: TrustDecision::Untrusted,
            })
            .await;
        handle
            .send(TrustMsg::InitReadOnly { path: path.clone() })
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn load_trust_sets_decisions() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorTrustActor::spawn(bus).await;

        let mut decisions = HashMap::new();
        decisions.insert(PathBuf::from("/proj1"), TrustDecision::Trusted);
        decisions.insert(PathBuf::from("/proj2"), TrustDecision::Untrusted);

        handle
            .send(TrustMsg::LoadTrust { decisions })
            .await;
        drop(handle);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
