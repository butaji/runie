# Delete dead runie-core utils module

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/utils.rs` (53 LOC) exports `pub fn truncate(s: &str, n: usize) -> String` and `pub fn join_optional(...)`. `rg 'crate::utils|runie_core::utils|use crate::utils' crates/` returns zero hits — neither helper has any caller inside the workspace. The file is `pub mod utils;` declared in `lib.rs:95` but contributes nothing.

These helpers were early scaffolding that got superseded by `textwrap::wrap()` (for line breaking) and ad-hoc `join("\n")` (for list joining).

## Acceptance Criteria

- [ ] `crates/runie-core/src/utils.rs` deleted.
- [ ] `pub mod utils;` removed from `crates/runie-core/src/lib.rs`.
- [ ] `rg "use crate::utils|runie_core::utils|crate::utils::|runie_core::utils::" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure deletion.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_utils_module_gone` — `ls crates/runie-core/src/utils.rs` fails; workspace builds.

## Files touched

- `crates/runie-core/src/utils.rs`
- `crates/runie-core/src/lib.rs`

## Notes

If a `truncate` helper is needed later, `textwrap::truncate` (already a workspace dep, though only `textwrap::wrap` is currently used in `core/layout.rs`) covers the same case. No behavioural risk.
