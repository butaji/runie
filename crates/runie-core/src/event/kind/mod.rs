//! Event taxonomy — `EventKind` enum and helper predicates.
//!
//! ## Naming Convention
//!
//! - **Intents** are imperative or noun-phrase requests from handlers/UI to actors:
//!   `SetTheme`, `TrustProject`, `SubmitInput`, `RunCompact`.
//!   Named like "set X", "do Y" — what the user/system wants.
//! - **Facts** are past-tense or descriptive broadcasts from actors:
//!   `ConfigLoaded`, `TrustChanged`, `SessionSaved`, `ToolEnd`.
//!   Named like "X happened" or "X changed" — what actually occurred.
//! - **Controls** are lifecycle / terminal signals:
//!   `Quit`, `Abort`, `Reset`, `TerminalSize`.
//!
//! ## Routing
//!
//! - Facts → `AppState::update()` (the projection path)
//! - Intents → actors via `ActorHandles` (see `actors/handles.rs`)
//! - Controls → `dispatch_event()` system handler (no state mutation)
//!
//! See [`intent`](crate::event::intent) for the typed intent enum.

pub use crate::event::EventCategory;

/// Kind of an `Event` — the top-level taxonomy for state sync.
///
/// Intents request state changes (routed to actors).
/// Facts describe state changes (projected into `AppState`).
/// Controls manage lifecycle / terminal events (routed to `update/system.rs`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    strum::Display,
    strum::IntoStaticStr,
    strum::VariantNames,
)]
pub enum EventKind {
    /// Request to an actor — produced by input handlers, commands, dialogs.
    Intent,
    /// Broadcast state change — produced by actors.
    #[default]
    Fact,
    /// Lifecycle / terminal event — produced by the IO layer.
    Control,
}

// The `Event::kind()` and `Event::category()` methods are defined inline in `event/mod.rs`.
