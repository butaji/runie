# Derive event taxonomies with `strum` or a proc macro

## Status

`todo`

## Description

`Event::kind()`, `Event::category()`, and `EVENT_NAMES` are hand-maintained tables that duplicate the enum definition. Replace them with `strum` derives or a single category attribute/proc macro.

## Acceptance criteria

1. **Unit tests** — Every `Event` variant returns the correct `EventKind`/`EventCategory` and bindable name without manual match tables.
2. **E2E tests** — Event dispatch and name lookup still work in a mock-provider replay.
3. **Live tmux tests** — Trigger each major event family in tmux and verify the UI category/status reflects it.

## Tests

### Unit tests
- `EventKind`/`EventCategory` coverage for all variants.
- `EVENT_NAMES` includes exactly the bindable zero-argument variants.

### E2E tests
- A replay turn exercises intent/fact/control categories.

### Live tmux tests
- Open the TUI, submit a message, observe status/category updates.
