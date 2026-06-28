# Clean up small duplicates and dead code

**Status**: partial
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: collapse-actor-handles-to-typed-map, expand-leader-start-for-tui-and-cli, replace-legacy-tool-parsers-with-thin-shim, narrow-runie-core-public-api, route-cli-config-through-configactor, unify-tui-render-test-helpers, fix-keybindings-dead-code
**Blocks**: none

## Description

This task captures the remaining cleanup items that are blocked until the larger architectural tasks settle. Quick wins that can be done independently (built-in tool names, TUI render helpers, keybindings dead code) are split out into their own tasks.

Remaining blocked items:

- **Skill hooks duplicated:** `crates/runie-agent/src/tool_runner.rs:193–255` (`execute_with_skill_hooks`, `check_before_hook`, `fire_after_hook`) and `crates/runie-agent/src/turn/tools.rs:57–130` (`skill_override_output`, `check_tool_call_before_hook`, `fire_tool_after_hook`). The two paths do the same semantic work with different return types. This is now unblocked because `BUILTIN_TOOL_NAMES` is canonical.
- **Dead actor handle fields:** after `ActorHandles` is collapsed and dead actors are deleted, remove any remaining dead fields and helper methods (`view`, `completion`, `trust`, `send_view`, `send_trust`, `send_init_read_only`, etc.).
- **Remaining `#[allow(dead_code)]` items:** review any leftover allows after the quick-win tasks and the actor/tool refactors. Known candidates from Round 5:
  - `crates/runie-core/src/model/view_cache.rs:14` (`cached_gen` — verify it is read anywhere; if not, remove).
  - `crates/runie-core/src/update/agent/core/mod.rs:134` (`append_response_delta`).
  - `crates/runie-core/src/actors/completion/ractor_completion.rs:50,102`.
- **Repetitive `FIXME: Audit environment access` comments:** 25+ identical comments in tests and production (`tool/context.rs:51`, `auth.rs:202`). Replace with a single policy note or remove the test ones.

The following items are **explicitly out of scope** because they are already resolved or unsafe to change:

- `DynProvider` is an intentional backward-compatibility wrapper used by tests and downstream crates; do not unify it.
- `now()` is already defined once in `runie-protocol` and re-exported by `runie-core`; no duplication remains.
- Manual `PartialEq`/`Clone`/`Default` impls in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` are intentional (cache fields, non-trivial defaults); do not replace them with derives.

## Acceptance Criteria

- [ ] Skill hook logic is consolidated into a single helper called from both `turn/tools.rs` and `tool_runner.rs`.
- [ ] Dead actor handle fields are removed after the actor-runtime and handle-collapse tasks.
- [ ] Every remaining `#[allow(dead_code)]` is either removed because the item is used, converted to `#[cfg(test)]`, or replaced with a documented `pub(crate)` justification.
- [ ] Repetitive `FIXME: Audit environment access` comments are either removed from tests or replaced with a documented policy.
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
- `crates/runie-core/src/update/agent/core/mod.rs`
- Files containing repetitive `FIXME: Audit environment access` comments

## Notes

- This task is intentionally the final sweep after the independent quick wins and the larger architectural changes land.
- The skill-hook consolidation is no longer blocked on tool-parser work because the canonical `BUILTIN_TOOL_NAMES` is in place.
- Do not move helpers to `runie-io` or `runie-domain`; those crates were deleted as empty facades.
- `telemetry.rs` should be revisited here: if it is only collecting events that `tracing` spans/events can model, replace it with `tracing` and delete the module.
- Any remaining custom keybinding parsing should be replaced by `crossterm` (already covered in `replace-custom-helpers-with-crates`); this task only cleans up leftover allows.
- Out of scope: `DynProvider`, `now()` (already resolved), manual derives with intentional semantics, large refactorings of the provider factory, the agent turn state machine, or the TUI widget tree.
