# Use `strum` to derive `Event`/`Intent`/`EventKind` names

**Status**: todo
**Milestone**: R1
**Category**: Core / State
**Priority**: P1

**Depends on**: collapse-event-intent-kind-taxonomies
**Blocks**: none

## Description

`runie-core/src/event/names.rs`, `name.rs`, and the manual `intent_impl.rs`/`kind/mod.rs` tables duplicate information that `strum` can generate from the enum definitions. `goose` already uses `strum` for derive-driven enum handling. Switching to `strum` removes hundreds of lines of mirror code and guarantees names stay in sync with variants.

## Acceptance Criteria

- [ ] Add `strum = { version = "0.28", features = ["derive", "std"] }` to `runie-core` (or workspace).
- [ ] `Event` derives `Display`, `EnumString`, `EnumIter`, `IntoStaticStr`, and `VariantNames` where appropriate.
- [ ] `EventKind`, `EventCategory`, and `Intent` derive the same traits; manual `name()`/`from_name()`/`names.rs`/`name.rs` helpers are deleted.
- [ ] `Intent` variants are generated from annotated `Event` variants (see `collapse-event-intent-kind-taxonomies`) so intent names also come from `strum`.
- [ ] `EVENT_NAMES` and similar static lookup tables are replaced with `VariantNames`/`EnumIter`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_name_round_trip` — every bindable variant serializes and deserializes via `strum` to the same canonical string used today.
- [ ] `intent_name_round_trip` — every generated `Intent` variant round-trips.
- [ ] `all_variants_have_unique_names` — `VariantNames` contains no duplicates.

### Layer 2 — Event Handling
- [ ] `keybinding_lookup_still_finds_quit` — loading `"q"` -> `Event::Quit` still works after removing `EVENT_NAMES`.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/event/names.rs`
- `crates/runie-core/src/event/name.rs`
- `crates/runie-core/src/event/intent_impl.rs`
- `crates/runie-core/src/event/kind/mod.rs`
- `crates/runie-core/src/event/kind/name.rs` (if any)
- `crates/runie-core/src/keybindings/mod.rs`
- callers that iterate or match on event names

## Notes

- Do this after the `Event` taxonomy annotation work in `collapse-event-intent-kind-taxonomies`; the annotation attributes can feed both the generated taxonomies and `strum` derives.
- `strum` cannot express parameterized variants like `Input(char)` directly; keep a tiny hand-written `FromStr`/`Display` shim for `Input:<char>` and document it.
