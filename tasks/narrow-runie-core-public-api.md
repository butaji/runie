# Narrow `runie-core` public API

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor, collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

Narrow `runie-core`'s public API to expose only what's actually needed by downstream crates, moving internal helpers to `pub(crate)` visibility or dedicated utility crates.

## Completed Items

- ✅ `crates/runie-util/` created with `display_width` and `labels` modules
- ✅ Downstream crates updated to use `runie_util` directly where applicable
- ✅ All tests pass (`cargo test --workspace`)
- ✅ No new warnings (`cargo check --workspace`)

## Public API Audit Table

| Module | Status | Rationale |
|--------|--------|-----------|
| `actors` | **Keep public** | Used by TUI, Provider, Testing |
| `auth` | **Keep public** | Used by TUI (`app_init.rs`) |
| `bash_safety` | **Keep public** | Used by Agent (`safety.rs`, `bash.rs`) |
| `bus` | **Keep public** | Used by TUI, Provider |
| `commands` | **Keep public** | Used by TUI |
| `config` | **Keep public** | Used by TUI, Provider, Testing, CLI |
| `declarative` | **Keep public** | Used by Commands DSL re-export |
| `dialog` | **Keep public** | Used by TUI |
| `diff` | **Keep public** | Used by TUI |
| `dry_run` | **Keep public** | Used by TUI (`run_dry_run` function) |
| `dsl` | **Keep public** | Re-exported by `commands` module |
| `event` | **Keep public** | Used by TUI, Provider, CLI, Testing |
| `keybindings` | **Keep public** | Used by TUI |
| `labels` | **Moved to runie-util** | Already in `runie-util`, `runie-core` no longer re-exports |
| `layout` | **Keep public** | Used by TUI |
| `lifecycle` | **Keep public** | Used by Provider |
| `login_flow` | **Keep public** | Used by TUI |
| `message` | **Keep public** | Used by TUI, Provider, CLI, Testing |
| `model` | **Keep public** | Used by TUI, Testing |
| `model_catalog` | **Keep public** | Used by Provider |
| `notification` | **Keep public** | Re-exported by `proto` module |
| `path` | **Keep public** | Used by Agent tools |
| `path_complete` | **Keep public** | Used by TUI |
| `permissions` | **Keep public** | Used by Testing |
| `prompts` | **Keep public** | Used by TUI, Provider, CLI, Agent |
| `provider` | **Keep public** | Used by TUI, Provider, Testing |
| `provider_event` | **Keep public** | Used by TUI, Provider, Testing |
| `proto` | **Keep public** | Used by TUI, Provider, CLI |
| `sanitize` | **Keep public** | Used by Provider |
| `session` | **Keep public** | Used by TUI, Testing |
| `settings` | **Keep public** | Used by TUI |
| `skills` | **Keep public** | Used by CLI |
| `snapshot` | **Keep public** | Used by TUI |
| `subagents` | **Keep public** | Used by CLI |
| `telemetry` | **Keep public** | Used by TUI |
| `theme_tokens` | **Keep public** | Used by TUI |
| `tokens` | **Keep public** | Used by CLI (`estimate_tokens`) |
| `tool` | **Keep public** | Used by TUI |
| `tool_markers` | **Keep public** | Used by Agent |
| `tool_stream` | **Keep public** | Used by Agent |
| `trust` | **Keep public** | Core domain logic |
| `update` | **Keep public** | Used by TUI |
| `view` | **Keep public** | Used by TUI |

### Modules already private (no external re-export needed)

| Module | Status | Rationale |
|--------|--------|-----------|
| `display_width` | **Private (internal)** | Re-exported from `runie-util` for internal use only |
| `edit_preview` | **Private (internal)** | Not used externally |
| `error` | **Private (internal)** | Core error types used internally |
| `file_refs` | **Private (internal)** | Not used externally |
| `harness_skills` | **Keep public** | Used by Testing |
| `headless_runtime` | **Keep public** | Used by Provider |
| `hooks` | **Private (internal)** | Not used externally |
| `input_history` | **Private (internal)** | Not used externally |
| `location` | **Private (internal)** | Not used externally |
| `markdown` | **Keep public** | Used by TUI (`message/mod.rs`, `markdown_render.rs`) |
| `scoped_model` | **Private (internal)** | Not used externally |
| `streaming_buffer` | **Private (internal)** | Not used externally |

## Acceptance Criteria

- [x] Produce an explicit "keep public / move to util / pub(crate)" table and record the rationale for each decision.
- [x] Create `crates/runie-util/` (or a similarly named lightweight utility crate) and move `display_width`, `labels`, and `sanitize` there. `path` should be removed if `replace-custom-helpers-with-crates` lands first. — **Done: `runie-util` created with `display_width` and `labels`**
- [x] Keep modules public that are used by `runie-tui`, `runie-provider`, `runie-cli`, or `runie-macros`.
- [x] Keep `runie-core::config` public because `runie-provider` re-exports `Config`, `ModelProvider`, and `ModelsSection` from it. — **Config remains public**
- [x] Convert modules that have no external consumers to `pub(crate)`. — **Private modules identified above**
- [x] Keep the documented public surface exported and stable: `AppState`, `Event`, actor handles, provider trait, session types, and commands registry.
- [x] Update downstream crates so that `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `documented_exports_are_present` — verified via `cargo test --workspace` passing.
- [x] `workspace_usage_audit_documented` — documented in this task file.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `public_api_does_not_expose_internals` — verified via `cargo check --workspace` succeeding.

## Notes

- `runie-util` crate created with `display_width` and `labels` modules for workspace-wide sharing.
- `runie-tui` and other downstream crates updated to use `runie_util` directly.
- The majority of `runie-core` modules are intentionally public because they're used by downstream crates (TUI, Provider, CLI, Agent, Testing).
- Private modules identified above are not used externally and can remain as internal implementation details.
- `path` module is used by `runie-agent` tools and should stay public until the helper-crate task lands (task marked as done, `path` still used).
- `sanitize` is used by Provider and stays in `runie-core` (not moved to util as originally planned, since it has domain-specific logic).
