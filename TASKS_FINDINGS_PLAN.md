# Task-Derived Findings & Cleanup Plan

This document consolidates the behavioral requirements and implementation tasks that were previously scattered across two broken harness task directories:

- `harness/tasks/` — 20 TUI/agent behavior specifications (invalid JSON, no graders)
- `crates/runie-agent/src/harness/tasks/` — 6 coding micro-tasks (invalid JSON, no graders)

Both directories have been removed. The findings are captured here and ranked so they can be addressed in the real codebase rather than in orphaned JSON files.

---

## Cleanup Actions Already Taken

1. Deleted `harness/tasks/*` (20 task directories with malformed JSON).
2. Deleted `crates/runie-agent/src/harness/tasks/*` (6 task directories with malformed JSON).
3. Removed the dead `crates/runie-agent/src/harness/` module entirely (runner, types, compaction).
4. Removed references to those task directories from harness documentation.
5. Left the top-level `harness/` crate in place; it gracefully handles empty task directories.

---

## Findings Ranked by Impact

### 🔴 Tier 1 — Core Reliability & Correctness

| # | Finding | Source Task(s) | Suggested Implementation |
|---|---------|----------------|--------------------------|
| 1 | **Permission handling is polled, not event-driven** | `permission_timeout`, `permission_rollback` | Replace `Arc<Mutex<Option<PermissionDecision>>>` polling in `crates/runie-agent/src/loop_engine/permissions.rs` with a per-tool-call `tokio::sync::oneshot` channel. Set a configurable timeout (default 5 min). |
| 2 | **Permission denial does not roll back partial file changes** | `permission_rollback` | Add a snapshot mechanism before destructive tools (`write_file`, `edit_file`, `bash` mutations). On `Deny`/`Timeout`, restore the snapshot. Store snapshots keyed by `tool_call_id`. |
| 3 | **Ctrl+C / cancellation leaves state inconsistent** | `ctrl_c_test`, `cancellation_clean_state` | Use `tokio_util::sync::CancellationToken` checked at turn boundaries. On cancel: abort current turn, set `agent_running = false`, return to `TuiMode::Chat`, restore any in-flight snapshots. |
| 4 | **Tool panics are not properly isolated** | `panic_recovery_test` | Remove misleading `catch_unwind` in `crates/runie-agent/src/loop_engine/tools.rs` or actually isolate async tool execution in `tokio::spawn` + `catch_unwind`. Ensure workspace state is preserved. |
| 5 | **Stale file edits are not detected** | `file_stale_edit` | Capture mtime at read time in `EditFileTool` and compare before write. Return a structured `StaleEdit` error instead of silently overwriting. |
| 6 | **Concurrent file edits have no locking** | `workspace_concurrent_edits` | Use the existing `Workspace::with_lock` / `atomic_write` helpers in `crates/runie-tools/src/workspace.rs` or delete them and implement proper file locking. |
| 7 | **Double submit is not deduplicated** | `double_submit_dedup` | Track `last_submitted_text` and `agent_running` in `AppState`. Block duplicate submission while an agent turn is running and provide visual feedback. |
| 8 | **Empty submit gives no feedback** | `idle_submit_feedback` | In `crates/runie-tui/src/tui/update/chat.rs`, detect empty textarea on submit and emit a transient status/notification instead of silently dropping. |

### 🟠 Tier 2 — Streaming, Network & Error Handling

| # | Finding | Source Task(s) | Suggested Implementation |
|---|---------|----------------|--------------------------|
| 9 | **Streaming errors discard partial content** | `stream_error_partial_response` | On provider stream error, emit `AgentEvent::Error` and keep the accumulated partial message in the feed. Mark the message with an error indicator. |
| 10 | **Streaming garbage / invalid UTF-8 can panic** | `streaming_garbage` | Validate UTF-8 in streaming chunks; skip or replace invalid bytes. Add a dedicated `StreamError` type. |
| 11 | **Network transient errors are not retried** | `network_retry` | Centralize retry logic in `runie-ai` with exponential backoff (1s, 2s, 4s) and max 3 attempts. Apply to provider `chat` calls and HTTP tool calls. |
| 12 | **Agent errors do not preserve UI state** | `error_state_recovery` | Ensure error path returns to `TuiMode::Chat`, preserves scroll position, and appends an error message without corrupting the message tree. |
| 13 | **Event channel backpressure strategy is undefined** | `channel_backpressure_test` | Document and implement backpressure for the main `Msg` channel (drop old ticks, await on agent events, bounded capacity with visible feedback). |
| 14 | **No model configured is not surfaced in status bar** | `no_model_warning` | In `StatusBar` rendering, when `current_model` is `None`, show a warning such as `⚠ No model configured` in a warning color. |

### 🟡 Tier 3 — TUI State, Rendering & UX

| # | Finding | Source Task(s) | Suggested Implementation |
|---|---------|----------------|--------------------------|
| 15 | **Empty chat has no placeholder** | `empty_state` | Render a placeholder in `MessageList` when `items` is empty: greeting, shortcut hints, and a CTA. |
| 16 | **TUI mode transitions are not validated** | `state_transition_test` | Replace flat `TuiMode` with an overlay stack or add an explicit transition table. Reject invalid transitions at the `update` layer. |
| 17 | **Progressive disclosure is missing** | `progressive_disclosure` | Hide advanced permission actions (`AllowAlways`, `Skip`) behind a modifier key or expandable section in `PermissionModal`. |
| 18 | **Component failures are not graceful** | `graceful_degradation` | Make sidebar/diff viewer/agent list failures non-fatal. Main content should render even if peripheral components error. |
| 19 | **Idempotency is not enforced for tool calls** | `idempotency_test` | Track executed `(tool_name, args)` pairs within a turn and skip duplicates on retry. |

### 🟢 Tier 4 — Code Quality Micro-Tasks (from agent harness)

| # | Finding | Source Task(s) | Suggested Implementation |
|---|---------|----------------|--------------------------|
| 20 | **Functions take too many positional parameters** | `param_struct` | Refactor large parameter lists (e.g., `execute_turn` with 15 args, `process_tool_calls` with 11 args) into `TurnContext` / `ToolContext` structs. |
| 21 | **Context compaction is incomplete/inconsistent** | `context_compact` | The dead `harness/compaction.rs` duplicate has been removed. Audit `loop_engine/context.rs`, add idempotency tests, and remove duplicate system prompts. |
| 22 | **Allocation/validation functions panic instead of returning `Result`** | `alloc_error` | Replace panics in validation code with typed `Result` errors. This applies broadly across tools and provider parsing. |
| 23 | **API call retry logic is missing** | `error_recovery` | Same as #11 — centralize retry in `runie-ai`. |
| 24 | **README/docs generation should be supported** | `readme_maker` | Optional: add a doc-generation tool or slash command that produces a basic README with title, install, usage, and license sections. |

---

## Consolidation Opportunities

Several task-derived findings overlap with the architecture review:

- **Permission system unification** — findings #1, #2, #7, #18 from tasks overlap with review finding #3 (polled permission `Option`). Address them together.
- **Context compaction** — finding #21 overlaps with review finding #12 (duplicated compaction). Pick one implementation and delete the other.
- **Error handling** — findings #9, #10, #11, #12, #13, #22 overlap with review finding #18 (`RunieError` string wrapping) and finding #30 (unwrap usage). Strongly type errors first, then add retries/graceful handling.
- **TUI state** — findings #15, #16, #17 overlap with review findings #13 (dead state scaffolding), #14 (legacy MessageItem/Feed), and #8 (permission queue duplication). A TUI state refactor should cover these.
- **Concurrency/safety** — findings #5, #6 overlap with review findings #16 and #17 (`Workspace` helpers, broken stale-edit check).

---

## Recommended Implementation Order

1. **Fix permission handling** (#1, #2, #7) — highest user-visible impact.
2. **Add cancellation / clean state** (#3, #4) — required for safe tool execution.
3. **Fix file safety** (#5, #6) — data-loss prevention.
4. **Unify error handling and retry** (#9, #10, #11, #12, #13, #22) — stability.
5. **TUI placeholders and mode transitions** (#15, #16, #17, #18, #19) — polish.
6. **Refactor large function signatures** (#20) and unify compaction (#21) — code health.
7. **Optional doc-generation tool** (#24) — nice-to-have.

---

## Notes on the Harness Itself

The `crates/runie-agent/src/harness/` module has been removed. The top-level `harness/` crate remains but has no task definitions; its runner code compiles and produces empty results.

Future options:

- **A. Reintroduce tasks as Rust integration tests** — replace JSON + Python graders with pure Rust tests under `harness/tests/`. This matches the existing `harness/tests/` files and avoids the Python dependency.
- **B. Convert agent harness tasks into unit tests** — implement the 6 micro-tasks from the plan as standard Rust tests in `crates/runie-agent/src/tests/`.
- **C. Remove the top-level `harness/` crate entirely** — if evaluation is not a near-term priority, delete `harness/` to reduce maintenance.

Until one of these is chosen, the harness crate compiles but produces empty results.
