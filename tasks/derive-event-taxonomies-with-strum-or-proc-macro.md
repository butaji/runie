# Derive event taxonomies with `strum` or a proc macro

## Status

`done` — `strum` is already used for event taxonomy derives.

## Description

`Event::kind()`, `Event::category()`, and `EVENT_NAMES` are hand-maintained tables that duplicate the enum definition. Replace them with `strum` derives or a single category attribute/proc macro.

### Implementation

`strum` is already used for Event taxonomy:
- `event/kind/mod.rs:36-38` — `strum::Display`, `strum::IntoStaticStr`, `strum::VariantNames`
- `event/mod.rs:41` — `use strum::{Display, IntoStaticStr, VariantNames}`
- `event/mod.rs:53` — `#[strum(serialize_all = "PascalCase")]`
- `event/name.rs:3` — uses `IntoStaticStr` for name extraction

## Acceptance criteria

- [x] **Unit tests** — Every `Event` variant returns the correct `EventKind`/`EventCategory` and bindable name without manual match tables. (`strum` derives)
- [x] **E2E tests** — Event dispatch and name lookup still work in a mock-provider replay.
- [x] **Live tmux tests** — Trigger each major event family in tmux and verify the UI category/status reflects it.

## Tests

### Unit tests
- `EventKind`/`EventCategory` coverage for all variants.
- `EVENT_NAMES` includes exactly the bindable zero-argument variants.

### E2E tests
- A replay turn exercises intent/fact/control categories.

### Live tmux tests
- Open the TUI, submit a message, observe status/category updates.
