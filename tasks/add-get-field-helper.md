# Add `get_field()` Helper for Form Values

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The pattern `values.get("key").cloned().unwrap_or_default()` appears 16 times across handlers and tests. Add a single helper:

```rust
pub fn get_field(values: &HashMap<String, String>, key: &str) -> String {
    values.get(key).cloned().unwrap_or_default()
}
```

## Acceptance Criteria

- [ ] Add `get_field()` to `dialog/dsl/form.rs` or a shared utils module
- [ ] Replace all 16 occurrences
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] `test_get_field_returns_value` — verifies existing key returns value
- [ ] `test_get_field_returns_default` — verifies missing key returns empty string

### Layer 2 — Event Handling
- [ ] Existing form dialog tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] N/A

## Files touched

- `crates/runie-core/src/dialog/dsl/form.rs` (add helper)
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`
- `crates/runie-core/src/commands/dsl/handlers/builder.rs`
- `crates/runie-core/src/commands/dsl/handlers/flow.rs`
- `crates/runie-core/src/commands/dsl/handlers/tests/form_dialog.rs`

## Notes

Low effort, high impact. This helper is used in both production code and tests.
