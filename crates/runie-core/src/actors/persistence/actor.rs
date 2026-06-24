//! `PersistenceActor` — owns trust and input-history file I/O.
//!
//! The actor loads `trust.json` and `history.jsonl` on startup and publishes
//! `TrustLoaded` / `HistoryLoaded`. It handles `SetTrust` and `AppendHistory`
//! messages by writing to disk in `spawn_blocking` and publishing follow-up
//! events.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::actor::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::trust::{TrustDecision, TrustManager};

use super::messages::{PersistenceActorHandle, PersistenceMsg};

/// Actor that owns trust + history persistence.
pub struct PersistenceActor {
    bus: EventBus<Event>,
    trust: TrustManager,
}

impl PersistenceActor {
    /// Spawn a `PersistenceActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (PersistenceActorHandle, ActorHandle) {
        let actor = Self {
            bus: bus.clone(),
            trust: TrustManager::default(),
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (PersistenceActorHandle::new(tx), handle)
    }
}

impl Actor for PersistenceActor {
    type Msg = PersistenceMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        self.load_all().await;
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl PersistenceActor {
    async fn load_all(&self) {
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        let decisions = trust.decisions();
        self.bus.publish(Event::TrustLoaded { decisions });

        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        self.bus.publish(Event::HistoryLoaded { entries });
    }

    async fn handle_msg(&mut self, msg: PersistenceMsg) {
        match msg {
            PersistenceMsg::SetTrust { path, decision } => self.set_trust(path, decision).await,
            PersistenceMsg::AppendHistory { entry } => self.append_history(entry).await,
        }
    }

    async fn set_trust(&mut self, path: PathBuf, decision: TrustDecision) {
        self.trust.set(&path, decision);
        let trust = self.trust.clone();
        let _ = tokio::task::spawn_blocking(move || trust.save()).await;
        self.bus.publish(Event::TrustChanged { path, decision });
    }

    async fn append_history(&self, entry: String) {
        let _ = tokio::task::spawn_blocking({
            let entry = entry.clone();
            move || crate::input_history::append_history(&entry)
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::Duration;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn actor_loads_and_emits_trust_and_history() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("cfg");
        let data = tmp.path().join("data");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::env::set_var("RUNIE_TEST_CONFIG_DIR", &cfg);
        std::env::set_var("RUNIE_TEST_DATA_DIR", &data);

        let bus = EventBus::<Event>::new(4);
        let mut sub = bus.subscribe_with_replay();
        let (handle, _actor_handle) = PersistenceActor::spawn(bus);

        let mut saw_trust = false;
        let mut saw_history = false;
        for _ in 0..60 {
            if saw_trust && saw_history {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            while let Some(Ok(evt)) = sub.try_recv() {
                match evt {
                    Event::TrustLoaded { .. } => saw_trust = true,
                    Event::HistoryLoaded { .. } => saw_history = true,
                    // intentionally ignored: other events fall through
                    _ => {}
                }
            }
        }
        assert!(saw_trust);
        assert!(saw_history);

        handle.set_trust(PathBuf::from("/tmp/project"), TrustDecision::Trusted).await;

        let mut saw_changed = false;
        for _ in 0..60 {
            if saw_changed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            while let Some(Ok(evt)) = sub.try_recv() {
                if matches!(evt, Event::TrustChanged { .. }) {
                    saw_changed = true;
                }
            }
        }
        assert!(saw_changed);

        std::env::remove_var("RUNIE_TEST_CONFIG_DIR");
        std::env::remove_var("RUNIE_TEST_DATA_DIR");
    }
}
