# Decompose `runie-core` god crate and flatten `Event` enum

**Status**: todo
**Milestone**: R8
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`runie-core` has grown into a catch-all crate (~60k production lines, ~40 public modules). Its `Event` enum at `crates/runie-core/src/event/mod.rs` is 1,253 lines long, with `Event::kind` (231 lines), `Event::category` (230 lines), and `Event::into_intent` (153 lines) all hand-written. This monolith slows compiles, creates merge conflicts, and forces every consumer to depend on unrelated subsystems.

## Acceptance Criteria

- [ ] Introduce category-specific event enums (`InputEvent`, `AgentEvent`, `DialogEvent`, `IoEvent`, `SessionEvent`, etc.) and wrap them in a top-level `Event`, OR use a trait-based event envelope.
- [ ] Move event metadata (`kind`, `category`, `into_intent`) out of the enum into a generated lookup or per-variant derive.
- [ ] Reduce `runie-core/src/event/mod.rs` to under the 500-line aspirational limit.
- [ ] Document a crate-boundary plan to split `runie-core` into smaller purpose-built crates (`runie-events`, `runie-model`, `runie-actors`, `runie-config`).
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_category_lookup_matches_legacy_match` — new metadata lookup returns the same `EventKind`/`EventCategory` as the old match.

### Layer 2 — Event Handling
- [ ] `categorized_events_serialize_roundtrip` — every variant round-trips through JSON/compact serialization.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_event_stream_unchanged` — existing SSE replay fixtures produce the same event stream after refactor.

### Live Tmux Testing Session
- [ ] Run the TUI through startup, a turn, and quit; verify events are emitted and rendered as before.

## Files touched

- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-core/src/lib.rs`
- `Cargo.toml` (workspace)

## Notes

- This is a large, risky refactor. Prefer incremental PRs: first introduce category enums behind the existing `Event` API, then migrate consumers, then split crates.
- Consider `strum` or a small proc-macro for event metadata.
