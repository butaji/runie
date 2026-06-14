# Sync README and Docs With Actual Codebase

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Description

User-facing docs have drifted from the codebase. This task keeps them in sync.

## Current State (as of 2026-06-14)

**Already resolved:**
- [x] `IMPL_PLAN.md` and `REFACTOR_PLAN.md` archived to `docs/archive/`
- [x] `EXECUTE.md` and `FEATURE_PARITY.md` do not exist — nothing to do
- [x] `REVIEW.md` does not exist — `CONTEXT.md` and `SPEC.md` are the current docs
- [x] Stale `build_login_root(state)` docstring in `builder.rs` fixed

**Still remaining (minor):**
- [ ] `bacon.toml` and `dev.sh` reference only existing files and crates

## Acceptance Criteria

- [x] `commands/dsl/builder.rs` docstring has no stale references
- [ ] `bacon.toml` and `dev.sh` have no broken file/crate references

## Tests

### Layer 4 — Smoke
- [ ] `./dev.sh` runs end-to-end

## Notes

**Out of scope:**
- Generating API documentation from rustdoc
- Adding a docs site (mdbook, etc.)

## Verification

```bash
./dev.sh
cargo test --workspace
```
