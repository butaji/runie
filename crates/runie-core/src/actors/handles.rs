//! `ActorHandles` — re-export alias for `LeaderHandle`.
//!
//! The canonical actor handle registry is `LeaderHandle` (spawned by
//! `Leader::start`). This module re-exports it as `ActorHandles` for
//! backwards compatibility so existing callers don't need to change import
//! paths.
//!
//! ## Migration note
//!
//! New code should use `crate::actors::LeaderHandle` directly. The
//! `ActorHandles` alias will be removed in a future release.

pub use super::leader::LeaderHandle as ActorHandles;
