//! Typed intent enum for the declarative DSL.
//!
//! ## Deprecation Notice
//!
//! This module previously contained a parallel `Intent` enum that mirrored
//! `Event` intent variants. It has been collapsed — intents are now represented
//! directly by `Event` variants with `EventKind::Intent`.
//!
//! Use `Event::kind() == EventKind::Intent` to identify intent events, and
//! pattern-match directly on `Event` instead of converting to a separate type.
//!
//! ## Routing
//!
//! See [`kind`](crate::event::kind) for the routing rules (facts → projection,
//! intents → actors, control → system handler).

// Intent is now just an alias/documentation marker — the actual type is Event.
pub use crate::event::Event as Intent;
