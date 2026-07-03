//! Tool default limits and thresholds.
//!
//! Centralizes magic numbers for tool result limits so they're easy to
//! audit, adjust, and test.

/// Default maximum number of grep matches to return.
pub const GREP_DEFAULT_LIMIT: usize = 100;

/// Default maximum number of find results to return.
pub const FIND_DEFAULT_LIMIT: usize = 100;

/// Default maximum number of find_definitions results to return.
pub const FIND_DEFINITIONS_DEFAULT_LIMIT: usize = 30;

/// Default maximum number of search results to return.
pub const SEARCH_DEFAULT_LIMIT: usize = 50;

/// Default maximum matches per file for content search.
pub const SEARCH_DEFAULT_MAX_MATCHES: usize = 10;
