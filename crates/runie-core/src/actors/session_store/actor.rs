//! `SessionStoreActor` — owns named-session IO.
//!
//! Load, save, delete, import, export, and list all go through this actor so
//! domain update handlers stay pure.

use std::path::PathBuf;

use crate::actor::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::session::Session;
use crate::session_index::{SessionIndex, SessionMetadata};
use crate::session_replay::session_to_durable_events;
use crate::session_store::SessionStore;

use super::messages::{SessionStoreActorHandle, SessionStoreMsg};

/// Actor that owns all named-session file IO.
pub struct SessionStoreActor {
    bus: EventBus<Event>,
    store: SessionStore,
}

impl SessionStoreActor {
    /// Spawn a `SessionStoreActor` on the given event bus.
    pub fn spawn(bus: EventBus<Event>) -> (SessionStoreActorHandle, ActorHandle) {
        let store = SessionStore::default_store().unwrap_or_else(|| {
            SessionStore::new(std::env::temp_dir().join("runie_sessions"))
        });
        let actor = Self { bus: bus.clone(), store };
        let (tx, handle) = spawn_actor(actor, bus);
        (SessionStoreActorHandle::new(tx), handle)
    }
}

impl Actor for SessionStoreActor {
    type Msg = SessionStoreMsg;
    type Event = Event;

    async fn run_body(self, mut rx: tokio::sync::mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle(msg).await;
        }
    }
}

impl SessionStoreActor {
    async fn handle(&self, msg: SessionStoreMsg) {
        match msg {
            SessionStoreMsg::Load { name } => self.load(name).await,
            SessionStoreMsg::Save { name, session } => self.save(name, session).await,
            SessionStoreMsg::Delete { name } => self.delete(name).await,
            SessionStoreMsg::Import { path } => self.import(path).await,
            SessionStoreMsg::Export { path, session } => self.export(path, session).await,
            SessionStoreMsg::List => self.list().await,
        }
    }

    async fn load(&self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let events = store.load_events(&name_for_task)?;
            if events.is_empty() {
                return Err(anyhow::anyhow!("not found"));
            }
            let data_dir = store.dir().parent().unwrap_or(store.dir()).to_path_buf();
            let metadata = SessionIndex::load(&data_dir)
                .ok()
                .and_then(|idx| idx.get(&name_for_task).cloned());
            Ok((events, metadata))
        })
        .await;
        match res {
            Ok(Ok((events, metadata))) => {
                self.emit(Event::SessionLoaded {
                    name,
                    events: Box::new(events),
                    metadata: metadata.map(Box::new),
                });
            }
            _ => self.fail("load", format!("Session '{}' not found. Use /sessions to list saved sessions.", name)),
        }
    }

    async fn save(&self, name: String, session: Session) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let events = session_to_durable_events(&session);
        let meta = metadata_from_session(&session, &name);
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.append_batch(&name_for_task, &events)?;
            store.update_index(&meta)?;
            Ok(())
        })
        .await;
        match res {
            Ok(Ok(())) => self.emit(Event::SessionSaved { name }),
            Ok(Err(e)) => self.fail("save", e.to_string()),
            Err(e) => self.fail("save", e.to_string()),
        }
    }

    async fn delete(&self, name: String) {
        let store = self.store.clone();
        let name_for_task = name.clone();
        let res = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            store.delete(&name_for_task)?;
            Ok(())
        })
        .await;
        match res {
            Ok(Ok(())) => self.emit(Event::SessionDeleted { name }),
            Ok(Err(_)) => self.fail(
                "delete",
                format!("Session '{}' not found. Use /sessions to list saved sessions.", name),
            ),
            Err(e) => self.fail("delete", e.to_string()),
        }
    }

    async fn import(&self, path: PathBuf) {
        let path_for_task = path.clone();
        let read_res = tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_for_task)).await;
        let json = match read_res {
            Ok(Ok(json)) => json,
            Ok(Err(e)) => return self.fail("import", e.to_string()),
            Err(e) => return self.fail("import", e.to_string()),
        };
        match serde_json::from_str::<Session>(&json) {
            Ok(session) => self.emit(Event::SessionImported { session: Box::new(session) }),
            Err(e) => self.fail("import", e.to_string()),
        }
    }

    async fn export(&self, path: PathBuf, session: Session) {
        let display = path.to_string_lossy().to_string();
        let json = match serde_json::to_string_pretty(&session) {
            Ok(json) => json,
            Err(e) => return self.fail("export", e.to_string()),
        };
        let res = tokio::task::spawn_blocking(move || std::fs::write(&path, json)).await;
        match res {
            Ok(Ok(())) => self.emit(Event::SessionExported { path: display }),
            Ok(Err(e)) => self.fail("export", e.to_string()),
            Err(e) => self.fail("export", e.to_string()),
        }
    }

    async fn list(&self) {
        let store = self.store.clone();
        let res = tokio::task::spawn_blocking(move || store.list()).await;
        match res {
            Ok(Ok(sessions)) => self.emit(Event::SessionList { sessions: Box::new(sessions) }),
            Ok(Err(e)) => self.fail("list", e.to_string()),
            Err(e) => self.fail("list", e.to_string()),
        }
    }

    fn emit(&self, event: Event) {
        let _ = self.bus.publish(event);
    }

    fn fail(&self, operation: &str, error: String) {
        self.emit(Event::SessionOperationFailed {
            operation: operation.to_string(),
            error,
        });
    }
}

fn metadata_from_session(session: &Session, name: &str) -> SessionMetadata {
    SessionMetadata {
        id: name.to_string(),
        display_name: session.display_name.clone().unwrap_or_else(|| name.to_string()),
        created_at: session.created_at,
        updated_at: crate::message::now(),
        message_count: session.messages.len(),
        summary: None,
        is_starred: false,
        is_system: false,
    }
}
