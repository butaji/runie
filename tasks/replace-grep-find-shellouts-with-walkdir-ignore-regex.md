# Replace grep/find shell-outs with `walkdir`/`ignore`/`regex`

## Status

`done`

## Description

`grep` and `find` tools shell out to `rg`/`grep`/`fd`/`find`, which is cross-platform fragile and duplicates command-building logic. Use `ignore` + `regex` instead.

## Implementation

- Added `ignore` and `regex` dependencies to `runie-agent`
- Rewrote `grep.rs` to use `ignore::WalkBuilder` for directory traversal and `regex` for pattern matching
- Rewrote `find.rs` to use `ignore::WalkBuilder` for directory traversal with glob-to-regex conversion
- Removed `FIND_FALLBACK_MAX_DEPTH` constant (no longer needed)

## Acceptance Criteria

- [x] Results match old shell-out behavior for representative queries
- [x] Unit tests cover glob patterns, regex, literal mode, case insensitivity, and limits
- [x] `cargo test --workspace` passes
- [x] `cargo check --workspace` passes with no warnings

## Tests

### Unit tests
- [x] `grep_no_matches` — no matches returns appropriate message
- [x] `grep_finds_matches` — finds matches in files
- [x] `grep_case_insensitive` — case insensitive mode works
- [x] `grep_literal` — literal mode escapes regex special chars
- [x] `grep_invalid_regex` — invalid regex returns error
- [x] `grep_respects_limit` — limits results correctly
- [x] `find_no_matches` — no matches returns appropriate message
- [x] `find_exact_match` — exact filename match works
- [x] `find_glob_pattern` — glob patterns like `*.txt` work
- [x] `find_star_pattern` — `test*` patterns work
- [x] `find_respects_limit` — limits results correctly
- [x] `find_nested_directories` — searches nested directories
- [x] `find_question_mark` — `?` wildcard works

### E2E tests
- [x] Replay tests cover grep and find tools

### Live tmux tests
- [ ] Ask the agent to grep a pattern and find files in a real session (not yet automated)

## Files Touched

- `crates/runie-agent/Cargo.toml` — added `ignore` and `regex` dependencies
- `crates/runie-agent/src/tool/grep.rs` — native Rust grep implementation
- `crates/runie-agent/src/tool/find.rs` — native Rust find implementation
- `crates/runie-agent/src/tool/constants.rs` — removed `FIND_FALLBACK_MAX_DEPTH`
