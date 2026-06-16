# Align build.rs Lint Thresholds with AGENTS.md

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

`AGENTS.md` mandates 500-line files / 40-line functions / 10 complexity (with a temporary 2000/150/30 relaxation), but `crates/runie-core/build.rs` enforces 1000/120/25. This allows 14 files over 500 lines to pass the build silently.

## Acceptance Criteria

- [ ] `build.rs` thresholds match the intended targets in `AGENTS.md`.
- [ ] An explicit, documented allow-list exists for files/functions currently over limit so the build stays green.
- [ ] Allow-list entries reference follow-up tasks or are marked temporary.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `build_rs_parses` — `build.rs` compiles and runs.
- [ ] `thresholds_match_agents_md` — constants equal the documented targets (or the documented temporary values).

## Files touched

- `crates/runie-core/build.rs`
- `AGENTS.md`

## Notes

The allow-list should shrink as `extract-core-monolith.md`, `coalesce-update-modules.md`, and related tasks land.
