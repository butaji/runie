//! `ActorHandles` — typed registry of actor handles for all production actors.
//!
//! Each field is a handle type with convenience methods. Callers use
//! `handle.some_method()` instead of raw `send_message` calls. Only
//! `fff_indexer` is `Option` because it can fail to spawn.
//!
//! ## Usage
//!
//! ```ignore
//! handles.io.open_external_editor(text).await;
//! handles.provider.validate_key(provider, key).await;
//! handles.config.send_message(ConfigMsg::SetTheme { name });
//! ```

use crate::actors::{
    RactorConfigHandle, RactorFffIndexerHandle, RactorInputHandle, RactorIoHandle,
    RactorPermissionHandle, RactorProviderHandle, RactorSessionHandle, RactorTurnHandle,
};

/// Single registry of actor handles for all production actors.
///
/// The `AppState` layer wraps this in `Option<ActorHandles>` so callers guard
/// with `if let Some(handles) = state.actor_handles()`.
#[derive(Clone, Debug)]
pub struct ActorHandles {
    /// Config actor handle.
    pub config: RactorConfigHandle,
    /// Provider actor handle.
    pub provider: RactorProviderHandle,
    /// Session actor handle.
    pub session: RactorSessionHandle,
    /// IO actor handle (convenience methods for clipboard, editor, gist, etc.).
    pub io: RactorIoHandle,
    /// FFF indexer handle (None if indexer failed to spawn).
    pub fff_indexer: Option<RactorFffIndexerHandle>,
    /// Input actor handle.
    pub input: RactorInputHandle,
    /// Permission actor handle.
    pub permission: RactorPermissionHandle,
    /// Turn actor handle.
    pub turn: RactorTurnHandle,
    /// Join handle for the turn actor (for graceful shutdown).
    /// Wrapped in Arc so ActorHandles remains Clone.
    pub turn_join: Option<std::sync::Arc<tokio::task::JoinHandle<()>>>,
}

impl Default for ActorHandles {
    fn default() -> Self {
        panic!("ActorHandles::default() is not valid; construct via spawn")
    }
}

impl ActorHandles {
    /// Returns 8 for the 8 always-present actor handles.
    pub const fn count(&self) -> usize { 8 }

    /// Check if the fff indexer is present.
    pub fn has_fff_indexer(&self) -> bool { self.fff_indexer.is_some() }
}
