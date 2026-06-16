# Split Large Files Into Modules

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Split files over 500 lines into focused modules. Target files:

| File | Current Lines | Target |
|------|--------------|--------|
| `crates/runie-core/src/state.rs` | 790 | Split into `agent_state.rs`, `view_state.rs`, `session_state.rs` |
| `crates/runie-tui/src/theme.rs` | 657 | Split into `theme/colors.rs`, `theme/styles.rs` |
| `crates/runie-core/src/harness_skills.rs` | 684 | Split into `harness_skill/*.rs` |
| `crates/runie-core/src/tool/search.rs` | 683 | Split into `search/*.rs` |
| `crates/runie-core/src/orchestrator.rs` | 612 | Split into `orchestrator/*.rs` |
| `crates/runie-core/src/update/mod.rs` | 631 | Split into `update/*.rs` |
| `crates/runie-core/src/keybindings.rs` | 571 | Split into `keybindings/*.rs` |
| `crates/runie-core/src/actors/fff_indexer.rs` | 570 | Already has submodules, check structure |

## Acceptance Criteria

- [ ] All files under 500 lines.
- [ ] Public API unchanged (re-exports from `lib.rs`).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A (restructuring only).

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test passes after restructuring.

## Files touched

Multiple files in `crates/runie-core/` and `crates/runie-tui/`

## Notes

High-effort task. Do after unifying status enums to reduce churn.
