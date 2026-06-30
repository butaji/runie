# Remove ractor send/send_message alias

## Status

`todo`

## Context

`crates/runie-core/src/actors/ractor_adapter.rs` exposes both `send` and `send_message` as aliases for the same ractor fire-and-forget call. Multiple actor handle wrappers repeat the same alias. Removing the alias reduces API surface area.

## Goal

Pick one name (`send`) and delete `send_message` everywhere. Update call sites.

**Design impact:** No change to TUI element design or composition. Only the internal actor-handle API changes.

## Acceptance Criteria

- [ ] Remove the `send_message` method from `RactorHandle` and any actor-specific handle wrappers.
- [ ] Update all call sites to use `send`.
- [ ] Ensure no `#[allow(dead_code)]` remains for the removed alias.

## Tests

- **Layer 1 — State/Logic:** Verify `RactorHandle::send` still delivers messages to a test actor.
- **Layer 2 — Event Handling:** Send a message through an actor handle and observe the resulting fact.
- **Layer 3 — Rendering:** N/A unless a TUI path uses the alias; verify no compile errors.
- **Layer 4 — E2E:** Actor-based provider replay still passes.
- **Live tmux validation:** Start the TUI and confirm normal input/agent flow works after the rename.
