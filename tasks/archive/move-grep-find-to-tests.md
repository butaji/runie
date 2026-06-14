# Move `grep_find.rs` from `src/` to `tests/`

**Status**: stale
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Resolution

Not started. `crates/runie-agent/src/grep_find.rs` still exists as a `mod` in `lib.rs:50`.
The file contains 6 tests for `Grep` and `Find` tool parsers — it is 100% test code.
The mechanical fix is to delete `src/grep_find.rs`, remove `mod grep_find;` from `lib.rs`,
and move tests to `src/tests/`. The `crate-replacement-audit` (done) and `agent-runtime-decision` (open) may change `runie-agent`'s structure anyway, so this is deprioritized.

Archived in tasks/archive/ as stale.
