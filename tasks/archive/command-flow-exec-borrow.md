# Make CommandFlow::exec Take &self

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Input / Commands
**Priority**: P0

## Description

`CommandFlow::exec` consumes `self`, so the dispatcher clones the whole flow on every slash command. None of the branches need ownership.

## Acceptance Criteria

- [ ] `CommandFlow::exec(&self, ...)`.
- [ ] Dispatcher no longer clones the flow.
- [ ] `Chain` and recursive branches work with references.
- [ ] `cargo build --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `exec_runs_handler`.
- [ ] `exec_chain_runs_both_branches`.

### Layer 2 — Event Handling
- [ ] `slash_command_dispatches_without_cloning_flow`.
