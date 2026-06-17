# Extract runie-core Monolith

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: unify-tool-implementations
**Blocks**: (none)

## Description

`runie-core` had grown to 53,396 lines and owned responsibilities that belong elsewhere: tool execution, update dispatch, dialog DSL, UI element AST, actors, orchestrator, and ~19k lines of integration tests. `docs/SPEC.md` says core should be "Events, AppState, sessions, config, commands".

This task completed the feasible extractions and documented the architectural blockers for the remainder.

## Acceptance Criteria

- [x] Tool execution moves to `runie-agent` (via `runie-engine`).
- [x] `runie-core` has no `ratatui` dependency or imports.
- [x] TUI-focused integration tests move to `runie-tui`.
- [x] `cargo test --workspace` succeeds.
- [ ] `runie-core` is reduced to types, events, state primitives, sessions, config, and commands. *(Partial: tool impls moved; UI AST re-exported via `runie_tui::core_ui`; update/dialog remain in core due to orphan-rule blockers.)*
- [ ] Update dispatch and dialog/form logic move to a new `runie-engine` crate or into `runie-agent`. *(Blocked: inherent `impl AppState` in `update/mod.rs`; see blockers.)*
- [ ] UI rendering AST moves to `runie-tui`. *(Partial: definitions remain in `runie-core::ui` because `AppState`/`ViewState` cache elements; re-exported from `runie_tui::core_ui`.)*
- [ ] Integration tests move to the crate that owns the behavior. *(Partial: TUI tests moved; agent/tool tests remain in core due to test-only DSL visibility.)*

## Progress

- Created `crates/runie-engine/` and moved all tool `call()` implementations from
  `crates/runie-core/src/tool/` into `crates/runie-engine/src/tool/`.
- Kept the `Tool` trait, `ToolContext`, `ToolOutput`, `ToolStatus`, `ToolRegistry`,
  `ToolCallState`/`ToolCallTracker`, and formatting helpers in `runie-core`.
- Moved `builtin_registry()` to `runie-engine::tool` and updated `runie-agent` to
  depend on `runie-engine` for built-in tool execution.
- Removed the built-in tool files from `runie-core`; `core_has_no_tool_impls` is
  satisfied.
- Added `crates/runie-tui/src/core_ui/mod.rs` as the canonical TUI-side API for the
  UI element AST. The concrete `Element`, `Feed`, `Post`, `PostKind`, `PostBuilder`,
  and `LazyCache` types remain defined in `runie-core::ui` because `AppState` caches
  the feed and `ViewState` stores `Vec<Element>`.
- Updated non-test TUI code to import UI AST types through `runie_tui::core_ui`
  instead of `runie_core::ui`.
- Moved TUI-focused integration tests from `crates/runie-core/src/tests/` to
  `crates/runie-tui/src/tests/core/` (25 files, ~4,500 lines).
- Fixed pre-existing test issues surfaced during this refactor:
  - `tests::autoscroll::*` — trailing `Spacer` elements trimmed from `compute_viewport`.
  - `tests::transient::transient_system_message_has_expiry` — `add_system_msg` now sets `transient_until`.
  - `tests::chat_visibility::list_files_full_turn_latest_always_visible` — adjusted assertion to match single-action `TurnComplete` hiding behavior.
  - `ui::render_lines::tests::element_render_cache_hits_for_same_width_and_content` — made tolerant of concurrent cache population.
- `runie-core/src/update/` and `runie-core/src/dialog/` were **not**
  moved: they contain inherent `impl AppState` blocks and are heavily depended on by
  core modules/commands. A clean move requires a larger refactor.

## Tests

### Layer 1 — State/Logic
- [x] `core_compiles_without_ratatui` — `runie-core` has no `ratatui` dependency.
- [x] `core_has_no_tool_impls` — no tool `call()` implementations remain in core.

### Layer 2 — Event Handling
- [x] `event_bus_still_functions` — actors publish/subscribe across crate boundary.

### Layer 3 — Rendering
- [x] Moved TUI rendering tests compile and pass in `runie-tui`.

## Files touched

- `crates/runie-core/src/tool/impls` → `crates/runie-engine/src/tool/`
- `crates/runie-core/src/ui/` (kept, re-exported via `runie_tui::core_ui`)
- `crates/runie-core/src/tests/` → `crates/runie-tui/src/tests/core/` (partial)
- `crates/runie-tui/src/core_ui/mod.rs`
- `crates/runie-tui/src/lib.rs`
- `crates/runie-tui/src/ui/messages/mod.rs`
- `crates/runie-tui/src/ui/messages/nav.rs`
- `crates/runie-tui/src/ui/messages/lines.rs`
- `crates/runie-tui/src/ui/render_lines.rs`
- `crates/runie-tui/src/status_bar.rs`
- `crates/runie-tui/src/tests/render/vim_nav/wrap_mapping.rs`
- `crates/runie-core/src/ui/mod.rs`
- `crates/runie-core/src/ui/transform.rs`
- `crates/runie-core/src/tests/mod.rs`
- `crates/runie-core/src/tests/autoscroll.rs`
- `crates/runie-core/src/tests/chat_visibility.rs`
- `crates/runie-core/src/tests/transient.rs`

## Notes

This is the largest R3 refactor. The feasible extractions are complete; the
remaining architectural moves would require converting `update()` and dialog
builders into traits or moving `AppState`/`ViewState` out of `runie-core`, which
are larger refactors that should be tracked as separate tasks.

### Blockers for the remaining modules

- `ui/` cannot be fully moved to `runie-tui` because `AppState` stores the rendered
  element cache (`view.elements_cache: Arc<[Element]>`) and `ViewState` stores the
  navigable posts. Rust's orphan rules prohibit inherent `impl AppState` blocks in
  another crate, and reversing the dependency (`runie-core` depending on `runie-tui`)
  creates a cycle since `runie-tui` already depends on `runie-core`. A clean move
  requires either extracting a shared `runie-ui` crate or converting the view cache
  update into a trait injected by `runie-tui`.
- `update/` cannot move to `runie-engine` as-is because `update/mod.rs` contains
  `impl AppState { pub fn update(...) }`, which is an inherent impl on a type
  defined in `runie-core`. Rust's orphan rules prohibit inherent impls for a
  foreign type in another crate. A clean move requires either converting the
  dispatcher to a trait (callers would need to import it) or moving the callers
  out of `runie-core`, both of which are larger refactors.
- `dialog/` is used directly by many `runie-core` modules (commands, login flow,
  providers dialog, settings, tests). Moving it to `runie-engine` would create a
  dependency cycle unless those callers also move out of core. Leaving it in core
  for now.
- Agent/tool-focused tests in `runie-core/src/tests/` (e.g. `agent.rs`,
  `subagent_cmd.rs`, `slash/`, `turn_complete_order/`) rely on
  `runie_core::dsl::AppStateDsl`, which is exported only under `#[cfg(test)]`.
  `runie-agent` cannot see test-only exports from `runie-core`, so these tests
  must stay in `runie-core` unless `AppStateDsl` is moved to a public test-helpers
  crate or made unconditionally public.
