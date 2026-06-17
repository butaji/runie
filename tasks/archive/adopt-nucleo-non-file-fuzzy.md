# Adopt `nucleo-matcher` for Non-File Fuzzy Matching

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the custom `fuzzy.rs` scorer for non-file items (command palette, model selector, dialog panels, session lists) with `nucleo-matcher`. File-path fuzzy matching stays with `fff-search`; this task is scoped to panels and selectors only.

## Acceptance Criteria

- [ ] `nucleo-matcher` is added as a dependency.
- [ ] `fuzzy.rs` / `dialog/score.rs` use `nucleo-matcher` for non-file candidates.
- [ ] File-path scoring continues to use FFF and is not duplicated.
- [ ] Ranking is at least as good as current for existing test cases; add golden tests.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `nucleo_scores_panel_items` — command/model items are scored and ranked.
- [ ] `nucleo_handles_unicode_query` — non-ASCII queries match correctly.

### Layer 2 — Event Handling
- [ ] `panel_filter_event_uses_nucleo` — filtering a panel publishes the expected selection event.

### Layer 3 — Rendering
- [ ] `panel_renders_filtered_items` — filtered panel renders in expected order.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/fuzzy.rs`
- `crates/runie-core/src/dialog/score.rs`

## Notes

- Reuse `Matcher` across calls to avoid repeated allocations.
- See `docs/CRATE_DECISIONS.md`.
