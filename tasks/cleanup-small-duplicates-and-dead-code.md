# Clean up small duplicates and dead code

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

The codebase has accumulated several small duplicates, repeated registries, manual trait implementations that could be derived, and stale `#[allow(dead_code)]` attributes. This task cleans them up in a single pass (or a series of small commits): unify the `DynProvider` and `BuiltProvider` provider wrappers; deduplicate the `now()` timestamp helper; consolidate skill-hook logic and the built-in tool registry; share TUI render helpers across test modules; remove dead-code allows; and replace hand-written `PartialEq`, `Clone`, and `Default` impls with derives where possible.

## Acceptance Criteria

- [ ] Unify `DynProvider` in `runie-provider/src/lib.rs` (lines 51–106) with `BuiltProvider` in `runie-core/src/actors/provider/factory.rs`; keep the better name/location and delete the duplicate.
- [ ] Deduplicate the identical `now()` helper in `runie-protocol/src/message/mod.rs` (lines 110–115) and `runie-core/src/message/mod.rs` (lines 13–18); move it to a single shared location.
- [ ] Consolidate skill hook logic duplicated between `runie-agent/src/turn/tools.rs` (lines 80–130) and `runie-agent/src/tool_runner.rs` (lines 193–255) into one helper.
- [ ] Unify the built-in tool registry repeated across `runie-agent/src/tool/mod.rs`, `runie-agent/src/tool_runner.rs`, and `runie-agent/src/headless/mod.rs` into a single source of truth.
- [ ] Share TUI render helpers (`render_content`, `render_chat`, etc.) across the ~10 test modules under `crates/runie-tui/src/tests/` instead of copy-pasting them.
- [ ] Remove `#[allow(dead_code)]` from `to_lines_internal`, `parse_key_combo`, `ViewCache::cached_gen`, `append_response_delta`, the `InputActorHandle` alias, and the `RactorCompletionActor` re-export; either use the item, delete it, or leave a documented `pub(crate)` justification.
- [ ] Replace manual `PartialEq`, `Clone`, and `Default` implementations in `session/tree.rs`, `model/state/view.rs`, and `config/mod.rs` with derived implementations where semantics match.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_duplicate_now_helper` — asserts that only one `now()` definition exists in the workspace and that both protocol and core message modules delegate to it.
- [ ] `builtin_tools_registered_once` — verifies that the built-in tool registry is defined in exactly one place and that `tool_runner`, `headless`, and the public `tool` module all reference that single registry.
- [ ] `manual_impls_replaced_with_derives` — compiles a test that exercises `PartialEq`, `Clone`, and `Default` on the affected types and asserts equivalent behavior after switching to derives.

### Layer 2 — Event Handling
- [ ] N/A — this task focuses on code deduplication and cleanup rather than event routing.

### Layer 3 — Rendering
- [ ] `render_helpers_shared` — adds a shared `test_helpers` module in `runie-tui` and updates at least two previously duplicated test modules to import helpers from it, confirming identical rendered output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `skill_hook_unified_still_invokes_hooks` — runs a mock-tool turn that triggers skill hooks and verifies the unified hook helper still dispatches correctly from both the turn path and the tool-runner path.
- [ ] `dead_code_allows_removed` — runs `cargo check --workspace` programmatically and asserts no `dead_code` allows remain for the listed items.

## Files touched

- `crates/runie-provider/src/lib.rs`
- `crates/runie-core/src/actors/provider/factory.rs`
- `crates/runie-protocol/src/message/mod.rs`
- `runie-core/src/message/mod.rs`
- `crates/runie-agent/src/turn/tools.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/tool/mod.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-tui/src/tests/*.rs` (render helper consolidation)
- `crates/runie-tui/src/tests/helpers.rs` or `crates/runie-tui/src/test_helpers.rs` (new shared test helpers)
- `crates/runie-core/src/session/tree.rs`
- `crates/runie-core/src/model/state/view.rs`
- `crates/runie-core/src/config/mod.rs`
- Files containing the `#[allow(dead_code)]` items listed above.

## Notes

- Each cleanup item can be its own commit; the task is intentionally a grab-bag of small safe refactors.
- When unifying `DynProvider` / `BuiltProvider`, prefer the version that depends on fewer crates and exposes the clearest trait bounds.
- For the shared `now()` helper, `runie-protocol` is the lower-level crate; if `runie-core` can depend on it, move the helper there. Otherwise create a tiny helper module in `runie-io` or a new `runie-time` utility crate.
- Rejected alternative: leaving the duplication in place to minimize churn. The repeated code already causes inconsistent updates and increases review burden.
- Out of scope: large refactorings of the provider factory, the agent turn state machine, or the TUI widget tree. Only the listed duplicates and dead code.
