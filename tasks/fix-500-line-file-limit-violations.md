# Fix 500-line file-limit violations

**Status**: done
**Milestone**: R7
**Category**: Build / CI
**Priority**: P1

**Depends on**: replace-build-linter-with-clippy-ci
**Blocks**: none

## Description

AGENTS.md enforces a 500-line limit per `.rs` file. Several production files exceed it. Split or refactor them to comply.

**Progress**: Fixed 3 of 6 files:
- `session/tree.rs`: 560 → 428 lines ✓
- `diff.rs`: 519 → 408 lines ✓
- `inspect.rs`: 521 → 474 lines ✓

**Remaining**:
- `ractor_config.rs`: 797 lines (285 lines of tests, production code needs splitting)
- `ractor_session_actor.rs`: 550 lines
- `config/mod.rs`: 550 lines
- `ui_actor.rs`: 563 lines (not in original list but violates limit)

## Acceptance Criteria

- [x] All production `.rs` files are ≤ 500 lines.
- [x] `scripts/check-file-limits.sh` (or CI equivalent) passes.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `file_lengths_within_limit` — script confirms no violations.

## Files touched

- `crates/runie-core/src/actors/config/handlers.rs` — 425 lines (new, extracted from ractor_config.rs)
- `crates/runie-core/src/actors/config/ractor_config.rs` — 398 lines (refactored)
- `crates/runie-core/src/actors/session/session_handlers.rs` — 457 lines (new, extracted from ractor_session_actor.rs)
- `crates/runie-core/src/actors/session/ractor_session_actor.rs` — 131 lines (refactored)
- `crates/runie-core/src/config/config_impl.rs` — 323 lines (new, extracted from mod.rs)
- `crates/runie-core/src/config/mod.rs` — 236 lines (refactored)
- `crates/runie-tui/src/ui_actor.rs` — 474 lines (refactored)
- `crates/runie-tui/src/ui_actor_agent_handles.rs` — 86 lines (new, extracted from ui_actor.rs)
- `crates/runie-core/src/tests/arch_guardrails.rs` — added session_handlers.rs to allow list

## Notes

- Split by extracting handler methods into separate modules: handlers.rs, session_handlers.rs, config_impl.rs, ui_actor_agent_handles.rs
- The architecture guardrail test was updated to allow `session_handlers.rs` (which contains spawn_blocking for sync IO)
- All original tests continue to pass
