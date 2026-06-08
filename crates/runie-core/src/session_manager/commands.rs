//! Session Manager Commands and Responses

use crate::event_bus::DomainEvent;
use crate::session_jsonl::SessionMeta;

/// Commands sent to SessionManager actor
#[derive(Debug, Clone)]
pub enum SessionCmd {
    /// Start a new session
    StartSession { name: String, provider: String, model: String },
    /// Resume an existing session
    ResumeSession { name: String },
    /// Close current session
    CloseSession,
    /// Record a domain event
    RecordEvent { event: DomainEvent },
    /// Load a session (returns metadata + events)
    LoadSession { name: String },
    /// List all sessions
    ListSessions,
    /// Delete a session
    DeleteSession { name: String },
    /// Request periodic snapshot
    MaybeSnapshot,
    /// Flush pending writes
    Flush,
}

/// Response from SessionManager actor
#[derive(Debug, Clone)]
pub enum SessionResponse {
    /// Session started successfully
    SessionStarted { name: String },
    /// Session resumed successfully with metadata
    SessionResumed { meta: SessionMeta },
    /// Session closed
    SessionClosed,
    /// Event recorded
    EventRecorded,
    /// Session loaded with events
    SessionLoaded { meta: SessionMeta, events: Vec<DomainEvent> },
    /// List of session names
    SessionsListed { names: Vec<String> },
    /// Session deleted
    SessionDeleted { name: String },
    /// Snapshot taken
    SnapshotTaken,
    /// Error occurred
    Error { message: String },
}
