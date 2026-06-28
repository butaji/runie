# Clean up small duplicates and dead code

**Status**: partial
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: collapse-actor-handles-to-typed-map, expand-leader-start-for-tui-and-cli, replace-legacy-tool-parsers-with-thin-shim, narrow-runie-core-public-api, route-cli-config-through-configactor, centralize-built-in-tool-names, unify-tui-render-test-helpers, fix-keybindings-dead-code
**Blocks**: none

## Description

This task captures the remaining cleanup items that are blocked until the larger architectural tasks settle. Quick wins that can be done independently (built-in tool names, TUI render helpers, keybindings dead code) are split out into their own tasks.

Remaining blocked items:

- **Skill hooks duplicated:** `crates/runie-agent/src/tool_runner.rs:193–255` (`execute_with_skill_hooks`, `check_before_hook`, `fire_after_hook`) and `crates/runie-agent/src/turn/tools.rs:57–130` (`skill_override_output`, `check_tool_call_before_hook`, `fire_tool_after_hook`). The two paths do the same semantic work with different return types. This is blocked until the tool-parser-shim task finalizes what "built-in tool" means in the MCP-only boundary.
- **Dead actor handle fields:** after `ActorHandles` is collapsed and dead actors are deleted, remove any remaining dead fields and helper methods.
- **Remaining `#[allow(dead_code)]` items:** review any leftover allows after the quick-win tasks and the actor/tool refactors.

The following items are **explicitly out of scope** because they are already resolved or unsafe to change:

- `DynProvider` is an intentional backward-compatibility wrapper used by tests and downstream crates; do not unify it.
- `now()` is already defined once in `runie-protocol` and re-exported by `runie-core`; no duplication remains.
- Manual `PartialEq`/`Clone`/`Default` impls in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` are intentional (cache fields, non-trivial defaults); do not replace them with derives.

## Acceptance Criteria

- [ ] Skill hook logic is consolidated into a single helper called from both `turn/tools.rs` and `tool_runner.rs`.
- [ ] Dead actor handle fields are removed after the actor-runtime and handle-collapse tasks.
- [ ] Every remaining `#[allow(dead_code)]` is either removed because the item is used, converted to `#[cfg(test)]`, or replaced with a documented `pub(crate)` justification.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `manual_impls_left_intact` — asserts that the manual `PartialEq`/`Clone`/`Default` impls in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` are still present and still pass their existing tests.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `skill_hook_unified_still_invokes_hooks` — runs a mock-tool turn that triggers skill hooks and verifies the unified hook helper still dispatches correctly from both the turn path and the tool-runner path.
- [ ] `dead_code_allows_justified` — runs `cargo check --workspace` and asserts that no new `dead_code` warnings are introduced and that every remaining allow is documented.

## Files touched

- `crates/runie-agent/src/turn/tools.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/model/view_cache.rs`
- `crates/runie-core/src/actors/input/mod.rs`
- `crates/runie-core/src/actors/completion/ractor_completion.rs`

## Notes

- This task is intentionally the final sweep after the independent quick wins and the larger architectural changes land.
- Do not move helpers to `runie-io` or `runie-domain`; those crates were deleted as empty facades.
- Out of scope: `DynProvider`, `now()` (already resolved), manual derives with intentional semantics, large refactorings of the provider factory, the agent turn state machine, or the TUI widget tree.
