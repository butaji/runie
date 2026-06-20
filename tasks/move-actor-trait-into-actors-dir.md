# Move Actor trait into actors/ directory

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The `Actor` trait lives at `crates/runie-core/src/actor.rs` (222 LOC) while every concrete actor lives under `crates/runie-core/src/actors/` (`actors/mod.rs` is 24 LOC and just declares submodules). Trait and impls in different module roots is a navigation hazard: `actor.rs` (singular) vs `actors/` (plural) is grep-ambiguous and the split has no semantic meaning. Fold the trait into the `actors/` directory as `actors/trait.rs` (or inline at the top of `actors/mod.rs` if it stays under 500 LOC combined).

## Acceptance Criteria

- [ ] `crates/runie-core/src/actor.rs` deleted.
- [ ] `Actor` trait + `spawn` helpers moved to `crates/runie-core/src/actors/trait.rs` (or inlined into `actors/mod.rs`).
- [ ] `actors/mod.rs` declares `mod trait;` (or contains the trait directly) and re-exports `Actor`, `ActorHandle`, and any spawn helpers.
- [ ] `lib.rs` no longer declares `pub mod actor;`; re-exports come from `actors::`.
- [ ] All `use crate::actor::` and `use runie_core::actor::` imports rewritten to `actors::`.
- [ ] `arch_guardrails.rs` path strings updated if they reference `src/actor.rs`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_trait_resolves_from_actors_module` — `use runie_core::actors::Actor;` compiles.

### Layer 2 — Event Handling
- [ ] N/A — pure file move, no trait changes.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-core/src/actor.rs` → delete (move to `actors/trait.rs`)
- `crates/runie-core/src/actors/mod.rs` — declare `mod trait;` + re-exports
- `crates/runie-core/src/actors/trait.rs` → new (content from `actor.rs`)
- `crates/runie-core/src/lib.rs` — remove `pub mod actor;`, update re-exports
- All files importing `crate::actor::` or `runie_core::actor::` (grep-driven)
- `crates/runie-core/tests/arch_guardrails.rs` — update path strings if needed

## Notes

Use `git mv` to preserve history. Complements `simplify-actor-trait` (which targets the trait's shape and default methods); do this move first so the simplify task operates on the trait in its final location. Rejected alternative: keep `actor.rs` at root as a "trait-only" file — rejected because the singular/plural naming collision is a documented navigation hazard and the trait has no reason to live apart from its impls.
