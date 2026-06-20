# Sweep redundant `.to_string()` allocations

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`.to_string()` appears 680x in the workspace vs `.to_owned()` 2x. Two waste patterns hide in that count: (1) `some_string.to_string()` on an already-owned `String` — a redundant clone; (2) `&str.to_string()` where the target type is `String` and `.into()` is idiomatic. The mix of `to_string`/`to_owned`/`into` is stylistic noise. Mechanical sweep to remove redundant clones and unify on `.into()` (or `.to_owned()` for `&str` → `String`).

## Acceptance Criteria

- [ ] All `String.to_string()` redundant clones removed (grep finds zero `: String = .*\.to_string\(\)` where the receiver is already `String`).
- [ ] `&str.to_string()` in `String`-typed positions converted to `.into()` or `.to_owned()`.
- [ ] No `to_string()` on `String` receivers remains in production code.
- [ ] `cargo clippy --workspace` passes (clippy::str_to_string, clippy::redundant_clone).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — no logic change, only allocation removal.

### Layer 2 — Event Handling
- [ ] N/A — no event handling change.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms no behavior change from the sweep.

## Files touched

- Workspace-wide sweep across `crates/runie-core/src/`, `crates/runie-tui/src/`, `crates/runie-agent/src/`, `crates/runie-provider/src/`, `crates/runie-engine/src/`.
- Highest-clone files from prior audit: `session_replay.rs` (27), `model/state/app_state.rs` (25), `model/cache.rs` (23), `update/login_flow.rs` (17) — start here.

## Notes

Use `rg "\.to_string\(\)" --type rust` to enumerate; for each site, check the receiver type. If `String`, it's a redundant clone — remove or replace with `clone()` if the original must be retained. If `&str` and the binding is `String`, use `.into()`. Enable `clippy::str_to_string` and `clippy::redundant_clone` in `Cargo.toml` or run `cargo clippy -- --warn clippy::str_to_string` to catch them automatically. Low-effort, mechanical, but catches real allocations. Do not touch `.to_string()` on non-string types (Display impls) — those are legitimate.
