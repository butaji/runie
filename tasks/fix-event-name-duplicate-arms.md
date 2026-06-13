# Fix Duplicate / Unreachable Arms in `Event::name`

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

`crates/runie-core/src/event.rs` defines `Event::name()` with two
conflicting match arms for the same set of variants:

- Lines 526–534 return `None` for `GoToTop`, `GoToBottom`,
  `ToggleVimMode`, `MouseClick/Release/Drag/Move`, and
  `FocusGained/Lost`.
- Lines 577–585 repeat the same variants and return canonical names
  (`"GoToTop"`, `"GoToTop"`, etc.).

Because the first block returns early, the second block is unreachable
and `clippy` emits 9 `unreachable pattern` warnings. It also means
`EVENT_NAMES` advertises bindable names that `Event::name()` currently
reports as `None`.

## Acceptance Criteria

- [ ] The duplicate arms are removed so each variant is matched exactly
  once.
- [ ] `GoToTop`, `GoToBottom`, `ToggleVimMode`, `FocusGained`, and
  `FocusLost` return their canonical names from `Event::name()` (they
  are listed in `EVENT_NAMES`).
- [ ] Mouse variants (`MouseClick`, `MouseRelease`, `MouseDrag`,
  `MouseMove`) keep returning `None` (they carry coordinates and cannot
  be bound canonically) and are present only once.
- [ ] `cargo clippy --workspace -p runie-core` no longer reports
  `unreachable pattern` warnings for this function.
- [ ] A round-trip test asserts that every entry in `EVENT_NAMES` maps
  back to itself via `Event::from_name(...).unwrap().name()`.

## Tests

### Layer 1 — State/Logic
- [ ] `event_name_roundtrip` — for every `(name, ctor)` in
  `EVENT_NAMES`, `Event::from_name(name)` produces an event whose
  `name()` equals `name`.
- [ ] `mouse_events_have_no_name` — `MouseClick`, `MouseRelease`,
  `MouseDrag`, `MouseMove` return `None` from `name()`.

### Layer 2 — Event Handling
- [ ] `default_keybindings_resolve` still passes after the canonical
  names are corrected.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] Not required for this pure refactor.

## Notes

**Why this is P0:**
The unreachable warnings hide other real issues, and the mismatch
between `EVENT_NAMES` and `Event::name()` makes keybinding validation
unreliable.

**Out of scope:**
- Splitting the `Event` enum (see `event-subenums.md`).
- Changing which variants are bindable.

## Verification

```bash
cargo clippy -p runie-core 2>&1 | grep -c "unreachable pattern"
# Expected: 0

cargo test -p runie-core --lib event
```
