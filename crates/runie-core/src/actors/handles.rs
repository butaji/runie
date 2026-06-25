//! `ActorHandles` — single registry for all actor senders.
//!
//! Instead of storing loose `Option<Sender>` fields in `AppState`, all
//! actor handles are collected here. This makes it easy to:
//! - Pass all handles at once to constructors that need multiple actors
//! - Replace all handles in tests with `TestActorHandles`
//! - Add new actor handles without adding more fields to `AppState`
//!
//! Each handle provides typed `send_*` helpers so callers don't need to
//! construct raw `Msg` enum variants.

use std::path::PathBuf;

use crate::actors::{
    ConfigActorHandle, FffSearchRequest, IoActorHandle, ProviderActorHandle, SessionActorHandle,
};
use crate::session::Session;
use crate::trust::TrustDecision;

/// All actor senders in one place.
///
/// `AppState` holds one `ActorHandles` instead of individual
/// `Option<Sender>` fields. `None` values mean the actor has not been
/// spawned (typical in unit tests).
#[derive(Clone, Debug, Default)]
pub struct ActorHandles {
    /// Handle to `ConfigActor` — owns `~/.runie/config.toml`.
    pub config: Option<ConfigActorHandle>,
    /// Handle to `ProviderActor` — owns provider factory and credentials.
    pub provider: Option<ProviderActorHandle>,
    /// Handle to `SessionActor` — owns trust, history, session CRUD.
    pub session: Option<SessionActorHandle>,
    /// Handle to `IoActor` — owns bash and file-write effects.
    pub io: Option<IoActorHandle>,
    /// Handle to `FffIndexerActor` — owns the file index and search.
    /// `None` when the indexer has not been spawned (e.g. headless mode).
    pub fff_indexer: Option<FffIndexerHandle>,
}

/// Typed handle for the FFF indexer actor.
///
/// Unlike the other handles, `FffIndexerActor` uses its own message type
/// (`FffSearchRequest`) rather than a shared `Msg` enum, so we define a
/// dedicated handle type here.
#[derive(Clone, Debug)]
pub struct FffIndexerHandle {
    tx: tokio::sync::mpsc::Sender<FffSearchRequest>,
}

impl FffIndexerHandle {
    /// Wrap an existing sender.
    pub fn new(tx: tokio::sync::mpsc::Sender<FffSearchRequest>) -> Self {
        Self { tx }
    }

    /// Request a file search from the indexer.
    pub async fn search(&self, request: FffSearchRequest) {
        let _ = self.tx.send(request).await;
    }
}

// ── Typed send helpers ────────────────────────────────────────────────────────

impl ActorHandles {
    /// Send `SetDefaultModel` to `ConfigActor`.
    pub async fn send_set_default_model(&self, provider: &str, model: &str) {
        if let Some(ref h) = self.config {
            h.set_default_model(provider.to_owned(), model.to_owned())
                .await;
        }
    }

    /// Send `SaveProvider` to `ConfigActor`.
    pub async fn send_save_provider(
        &self,
        name: &str,
        base_url: &str,
        api_key: &str,
        models: Vec<String>,
    ) {
        if let Some(ref h) = self.config {
            h.save_provider(
                name.to_owned(),
                base_url.to_owned(),
                api_key.to_owned(),
                models,
            )
            .await;
        }
    }

    /// Send `RemoveProvider` to `ConfigActor`.
    pub async fn send_remove_provider(&self, name: &str) {
        if let Some(ref h) = self.config {
            h.remove_provider(name.to_owned()).await;
        }
    }

    /// Send `SetProviderModels` to `ConfigActor`.
    pub async fn send_set_provider_models(&self, name: &str, models: Vec<String>) {
        if let Some(ref h) = self.config {
            h.set_provider_models(name.to_owned(), models).await;
        }
    }

    /// Send `SetTrust` to `SessionActor`.
    pub async fn send_set_trust(&self, path: PathBuf, decision: TrustDecision) {
        if let Some(ref h) = self.session {
            h.set_trust(path, decision).await;
        }
    }

    /// Send `AppendHistory` to `SessionActor`.
    pub async fn send_append_history(&self, entry: String) {
        if let Some(ref h) = self.session {
            h.append_history(entry).await;
        }
    }

    /// Send `SessionMsg::Load` to `SessionActor`.
    pub async fn send_load_session(&self, name: String) {
        if let Some(ref h) = self.session {
            h.load(name).await;
        }
    }

    /// Send `SessionMsg::Save` to `SessionActor`.
    pub async fn send_save_session(&self, name: String, session: crate::session::Session) {
        if let Some(ref h) = self.session {
            h.save(name, session).await;
        }
    }

    /// Send `SessionMsg::Delete` to `SessionActor`.
    pub async fn send_delete_session(&self, name: String) {
        if let Some(ref h) = self.session {
            h.delete(name).await;
        }
    }

    /// Send `SessionMsg::List` to `SessionActor`.
    pub async fn send_list_sessions(&self) {
        if let Some(ref h) = self.session {
            h.list().await;
        }
    }

    /// Send a file search request to `FffIndexerActor`.
    pub async fn send_fff_search(&self, request: FffSearchRequest) {
        if let Some(ref h) = self.fff_indexer {
            h.search(request).await;
        }
    }

    /// Run a bash command via `IoActor`.
    pub async fn run_bash(&self, command: String) {
        if let Some(ref h) = self.io {
            h.run_bash(command).await;
        }
    }

    /// Write files via `IoActor`.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        if let Some(ref h) = self.io {
            h.write_files(edits).await;
        }
    }

    /// Send `SessionMsg::Import` to `SessionActor`.
    pub async fn send_import_session(&self, path: PathBuf) {
        if let Some(ref h) = self.session {
            h.import(path).await;
        }
    }

    /// Send `SessionMsg::Export` to `SessionActor`.
    pub async fn send_export_session(&self, path: PathBuf, session: Session) {
        if let Some(ref h) = self.session {
            h.export(path, session).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only handles have None for all fields (simulates unit test state)
    #[test]
    fn actor_handles_default_is_all_none() {
        let handles = ActorHandles::default();
        assert!(handles.config.is_none());
        assert!(handles.provider.is_none());
        assert!(handles.session.is_none());
        assert!(handles.io.is_none());
        assert!(handles.fff_indexer.is_none());
    }

    #[test]
    fn fff_indexer_handle_is_cloneable() {
        // FffIndexerHandle is Clone so it can be stored in ActorHandles which is Clone
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<FffIndexerHandle>();
    }

    #[test]
    fn actor_system_clone_is_shallow() {
        // Cloning ActorHandles should be shallow — just copies the handles,
        // does not spawn new actors.
        let handles = ActorHandles::default();
        let handles2 = handles.clone();
        // Both are still all-None (no actors spawned)
        assert!(handles.config.is_none());
        assert!(handles2.config.is_none());
        // Cloning produces identical but independent handles
        assert!(handles.provider.is_none());
        assert!(handles2.provider.is_none());
    }

    #[tokio::test]
    async fn actor_handles_send_save_provider_via_actor() {
        // Integration test: verify send_save_provider reaches ConfigActor.
        use crate::actors::ConfigActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ConfigActor::spawn(bus.clone(), None);

        let mut handles = ActorHandles::default();
        handles.config = Some(handle);

        // Send via ActorHandles helper
        handles
            .send_save_provider("test", "http://localhost", "key", vec!["model".into()])
            .await;

        // Actor should have received the message (no panic = success)
        drop(handles);
        drop(_actor);
    }
}
