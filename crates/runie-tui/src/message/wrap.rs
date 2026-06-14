//! Word-wrapping helpers for message rendering.
//!
//! The implementation lives in `runie_core::layout` so that core scroll math
//! and the TUI renderer share the exact same wrapping rules.

pub(crate) use runie_core::layout::word_wrap;
