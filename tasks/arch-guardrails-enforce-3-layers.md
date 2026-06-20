# Enforce 3-layer architecture with guardrails

**Status**: done  
**Milestone**: R4  
**Category**: Architecture / Actors  
**Priority**: P0  

**Depends on**: none  
**Blocks**: centralize-app-state-ownership, remove-io-from-runie-core, simplify-event-vocabulary, pure-snapshot-and-tool-runtime-trait, consolidate-binary-setup  

## Description

Before refactoring the rest of the codebase, establish enforceable guardrails that codify the target architecture: IO behind interfaces, domain pure, UI pure/MVU. This task creates architectural tests and updates the canonical documentation so future changes cannot reintroduce the violations we are about to remove.

## Acceptance Criteria

- [x] `docs/Architecture.md` explicitly states the 3-layer rule and actor responsibility table.
- [x] A test fails if new sync IO appears in `crates/runie-core/src` production code outside approved adapter modules and the documented legacy allow-list.
- [x] A test fails if a new `&mut AppState` parameter is introduced in production code outside `runie-core/src/update/` and `runie-core/src/model/state/`.
- [x] `AppState::snapshot` signature is tracked; the guardrail documents the current `&mut self` violation and will be flipped to require `&self` in Phase 4.
- [x] The flat `Event` enum variant count is captured and a budget test prevents unbounded growth.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `arch_test_no_sync_io_in_core` — scans `crates/runie-core/src` for `std::fs::`, `std::process::Command`, `std::env::current_dir`, `arboard` usage outside allow-listed adapter modules and documented legacy files.
- [x] `arch_test_app_state_mutation_is_centralized` — parses Rust source for `&mut AppState` parameters and asserts only approved locations plus documented legacy files.
- [x] `arch_test_snapshot_signature_is_tracked` — documents the current `snapshot(&mut self)` signature and will be updated to require `&self` once Phase 4 lands.
- [x] `arch_test_event_enum_variant_budget` — counts `Event` enum variants and asserts they are at or below the current baseline of 109.
- [x] `arch_test_event_variant_count_is_tracked` — fails if the variant count changes without an intentional baseline update.

### Layer 2 — Event Handling
- [x] N/A — guardrails are static source scans, not runtime event handling.

### Layer 3 — Rendering
- [x] N/A — guardrails do not add rendering behavior.

### Layer 4 — Smoke / Crash
- [x] Architectural tests run as part of `cargo test --workspace`.

## Files touched

- `docs/Architecture.md`
- `crates/runie-core/tests/arch_guardrails.rs` (new)
- `tasks/index.json`
- `tasks/arch-guardrails-enforce-3-layers.md`

## Notes

- The sync-IO and `&mut AppState` tests use explicit legacy allow-lists so the suite passes today. Remove entries from those lists as Phases 1 and 2 land.
- Test files (`tests/` directories and `*_tests.rs`/`tests.rs` modules) are exempt from the production-code guardrails.
- Baseline variant count is 109, budget is 120. Lower the budget when variants are nested or removed.
- The snapshot signature guardrail is intentionally pragmatic: it documents the current violation and will be flipped to require `&self` by `pure-snapshot-and-tool-runtime-trait`.
