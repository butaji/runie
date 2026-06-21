//! Session persistence — pure domain struct (no file IO).
//!
//! All actual file IO is in `actors/io/session_io.rs`, called from
//! `SessionActor` via `spawn_blocking`.
//!
//! Each session gets its own redb database file: `<dir>/<id>.redb`.
//! Tables:
//!   - `meta`    (key=u32::MAX) → JSON-serialized SessionMetadata
//!   - `events`  (key=u32)       → JSON-serialized DurableCoreEvent

/// Alias for backward compatibility with SessionActor.
pub use crate::session::index::SessionMetadata as SessionMeta;

use std::path::{Path, PathBuf};

/// Redb-backed session store — pure domain struct. No file IO in this module.
#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    /// Create a new store at the given directory.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path to the redb file for a session.
    pub fn path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{}.redb", session_id))
    }
}
