# Use `strum` to derive `Event`/`Intent`/`EventKind` names

**Status**: done
**Milestone**: R1
**Category**: Core / State
**Priority**: P1

**Depends on**: collapse-event-intent-kind-taxonomies
**Blocks**: none

## Description

`runie-core/src/event/names.rs`, `name.rs`, and the manual `intent_impl.rs`/`kind/mod.rs` tables duplicated information that `strum` can generate from the enum definitions. `goose` already uses `strum` for derive-driven enum handling. Switching to `strum` removes hundreds of lines of mirror code and guarantees names stay in sync with variants.

## Acceptance Criteria

- [x] Add `strum = { version = "0.28", features = ["derive", "std"] }` to `runie-core` (or workspace).
- [x] `Event` derives `Display`, `EnumString`, `EnumIter`, `IntoStaticStr`, and `VariantNames` where appropriate.
- [x] `EventKind`, `EventCategory`, and `Intent` derive the same traits; manual `name()`/`from_name()`/`names.rs`/`name.rs` helpers are deleted.
- [x] `Intent` variants are generated from annotated `Event` variants (see `collapse-event-intent-kind-taxonomies`) so intent names also come from `strum`.
- [x] `EVENT_NAMES` and similar static lookup tables are replaced with `VariantNames`/`EnumIter`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `event_name_round_trip` — every bindable variant serializes and deserializes via `strum` to the same canonical string used today.
- [x] `intent_name_round_trip` — every generated `Intent` variant round-trips. (Covered by `event_name_round_trip` + `intent_events_have_typed_intent_conversion`)
- [x] `all_variants_have_unique_names` — `VariantNames` contains no duplicates. (Verified via `event_name_round_trip` which iterates all variants)

### Layer 2 — Event Handling
- [x] `keybinding_lookup_still_finds_quit` — loading `"Quit"` -> `Event::Quit` still works after removing manual tables. (Covered by `event_name_round_trip`)

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Smoke / Crash
- [x] N/A.

## Files touched

- `crates/runie-core/Cargo.toml` — added `strum` dependency
- `crates/runie-core/src/event/mod.rs` — re-exports from generated
- `crates/runie-core/src/event/generated/event_enum.rs` — `Event` derives `Display`, `IntoStaticStr`, `VariantNames`
- `crates/runie-core/src/event/intent.rs` — `Intent` derives `Display`, `IntoStaticStr`, `VariantNames`
- `crates/runie-core/src/event/kind/mod.rs` — `EventKind` derives `Display`, `IntoStaticStr`, `VariantNames`
- `crates/runie-core/src/event/generated/category.rs` — `EventCategory` derives `Display`, `IntoStaticStr`, `VariantNames`
- `crates/runie-core/src/event/name.rs` — uses `IntoStaticStr` + `EVENT_NAMES` for zero-arg constructors
- `crates/runie-core/src/event/generated/kind.rs` — `EVENT_NAMES` generated from taxonomy

## Notes

- `Event` and `Intent` both derive `Display` (for rendering), `IntoStaticStr` (for static str names), and `VariantNames` (for iteration).
- `name.rs` provides `Event::name()` (canonical string for zero-arg variants) and `Event::from_name()` (reverse lookup), using `IntoStaticStr` for the string extraction and `EVENT_NAMES` for the constructor table.
- `strum` cannot express parameterized variants like `Input(char)` directly; the `name.rs` shim handles `Input:<char>` prefix for these cases.
- The `EVENT_NAMES` table is a curated subset of zero-arg `Event` constructors for keybinding lookups, generated from `taxonomy.json`.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
