# Move `crates/runie-agent/src/grep_find.rs` to `tests/`

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Description

`crates/runie-agent/src/grep_find.rs` (117 lines) is declared
in `lib.rs:48` as `mod grep_find;` but the file is **100%
`#[cfg(test)]` content**. It contains 6 tests for the `Grep` and
`Find` tool parsers. The file should live in the `tests/`
directory (which is `#[cfg(test)]` automatically), not in `src/`.

This is a trivial cleanup: the file is test-only code in a
production path. The current setup means:
- The `grep_find` module is part of the public API surface
  (technically reachable as `runie_agent::grep_find`)
- IDE tooling may index it as production code
- The build wrapper has to scan it on every `cargo build`
- It violates the "src/ is for production code" convention

## Current State

`crates/runie-agent/src/grep_find.rs` (117 lines):

```rust
use crate::{parser::parse_tool_calls, Tool};

#[test]
fn parse_grep_tool_json() { ... }

#[test]
fn parse_find_tool_json() { ... }

#[test]
fn grep_executes_and_finds_matches() { ... }

// 3 more tests, all #[cfg(test)]-style test bodies
```

There is **zero non-test code** in this file. No `pub fn`,
no `pub struct`, no `pub const`. Every line is either a `use`
or a `#[test]` declaration.

## Acceptance Criteria

- [ ] `crates/runie-agent/src/grep_find.rs` is deleted
- [ ] `crates/runie-agent/src/tests/grep_find.rs` is created with
  the same 6 tests
- [ ] `crates/runie-agent/src/lib.rs:48` `mod grep_find;` line is
  removed
- [ ] `crates/runie-agent/src/lib.rs` `#[cfg(test)] mod grep_find;`
  is added (or the file is included via `mod tests;` which
  already exists at line 49)
- [ ] `cargo build -p runie-agent` succeeds
- [ ] `cargo test -p runie-agent --lib` succeeds
- [ ] The same 6 tests run (verified by `cargo test --list`)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build -p runie-agent` succeeds
- [ ] `cargo test -p runie-agent --lib` succeeds
- [ ] `cargo test -p runie-agent --lib grep_find` lists 6 tests
  after the move
- [ ] `grep "grep_find" crates/runie-agent/src/lib.rs` returns
  zero hits after the move (the module is fully gone from `src/`)

### Layer 4 — Smoke
- [ ] `cargo test --workspace` succeeds with the same total
  test count (1,637 — the 6 grep_find tests plus the existing
  1,631)

## Notes

**Why is this P2:** Trivial mechanical move, no design
decisions. Doesn't fix any user-visible bug. Improves code
organization and reduces the `src/` surface.

**Pattern to follow:**

```rust
// In src/lib.rs, the current pattern is:
mod grep_find;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod truncate_tests;
```

The new pattern:

```rust
// src/lib.rs (after the move)
// No more `mod grep_find;`

#[cfg(test)]
mod tests;
#[cfg(test)]
mod truncate_tests;
```

`crates/runie-agent/src/tests/grep_find.rs` is then picked up
by the existing `mod tests;` declaration (which uses
`mod.rs`-style file inclusion) and `mod.rs:5` already does
`pub mod grep_find;`-like declarations for its submodules.

Wait, let me check what `tests/mod.rs` looks like:</mm:think>