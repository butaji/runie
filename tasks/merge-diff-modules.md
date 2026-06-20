# Merge duplicate diff.rs modules

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Two `diff.rs` files exist:

| File | LOC | Role |
|------|-----|------|
| `crates/runie-core/src/diff.rs` | — | Domain diff logic (line-level patch application / parsing) |
| `crates/runie-tui/src/diff.rs` | 466 | Rendering-flavored diff (gutter backgrounds, span styling) |

Verify the overlap. The domain file should own pure diff computation; the tui file should own only the rendering adapter (turning domain diff lines into styled spans). If the two share logic, merge into one domain `diff.rs` + a thin render adapter in tui. If the tui file duplicates computation, delete the duplicate and have tui consume the domain types.

## Acceptance Criteria

- [ ] Audit complete: document which functions in `runie-tui/src/diff.rs` duplicate `runie-core/src/diff.rs` vs which are render-only.
- [ ] Duplicated computation removed from `runie-tui/src/diff.rs`; tui consumes `runie_core::diff::` types/functions.
- [ ] `runie-tui/src/diff.rs` contains only rendering adapters (no pure diff logic).
- [ ] `render-tui-diff-from-canonical-type` task (if still relevant) is satisfied by this consolidation.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `core_diff_parses_and_applies` — existing domain diff tests pass unchanged.
- [ ] `no_diff_logic_in_tui_module` — grep assertion: `runie-tui/src/diff.rs` contains no `fn parse`/`fn apply`/pure-computation functions, only render adapters.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] `diff_renders_with_gutter_and_spans` — existing tui diff rendering tests pass after consuming domain types.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green.

## Files touched

- `crates/runie-core/src/diff.rs` — keep (domain logic)
- `crates/runie-tui/src/diff.rs` — strip duplicated logic, keep render adapters only
- Files importing from `runie-tui/src/diff.rs` that should import from `runie_core::diff` (grep-driven)

## Notes

Satisfies `render-tui-diff-from-canonical-type` if that task is still open — this is the concrete implementation. The `similar` crate is already a workspace dep; the domain `diff.rs` should be the sole consumer of `similar` for computation, and tui should not call `similar` directly.
