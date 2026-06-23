# Verify ConfigActor propagates write errors

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`ConfigActor` previously checked only `result.is_ok()` on the `JoinHandle`, swallowing inner file-write errors. It now uses `handle_write_result` for all four mutating helpers (`save_provider`, `remove_provider`, `set_default_model`, `set_provider_models`) and emits `Event::Error { id: "config", message }` on failure.

## Acceptance Criteria

- [ ] `cargo test --workspace` passes.
- [ ] A write failure emits `Event::Error` instead of a silent reload.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- Consider adding `config_actor_emits_error_on_failed_save` if not present.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs` — verify only.

## Implementation

No code changes needed. Verify that lines 86–134 call `self.handle_write_result(result, bus).await` and that `handle_write_result` matches:

```rust
Ok(Ok(())) => self.load_and_emit(bus).await,
Ok(Err(e)) => bus.publish(Event::Error { id: "config".into(), message: ... }),
Err(e) => bus.publish(Event::Error { id: "config".into(), message: ... }),
```

Run verification:

```bash
cargo test --workspace
```

## Notes

- If regression occurs, ensure all four mutating helpers use `handle_write_result` exactly once.
