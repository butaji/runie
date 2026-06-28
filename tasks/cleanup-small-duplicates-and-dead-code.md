# Clean up small duplicates and dead code

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: collapse-actor-handles-to-typed-map, expand-leader-start-for-tui-and-cli, replace-legacy-tool-parsers-with-thin-shim, narrow-runie-core-public-api, route-cli-config-through-configactor
**Blocks**: none

## Description

The codebase has accumulated several small duplicates, repeated registries, and stale `#[allow(dead_code)]` attributes. This task cleans them up in a series of small, safe commits **after** the larger architectural tasks are stable:

- Consolidate skill-hook logic duplicated between `runie-agent/src/turn/tools.rs` and `runie-agent/src/tool_runner.rs`.
- Unify the built-in tool registry repeated across `runie-agent/src/tool/mod.rs`, `runie-agent/src/tool_runner.rs`, and `runie-agent/src/headless/mod.rs`.
- Share TUI render helpers across test modules under `crates/runie-tui/src/tests/`.
- Convert justified `#[allow(dead_code)]` attributes to `#[cfg(test)]` gates or documented `pub(crate)` items.

The following items are **explicitly out of scope** because they are already resolved or unsafe to change:

- `DynProvider` is an intentional backward-compatibility wrapper used by tests and downstream crates; do not unify it.
- `now()` is already defined once in `runie-protocol` and re-exported by `runie-core`; no duplication remains.
- Manual `PartialEq`/`Clone`/`Default` impls in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` are intentional (cache fields, non-trivial defaults); do not replace them with derives.

## Acceptance Criteria

- [ ] Skill hook logic is consolidated into a single helper called from both `turn/tools.rs` and `tool_runner.rs`.
- [ ] Built-in tool names/schema/dispatch are defined in exactly one registry and referenced by `tool/mod.rs`, `tool_runner.rs`, and `headless/mod.rs`.
- [ ] TUI render helpers (`render_content`, `render_chat`, etc.) are moved to a shared test helper module and imported by the test modules that previously duplicated them.
- [ ] Every remaining `#[allow(dead_code)]` in the listed files is either removed because the item is used, converted to `#[cfg(test)]`, or replaced with a documented `pub(crate)` justification.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `builtin_tools_registered_once` — verifies that the built-in tool registry is defined in exactly one place and that `tool_runner`, `headless`, and the public `tool` module all reference that single registry.
- [ ] `manual_impls_left_intact` — asserts that the manual `PartialEq`/`Clone`/`Default` impls in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` are still present and still pass their existing tests.

### Layer 2 — Event Handling
- [ ] N/A — this task focuses on code deduplication and cleanup rather than event routing.

### Layer 3 — Rendering
- [ ] `render_helpers_shared` — adds a shared `test_helpers` module in `runie-tui` and updates at least two previously duplicated test modules to import helpers from it, confirming identical rendered output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `skill_hook_unified_still_invokes_hooks` — runs a mock-tool turn that triggers skill hooks and verifies the unified hook helper still dispatches correctly from both the turn path and the tool-runner path.
- [ ] `dead_code_allows_justified` — runs `cargo check --workspace` and asserts that no new `dead_code` warnings are introduced and that every remaining allow is documented.

## Files touched

- `crates/runie-agent/src/turn/tools.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/tool/mod.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-tui/src/tests/*.rs` (render helper consolidation)
- `crates/runie-tui/src/tests/helpers.rs` or `crates/runie-tui/src/test_helpers.rs` (new shared test helpers)
- `crates/runie-core/src/keybindings/mod.rs`
- `crates/runie-core/src/model/view_cache.rs`
- `crates/runie-core/src/actors/input/mod.rs`
- `crates/runie-core/src/actors/completion/ractor_completion.rs`

## Notes

- Each cleanup item should be its own commit; the task is intentionally a grab-bag of small safe refactors.
- Do not move helpers to `runie-io` or `runie-domain`; those crates were deleted as empty facades. Use `runie-protocol` for shared protocol-level helpers or create a new tiny utility crate if necessary.
- Rejected alternative: leaving the duplication in place to minimize churn. The repeated registry and hook logic already causes inconsistent updates and increases review burden.
- Out of scope: `DynProvider`, `now()` (already resolved), manual derives with intentional semantics, large refactorings of the provider factory, the agent turn state machine, or the TUI widget tree.
