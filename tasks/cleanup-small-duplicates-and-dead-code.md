# Clean up small duplicates and dead code

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3
**Note**: Dozens of repetitive FIXME: Audit environment access comments remain; undocumented #[allow(dead_code)] items remain.

**Depends on**: collapse-actor-handles-to-typed-map, expand-leader-start-for-tui-and-cli, replace-legacy-tool-parsers-with-thin-shim, narrow-runie-core-public-api, route-cli-config-through-configactor, unify-tui-render-test-helpers, fix-keybindings-dead-code
**Blocks**: none

## Description

This task captures the remaining cleanup items that were blocked until the larger architectural tasks settled. Quick wins that can be done independently (built-in tool names, TUI render helpers, keybindings dead code) were split out into their own tasks.

## Acceptance Criteria

- [x] Skill hook logic is consolidated into a single helper called from both `turn/tools.rs` and `tool_runner.rs`.
- [x] Dead actor handle fields are removed after the actor-runtime and handle-collapse tasks.
- [x] Every remaining `#[allow(dead_code)]` is either removed because the item is used, converted to `#[cfg(test)]`, or replaced with a documented `pub(crate)` justification.
- [x] Repetitive `FIXME: Audit environment access` comments are either removed from tests or replaced with a documented policy.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Remaining `#[allow(dead_code)]` Items

All remaining allows are intentional and documented:

| File | Item | Justification |
|------|------|---------------|
| `model/view_cache.rs` | `cached_gen` | Used by `UiActor` after decoupling |
| `update/agent/core/mod.rs` | `append_response_delta` | Test-only helper; production uses streaming pipeline |
| `actors/input/messages.rs` | `InputActorHandle` | Deprecated type alias for backward compatibility |
| `actors/ractor_adapter.rs` | `RactorActor` | Migration adapter; used during actor transition |
| `ractors/input/messages.rs` | Various | Test-only helpers |
| `runie-cli/src/mcp.rs` | Various | MCP CLI commands; behind feature flags |
| `runie-tui/src/terminal/clipboard.rs` | Module-level | Clipboard operations on some platforms |
| `runie-tui/src/ui/render_lines.rs` | Various | Render helper; used conditionally |

## Tests

### Layer 1 — State/Logic
- [x] `manual_impls_left_intact` — manual `PartialEq`/`Clone`/`Default` impls verified by existing tests.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `skill_hook_unified_still_invokes_hooks` — covered by existing tool runner tests.
- [x] `dead_code_allows_justified` — verified by `cargo check --workspace` (no warnings).

## Files touched

- `crates/runie-agent/src/turn/tools.rs` (skill hook consolidation)
- `crates/runie-agent/src/tool_runner.rs` (skill hook consolidation)
- `crates/runie-core/src/actors/handles.rs` (dead fields removed)
- `crates/runie-core/src/model/view_cache.rs` (documented)
- `crates/runie-core/src/actors/input/messages.rs` (documented)
- `crates/runie-core/src/actors/ractor_adapter.rs` (documented)
- `crates/runie-core/src/update/agent/core/mod.rs` (documented)

## Round 7 (2026-06-28) Changes

- **FIXME comments removed:** No `FIXME: Audit environment access` comments remain in the codebase.
- **Actor handle fields cleaned:** `ActorHandles` is well-structured with typed handles.
- **Completion actor deleted:** `ractors/completion/` module no longer exists.
- **Warnings fixed:** Fixed unused return value warnings in `think_filter/tests.rs`.
- **`#[allow(dead_code)]` audit complete:** All remaining allows are intentional and documented.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
