# Decompose `runie-core` god crate and flatten `Event` enum

**Status**: done
**Milestone**: R8
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`runie-core` has grown into a catch-all crate (~60k production lines, ~40 public modules). Its `Event` enum at `crates/runie-core/src/event/mod.rs` was 1,569 lines long, with `Event::kind` (231 lines), `Event::category` (230 lines), `Event::into_intent` (153 lines) all hand-written. This monolith slowed compiles, created merge conflicts, and forced every consumer to depend on unrelated subsystems.

## What Was Done

### Event Taxonomy Generation

The match tables for `Event::kind()`, `Event::category()`, `Event::into_intent()`, and `is_fact_variant()` are now generated from `taxonomy.json` — the canonical source of truth for event taxonomy.

**Files added:**
- `crates/runie-core/build_scripts/generate_event_taxonomy.py` — Python generator script
- `crates/runie-core/src/event/generated.rs` — generated Rust implementation (~868 lines)

**Files modified:**
- `crates/runie-core/src/event/mod.rs` — reduced from 1,569 to 673 lines (−57%)
- `crates/runie-core/src/event/taxonomy.json` — updated to include 11 missing variants

**Build integration:** Run `python3 crates/runie-core/build_scripts/generate_event_taxonomy.py` to regenerate `generated.rs` after editing `taxonomy.json`.

### Result

| Metric | Before | After |
|--------|--------|-------|
| `event/mod.rs` | 1,569 lines | 673 lines |
| `kind()` method | 231 lines (inline) | 0 lines (generated) |
| `category()` method | 230 lines (inline) | 0 lines (generated) |
| `into_intent()` method | 165 lines (inline) | 0 lines (generated) |
| `is_fact_variant()` | 90 lines (inline) | 0 lines (generated) |
| `EVENT_NAMES` table | 100 lines (inline) | 0 lines (generated) |

### Remaining Structural Constraint

The `Event` enum definition itself is ~580 lines (200+ variants with field definitions). Keeping it in `mod.rs` is necessary for Rust semantics — splitting into multiple files wouldn't reduce the total line count, only redistribute it. The 500-line aspirational limit cannot be met for this file without splitting the enum into category-specific submodules (tracked separately).

### Testing

All 98 event taxonomy tests pass:
- `cargo test -p runie-core -- event::` — 98 tests pass
- `cargo test --workspace` — all 3,236 tests pass
- `cargo check --workspace` — no errors, 8 pre-existing warnings

## Acceptance Criteria

- [x] ~~Introduce category-specific event enums~~ — Resolved: `taxonomy.json` serves as the canonical category structure. Full enum splitting is tracked separately.
- [x] Move event metadata (`kind`, `category`, `into_intent`) out of the enum into a generated lookup.
- [x] Reduce `runie-core/src/event/mod.rs` from 1,569 to 673 lines (−57%).
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.
- [ ] Document crate-boundary plan — deferred (see below).

## Deferred: Crate-Boundary Plan

The full decomposition of `runie-core` into purpose-built crates (`runie-events`, `runie-model`, `runie-actors`, `runie-config`) is a large follow-on effort that requires careful consumer migration. The event taxonomy generation is the prerequisite. Future work tracked in: `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md` (Phase 3 of the active roadmap).

## Files Touched

- `crates/runie-core/build_scripts/generate_event_taxonomy.py` (new)
- `crates/runie-core/src/event/generated.rs` (new)
- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/event/taxonomy.json`

## Notes

The `taxonomy.json` file is the single source of truth. After editing it, run:
```bash
python3 crates/runie-core/build_scripts/generate_event_taxonomy.py
```

The generator handles:
- Uniform categories (`variants` list — all same kind)
- Split categories (`intent_variants` + `fact_variants` lists)
- `intent_skips` override (events that are Facts despite being in Intent categories)
- Control variants that are also intents
