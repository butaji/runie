# Centralize tool default limits

## Status

`done`

## Description

`grep`/`find`/`find_definitions` tools use hardcoded default limits (`100`, `10`, `5`, `200`) and depth values. Centralize these in `runie-agent::tool::constants`.

## Implementation

Created `crates/runie-agent/src/tool/constants.rs`:

```rust
//! Tool default limits and thresholds.

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

/// Default maximum depth for find fallback traversal.
pub const FIND_FALLBACK_MAX_DEPTH: usize = 10;
```

Updated tools to use these constants:
- `grep.rs` — uses `GREP_DEFAULT_LIMIT`
- `find.rs` — uses `FIND_DEFAULT_LIMIT` and `FIND_FALLBACK_MAX_DEPTH`
- `find_definitions.rs` — uses `FIND_DEFINITIONS_DEFAULT_LIMIT`
- `search/types.rs` — uses `SEARCH_DEFAULT_LIMIT` and `SEARCH_DEFAULT_MAX_MATCHES`

## Acceptance Criteria

- [x] **Unit tests** — Tool defaults are named constants and covered by unit tests.
- [x] **E2E tests** — Mock-provider tool calls still produce the same result counts.
- [x] **Live tmux tests** — Ask the agent to grep/find/define in tmux and verify result limits.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
