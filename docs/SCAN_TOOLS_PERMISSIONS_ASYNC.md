# Scan: tools-permissions-async

**Date:** 2026-06-18
**Focus area:** tool execution, permission gating, async agent turns, provider construction from saved config, and turn-state reset.

## Summary

The recent migration to read API keys from saved config (`build_provider_with_warning_with_config`, `DynProvider::new_with_config`, `run_subagent_with_config`) is mostly complete, but several production paths still ignore saved credentials or use a stale snapshot of the config file. The permission system has an unimplemented interactive approval path (`EmitApprovalSink`), so any policy that returns `Ask` is silently denied. Turn-state reset on `AgentEvent::Error` is now covered, but the surrounding async plumbing and some skill/tool edge cases still have gaps. Several focus files and functions are already at the enforced 500/40 line limits.

- **P1:** 1 finding
- **P2:** 6 findings
- **P3:** 5 findings

---

## Findings

### P1 — TUI agent_loop uses a stale config snapshot, breaking the first turn after onboarding

- **File:** `crates/runie-tui/src/main.rs`
- **Lines:** 157-159 (snapshot captured at startup), 240-275 (`run_single_turn` uses it)
- **Severity:** P1
- **Description:** `agent_loop` loads `Config` once in `spawn_background_tasks` and passes that immutable snapshot to every turn. After the user completes `/login` and saves an API key to `~/.runie/config.toml`, the snapshot still reflects the empty config from before onboarding, so `build_provider_with_warning_with_config` returns `MissingApiKey` on the first real message. The config watcher also does not emit events for credential-only changes, so the snapshot is never refreshed.
- **Suggested fix:** Reload the config from disk inside `run_single_turn` before building the provider, or give `agent_loop` a `watch::Receiver<Config>` that is updated whenever the file changes. Extend `ConfigChange` to include credential changes and route them to the agent loop.

### P2 — PermissionRequest events are emitted but never handled

- **File:** `crates/runie-agent/src/emit_approval_sink.rs`
- **Lines:** 25-38
- **Related:** `crates/runie-tui/src/main.rs:262-267` (gate uses this sink); `crates/runie-core/src/update/dispatch.rs` (no `PermissionRequest` handler)
- **Severity:** P2
- **Description:** `EmitApprovalSink::ask` emits `Event::PermissionRequest` and then returns `PermissionAction::Deny`. There is no dispatcher branch, dialog, or response channel for `PermissionRequest` / `PermissionResponse`, so tools matching a policy that returns `Ask` (e.g. `FileAccessAsk`) are silently blocked. The module doc comment explicitly calls this a “future iteration.”
- **Suggested fix:** Either implement a real permission-request dialog with a `PermissionResponse` handler and a oneshot/channel wired back into the sink, or remove the misleading event and emit a transient warning when a tool is blocked by an Ask policy.

### P2 — `switch_provider` rebuilds the provider without saved config

- **File:** `crates/runie-provider/src/lib.rs`
- **Lines:** 239-246
- **Severity:** P2
- **Description:** `switch_provider` calls `build_dyn_provider(key, model, None)`, ignoring any API key or base URL saved in `config.toml`. If a caller uses this to change provider/model at runtime, it will fail unless the key is present in the environment.
- **Suggested fix:** Add a `&Config` parameter (or reload the config inside the function) and pass it to `build_dyn_provider`. If the function is unused, deprecate or remove it.

### P2 — Legacy env-only provider builders are still public

- **File:** `crates/runie-provider/src/lib.rs` lines 47-65 (`DynProvider::new`, `new_checked`) and 197-202 (`build_provider_with_warning`); `crates/runie-agent/src/lib.rs` lines 39-50 (`build_provider`, `build_provider_with_warning`)
- **Severity:** P2
- **Description:** These public APIs ignore saved config and are not used in production paths anymore. They remain a footgun for external callers or future refactors. `build_provider` also panics on misconfiguration.
- **Suggested fix:** Mark them `#[deprecated(note = "use *_with_config")]` and, where possible, gate them behind `#[cfg(test)]` so only the config-aware variants are available to production code.

### P2 — `run_subagent` wrapper ignores saved config

- **File:** `crates/runie-agent/src/subagent.rs`
- **Lines:** 33-55
- **Severity:** P2
- **Description:** The public `run_subagent` function delegates to `run_subagent_with_config` with `Config::default()`, which has no saved provider credentials. It is currently only used by tests, but its presence invites misuse.
- **Suggested fix:** Deprecate it or make it `#[cfg(test)]`, and require all production callers to use `run_subagent_with_config`.

### P2 — `Config::load` silently falls back to default on parse/migration errors

- **File:** `crates/runie-core/src/config.rs`
- **Lines:** 195-217
- **Severity:** P2
- **Description:** If `config.toml` is malformed or migration fails, the function returns `Config::default()` without surfacing an error. Saved API keys and other settings disappear silently, making subsequent provider builds fail with a confusing `MissingApiKey`.
- **Suggested fix:** Return a `Result` or at least log/emit a system message when the config file cannot be parsed or migrated, so the user knows credentials were not loaded.

### P2 — Config watcher does not notify on credential-only changes

- **File:** `crates/runie-core/src/config.rs` lines 318-337 (`classify_change`); `crates/runie-core/src/config_reload/watcher.rs` lines 57-77 (`apply_config_changes`)
- **Severity:** P2
- **Description:** `classify_change` only compares provider, model, theme, and keybindings. If the user edits `config.toml` to add or update an API key/base URL while leaving provider/model unchanged, the watcher emits no event. Runtime `ConfigState` and the agent-loop snapshot remain stale.
- **Suggested fix:** Add `ConfigChange::Credentials` and emit it when `model_providers` or `fallback_providers` differ. Have the TUI agent loop (or a central config updater) reload the saved config in response.

### P3 — Skill tool abort panics instead of failing gracefully

- **File:** `crates/runie-agent/src/turn.rs`
- **Lines:** 415-420
- **Severity:** P3
- **Description:** In `check_tool_call_before_hook`, a `ToolCallResult::Abort` triggers `panic!("Tool abort not implemented in this path")`. A misbehaving or adversarial skill can crash the agent turn.
- **Suggested fix:** Treat `Abort` as a blocked tool: emit a `ToolEnd` with `ToolStatus::Blocked` or `Error`, record a failure message, and continue or stop the turn cleanly instead of panicking.

### P3 — Headless mode treats `PermissionAction::Ask` as silent deny

- **File:** `crates/runie-agent/src/headless.rs`
- **Lines:** 153-160
- **Severity:** P3
- **Description:** `execute_tool_call` maps both `Deny` and `Ask` to a blocked tool output with no indication that the tool was blocked because of an Ask policy. In `runie-json`/`runie-print` this can be confusing because the user sees only “Permission denied.”
- **Suggested fix:** Differentiate the message (`Permission denied (ask policy)`), and document that non-interactive modes require `--yolo` for Ask policies to be approved.

### P3 — EventBus can drop live events for slow subscribers

- **File:** `crates/runie-core/src/bus.rs`
- **Lines:** 66-72
- **Severity:** P3
- **Description:** `publish` uses `broadcast::Sender::send` and discards errors with `unwrap_or(0)`. If a subscriber lags and the broadcast buffer fills, live delivery is dropped for that subscriber. A dropped `AgentEvent::Done` or `Error` can leave the UI thinking the turn is still active.
- **Suggested fix:** Increase broadcast capacity relative to replay capacity, log when `send` returns `Err`, or switch to per-subscriber channels so backpressure is explicit.

### P3 — Focus files/functions are at or near linter limits

- **Files/lines:**
  - `crates/runie-agent/src/turn.rs` — 500 lines (exact file limit)
  - `crates/runie-agent/src/turn.rs:34-73` — `run_agent_turn_with_skills` (40 lines)
  - `crates/runie-agent/src/turn.rs:194-233` — `run_agent_iteration` (40 lines)
  - `crates/runie-agent/src/headless.rs:123-162` — `execute_tool_call` (40 lines)
  - `crates/runie-core/src/update/system.rs:273-312` — `control_event` (40 lines)
  - `crates/runie-tui/src/main.rs:135-173` — `spawn_background_tasks` (39 lines)
- **Severity:** P3
- **Description:** The build script at `crates/runie-core/build.rs` enforces 500-line files and 40-line production functions. Several files/functions in the focus area are already at the limit, so any non-trivial fix must be accompanied by extraction/refactoring.
- **Suggested fix:** Refactor before adding logic: split `turn.rs` into `turn/` submodules, extract helper functions from the 40-line functions, and keep new code under the limits.

---

## Test gaps

- No test verifies that the TUI agent loop uses an up-to-date config after onboarding or after a manual `config.toml` edit.
- No test exercises the `PermissionRequest` / `PermissionResponse` flow or the UI behavior when `FileAccessAsk` returns `Ask`.
- No test guards against accidental production use of `DynProvider::new`, `build_provider_with_warning`, or `run_subagent`.
- No test covers the skill `ToolCallResult::Abort` path.
- No test verifies that a lagging `EventBus` subscriber still receives a consistent final turn state.
