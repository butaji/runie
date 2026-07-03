# Add session compaction with threshold and tool-pair summarization

## Status

`partial` — Compaction exists (`compact()`, `truncate_messages_structurally()`); token-ratio trigger implemented; async tool-pair summarization not yet implemented.

## Context

Runie has session compaction via `/compact` command. This task tracks adding automatic triggering and tool-pair summarization.

## What exists

### Compaction (`crates/runie-core/src/model/compaction.rs`)

- `compact(keep_recent_tokens)` — keeps recent messages by token count, preserves pinned messages
- `truncate_messages_structurally()` — truncates long code blocks and `<details>` sections
- `total_tokens()` — token accounting
- `MessageOrigin::Compaction` — already exists in `proto/message/role.rs`
- `COMPACT_FILE_SIZE_BYTES`, `COMPACT_EVENT_COUNT`, `COMPACT_TURN_COUNT`, `COMPACT_TARGET_EVENTS` — in `session/store.rs`
- `/compact` slash command — registered in `commands/dsl/handlers/session/run.rs`

## What is missing

### 1. Token-ratio trigger ✅

Add automatic compaction triggered when `tokens_in / context_window > threshold_ratio`.

**Implementation:**
1. `COMPACT_TOKEN_RATIO: f64 = 0.7` added to `session/store.rs`
2. `CompactionTriggered { ratio, tokens_in, context_window }` added to `event/mod.rs`
3. `current_model_context_window()` added to `AppState` accessors (`accessors.rs`)
4. In `dispatch.rs` `handle_turn_events`, after `apply_token_stats`:
   - Check `tokens_in > context_window * COMPACT_TOKEN_RATIO`
   - Emit `Event::CompactionTriggered` if threshold exceeded
5. `CompactionTriggered` handler calls `state.compact(keep_tokens)`

**Files modified:**
- `crates/runie-core/src/event/mod.rs` — `CompactionTriggered` event variant + taxonomy
- `crates/runie-core/src/event/taxonomy.json` — taxonomy entry
- `crates/runie-core/src/event/durable.rs` — non-durable event
- `crates/runie-core/src/session/store.rs` — `COMPACT_TOKEN_RATIO` constant
- `crates/runie-core/src/model/state/accessors.rs` — `current_model_context_window()`
- `crates/runie-core/src/update/dispatch.rs` — ratio check + `CompactionTriggered` handler
- `crates/runie-core/src/model/state/turn_projections.rs` — ratio check tests

### 2. Async tool-pair summarization

Summarize consecutive tool-call/tool-result pairs asynchronously off the hot path.

**Approach:**
1. Add `ToolPairSummaryRequested` and `ToolPairSummarized` events
2. In a `CompactionActor` or turn handler:
   - Scan messages for consecutive `Tool`/`User` origin pairs
   - Spawn `tokio::task::spawn_blocking` to call the model for summarization
   - Emit `ToolPairSummarized` with the summary
   - Replace the pair with a compact summary message

## Acceptance Criteria

- [x] Add `Compaction` origin and compaction event. — `MessageOrigin::Compaction` exists; `compact()` implemented
- [x] Trigger compaction at configurable context-limit ratio. — `COMPACT_TOKEN_RATIO = 0.7`; `CompactionTriggered` event emitted when `tokens_in > context_window * ratio`
- [ ] Summarize tool pairs asynchronously. — Not yet implemented

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for compaction strategy.
  - `compact_if_needed` fires at correct ratio threshold — ✅ implemented in dispatch.rs
  - Tool pair scanning identifies consecutive pairs — pending
- **Layer 2 — Event Handling:** Compaction facts emitted.
  - `CompactionTriggered` emitted when threshold exceeded — ✅ implemented
  - `ToolPairSummarized` emitted after async summarization — pending
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Long conversation replay tests pass.
- **Live tmux testing session (required):** Very long chat does not crash.

## Completion Validation

- [x] **Unit tests** — `current_model_context_window()` tests in `accessors.rs`; ratio check tests in `turn_projections.rs`.
- [x] **E2E tests** — `cargo nextest run --workspace --exclude runie-core` passes (pre-existing 2 failures unrelated).
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `AppState` owns compaction state; `compact()` is the canonical mutation method.
- [x] **Trigger events:** Token ratio threshold triggers compaction; `CompactionTriggered` event emitted via `dispatch_event`.
- [ ] **Observer events:** `CompactionComplete`, `ToolPairSummarized` notify observers. — pending (tool-pair summarization not implemented)
- [x] **No direct mutations:** Compaction triggers via `CompactionTriggered` event, handled by calling `state.compact()`.
- [x] **No new mirrors:** Compaction summary stored in session messages; no duplicate state.
- [ ] **Async work observed:** Tool-pair summarization must be awaited or have a JoinHandle owner. — pending
