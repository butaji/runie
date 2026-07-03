//! Search tool — unified FFF-backed search for files and content.
//!
//! Replaces the separate `grep`, `find`, and `list_dir` tools with a single
//! `search` tool backed by `fff-search`. Supports file search, content search,
//! glob patterns, and git-status filters via a unified query syntax.

mod core;
pub(crate) mod fff_helpers;
mod modes;
mod types;

#[cfg(test)]
mod tests;

pub use core::SearchTool;
