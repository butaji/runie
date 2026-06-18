# Agent Turn Lifecycle Scan

**Focus area:** agent-turn-lifecycle  
**Workspace:** `/Users/admin/Code/GitHub/runie-dev`  
**Scanned:** `crates/runie-agent`, `crates/runie-core` (commands, dialog, update, state, login flow, orchestrator), `crates/runie-tui`, `crates/runie-print`, `crates/runie-json`, `crates/runie-provider`  
**Date:** 2026-06-18

## Summary

The agent turn lifecycle is mostly solid after the recent `AgentEvent::Error` reset work, but several concrete gaps remain around **config propagation**, **error-path state cleanup**, and **unimplemented subagent/orchestrator paths**. The most impactful issue is that the TUI's agent loop captures a one-time snapshot of the saved config at startup; provider credentials added through the onboarding/login flow are invisible to the agent loop until the app is restarted. A handful of error paths also leave turn state partially reset (streaming buffer, queued messages) and do not emit the expected `Done` event, which can strand user input.

| Severity | Count |
|----------|-------|
| P1       | 2     |
| P2       | 7     |
| P3       | 4     |

---

## P1 Findings

### 1. TUI agent loop uses a stale config snapshot
- **File:** `crates/runie-tui/src/main.rs`
- **Lines:** 157–159, 230–236, 248
- **Severity:** P1
- **Description:** `spawn_background_tasks` loads `Config::load(...)` once into `provider_config` and passes it to `agent_loop`. The loop never reloads this config. When the user completes the login flow and saves a new provider/API key, the runtime `ConfigState` is updated, but the agent loop continues building providers from the old snapshot. The next user message will fail with `MissingApiKey` even though the key was just saved.
- **Suggested fix:** Either pass a `Arc<tokio::sync::RwLock<Config>>`/`watch::Receiver<Config>` to `agent_loop` and reload it on `ModelConfigEvent::ReloadAll` / `SwitchModel`, or have `run_single_turn` load the latest config from disk on each turn (slower but simpler). Add a Layer-2 test that simulates saving a provider and then submitting a message, asserting the agent loop sees the new key.

### 2. `AgentEvent::Error` does not reset the streaming buffer or deliver queued messages
- **File:** `crates/runie-core/src/update/agent/core.rs`
- **Lines:** 331–362
- **Severity:** P1
- **Description:** `add_error` resets `streaming`, `turn_active`, `current_request_id`, `inflight`, timers, and action flags, but it does **not** reset `streaming_buffer`. It also does **not** call `deliver_queued()` / `maybe_end_streaming()` the way `finish_turn` does. After a provider error, any queued follow-ups/steering messages remain stuck in `agent.message_queue`; the user must manually abort or type again.
- **Suggested fix:** In `add_error`, call `self.agent.streaming_buffer.reset()` and then call `self.deliver_queued()` and `self.maybe_end_streaming()` after clearing the turn flags. Add a Layer-2 test that queues a message during a streaming error and asserts it is delivered once `Error` is handled.

---

## P2 Findings

### 3. `run_agent_turn` error return path does not emit `Done`
- **File:** `crates/runie-agent/src/turn.rs`
- **Lines:** 161–189, 194–233, 240–271
- **Severity:** P2
- **Description:** When `stream_response` encounters an `LLMEvent::Error`, it returns `Err(anyhow!(...))`. This error propagates out of `run_agent_iteration` and `run_iterations`, and `run_agent_turn_with_skills` returns it without emitting `AgentEvent::Done`. The TUI's `run_single_turn` catches the error and publishes `AgentEvent::Error`, but also omits `Done`. The lifecycle is therefore `Thinking → Error` with no terminal `Done`, so any code waiting on `Done` (e.g., queue drain, session persistence) never fires.
- **Suggested fix:** Emit `AgentEvent::Done { id }` in the TUI's `run_single_turn` error branch before returning, or change `run_agent_turn` to emit `Error`+`Done` internally for any error and return `Ok(())`. Add a Layer-2/3 test asserting `Done` follows `Error`.

### 4. `/spawn` command emits `SpawnAgent` but the event is a no-op at runtime
- **File:** `crates/runie-core/src/update/system.rs`
- **Lines:** 306–307
- **Severity:** P2
- **Description:** `handle_spawn` emits `ControlEvent::SpawnAgent { prompt }`, but the `ControlEvent` dispatcher matches `ControlEvent::SpawnAgent { .. }` as an empty arm. No subagent is actually executed; the command appears to do nothing. This is a user-facing incomplete scenario.
- **Suggested fix:** Either wire `SpawnAgent` to the existing `runie_agent::run_subagent_with_config` async path (inheriting current provider/model/config), or remove the command until Team mode subagent execution is implemented. If kept, add a Layer-4 smoke test verifying a `/spawn` command produces a subagent response.

### 5. Permission "Ask" is always denied in the TUI
- **File:** `crates/runie-agent/src/emit_approval_sink.rs`
- **Lines:** 24–38
- **Severity:** P2
- **Description:** `EmitApprovalSink::ask` emits a `PermissionRequest` event but immediately returns `PermissionAction::Deny`. The comment acknowledges the response channel is not wired. Any tool that reaches the approval sink (e.g., write operations when policy returns `Ask`) is silently blocked.
- **Suggested fix:** Implement a oneshot channel handshake: emit `PermissionRequest { request_id, tool, input }`, await a `PermissionResponse` event (or internal channel), and return the user's choice. Add Layer-2 tests for allow/deny and timeout behaviors.

### 6. `SubagentActor` does not execute tasks
- **File:** `crates/runie-core/src/actors/subagent.rs`
- **Lines:** 199–227, 239–268
- **Severity:** P2
- **Description:** The actor's `run_body` only emits `Started` and loops over `SubagentCommand`, ignoring `Run` except to publish a status change. No actual LLM turn runs, no output is collected, and `Completed`/`Failed` are never emitted by normal execution. Team mode subagents are non-functional.
- **Suggested fix:** Implement the `Run` handler to build a subagent command and call `runie_agent::run_subagent_with_config` with the inherited provider/model/config, then publish `Completed`/`Failed` with the result. Respect `Cancel` by dropping the future.

### 7. Public `run_subagent` wrapper uses an empty config
- **File:** `crates/runie-agent/src/subagent.rs`
- **Lines:** 33–55
- **Severity:** P2
- **Description:** `run_subagent` calls `run_subagent_with_config(..., &Config::default())`. Because the default config has no `model_providers`, any real provider will fail with `MissingApiKey`. This is a regression trap: callers that migrate from `run_subagent` to the new config-aware API are safe, but any new caller of the old API will break.
- **Suggested fix:** Deprecate `run_subagent` or make it load `Config::load(None)` internally (matching `runie-print`/`runie-json`). Update tests to prefer `run_subagent_with_config`.

### 8. `switch_provider` ignores saved config
- **File:** `crates/runie-provider/src/lib.rs`
- **Lines:** 239–246
- **Severity:** P2
- **Description:** `DynProvider::switch_provider` rebuilds the provider with `build_dyn_provider(key, model, None)`, i.e., env-only credentials. If this function is ever used after onboarding, it will discard saved API keys.
- **Suggested fix:** Change the signature to accept a `&Config` and pass it to `build_dyn_provider`. Add a test that switches a provider whose key exists only in config.

### 9. Stale async validation events after login flow cancellation
- **File:** `crates/runie-core/src/update/login_flow.rs`
- **Lines:** 176–200, 296–301
- **Severity:** P2
- **Description:** `login_flow_validation_done` and `login_flow_models_fetched` early-return if `flow.step != LoginStep::Validating`. However, the async validation task itself is not cancelled when the user presses Cancel/Esc. A late-arriving `ModelsFetched` event after cancellation is dropped (safe), but there is no mechanism to abort the in-flight HTTP request, wasting resources and potentially publishing a transient after the dialog closed.
- **Suggested fix:** Store an `AbortHandle` for the validation task in `LoginFlowState` and abort it in `login_flow_cancel`. Drop events whose `request_id`/counter no longer matches the current flow.

---

## P3 Findings

### 10. File-length linter limits are at risk
- **Files:**
  - `crates/runie-agent/src/turn.rs` — **500 lines** (exactly at limit)
  - `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — **470 lines**
  - `crates/runie-core/src/update/login_flow.rs` — **460 lines**
  - `crates/runie-core/src/update/system.rs` — **469 lines**
  - `crates/runie-core/src/orchestrator_actor/mod.rs` — **424 lines**
- **Severity:** P3
- **Description:** Several core files are within 10–40 lines of the 500-line hard cap enforced by `crates/runie-core/build.rs`. Adding error-handling cleanup, config plumbing, or subagent wiring to these files is likely to trigger a build failure.
- **Suggested fix:** Before landing any new logic in these files, split them:
  - `turn.rs` → extract `execute_tools`/`execute_single_tool` into `turn/execute.rs`
  - `session/mod.rs` → move session I/O helpers to `session/io.rs` (already partially there)
  - `login_flow.rs` → move panel-stack helpers to `update/login_flow/stack.rs`
  - `system.rs` → move control handlers to `update/control.rs`

### 11. `OrchestratorActor` Team mode is not implemented
- **File:** `crates/runie-core/src/orchestrator_actor/mod.rs`
- **Lines:** 268–309
- **Severity:** P3
- **Description:** `handle_start_request` and `handle_user_answer` immediately publish `PlanningFailed` with "Team mode is not yet implemented". This is intentional scaffolding but represents an incomplete user-facing feature.
- **Suggested fix:** Complete the planner integration or disable `/team` and `/workflow` in the command registry until ready, with a clear user message.

### 12. `handle_reload` does not refresh the agent loop's provider config
- **File:** `crates/runie-core/src/commands/dsl/handlers/system.rs`
- **Lines:** 172–176
- **Severity:** P3
- **Description:** `/reload` reloads keybindings and emits `ReloadAll`, which reloads theme/skills/prompts, but it does not communicate the new provider config to `agent_loop` because of the stale snapshot from finding #1. Even after `/reload`, newly saved keys are unavailable.
- **Suggested fix:** Fix #1; then `/reload` automatically benefits because the agent loop reads live config.

### 13. Runtime `ConfigState` and saved `Config` can diverge on `/new`
- **File:** `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- **Lines:** 309–324
- **Severity:** P3
- **Description:** `handle_new` resets `current_provider`/`current_model` to `config_provider`/`config_model`, but those runtime fields are only initialized from the saved config at startup. If the user saved a new default model via login flow and `config_provider` was not updated, `/new` reverts to an outdated selection.
- **Suggested fix:** After a successful login flow save, also update `state.config.config_provider` and `state.config.config_model` (in addition to `current_*`). Add a test verifying `/new` restores the most recently saved provider/model.

---

## Recommended Priority Order

1. **P1 #1** — Make the TUI agent loop read live config (blocks onboarding from actually working for new users).
2. **P1 #2** — Complete `AgentEvent::Error` cleanup (streaming buffer + queued delivery).
3. **P2 #3** — Ensure `Done` is always emitted to close the turn lifecycle.
4. **P2 #4 / #6** — Decide whether `/spawn`/Team mode is shipped; if so, implement the actor execution path.
5. **P2 #5** — Wire the permission approval response channel.
6. **P2 #7 / #8** — Remove/deprecate env-only subagent/provider APIs.
7. **P3 #10** — Split files approaching the linter limit before adding the above fixes.

---

## Test Gaps to Close

- No Layer-2 test that `AgentEvent::Error` resets `streaming_buffer`.
- No Layer-2 test that queued messages are delivered after an error.
- No Layer-2 test that `Done` follows `Error`.
- No Layer-2/3 test that the TUI agent loop uses an updated config after login flow save.
- No Layer-4 smoke test for `/spawn` producing output.
- No Layer-2 test for permission approval allow/deny flow.
