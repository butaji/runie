//! Session Manager Actor — Delegates to session_manager module
//!
//! This module re-exports the session manager implementation from the
//! parent session_manager module.

pub use crate::session_manager::{run_session_manager, SessionState};
