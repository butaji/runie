# Unify Form Submit Handlers

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

8 nearly identical `*_submit` functions in `commands/dsl/handlers/session/mod.rs:76-116` that extract a field from a HashMap and construct a CommandEvent. These follow an identical pattern:

```rust
fn save_submit(values: &HashMap<String, String>) -> Event {
    CommandEvent::RunSaveCommand { name: values.get("name").cloned().unwrap_or_default() }
}
```

Replace with a `form_submit!` macro or a helper function.

## Acceptance Criteria

- [ ] Extract `form_submit!` macro or `fn make_submit(cmd: CommandEvent, key: &str)` helper
- [ ] Replace all 8 submit handlers with the new abstraction
- [ ] `cargo test --workspace` succeeds
- [ ] ~70 LOC reduced

## Tests

Layer 1 only — pure function refactoring, no state or UI changes.

### Layer 1 — State/Logic
- [ ] All existing submit tests still pass (verify behavior unchanged)

### Layer 2 — Event Handling
- [ ] N/A (no event logic changes)

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] `scripts/smoke-tmux.sh` passes

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`

## Notes

Consider whether `get_field(values, key)` helper (separate task) would help here too.
