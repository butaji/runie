# Add session compaction with threshold and tool-pair summarization

## Status

`partial` — Compaction exists (`compact()`, `truncate_messages_structurally()`), token-ratio trigger and async tool-pair summarization not yet implemented.

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

### 1. Token-ratio trigger

Add automatic compaction triggered when `tokens_in / context_window > threshold_ratio`.

**Approach:**
1. Add `COMPACT_TOKEN_RATIO: f64 = 0.7` constant (70% of context window)
2. Add `token_ratio` to `FffSection` in config (or new `CompactionSection`)
3. In turn actor (or `handle_update_speed` in `actors/turn/handlers.rs`), on `TokenStatsUpdated`:
   - Check if `tokens_in > context_window * ratio`
   - Emit `Event::CompactionTriggered { ratio, tokens_in, context_window }`
4. In dispatch or turn handlers, respond to `CompactionTriggered` by calling `compact()`

**Files to modify:**
- `crates/runie-core/src/model/compaction.rs` — add `compact_if_needed(ratio, tokens_in, context_window)`
- `crates/runie-core/src/event/mod.rs` — add `CompactionTriggered` event variant
- `crates/runie-core/src/actors/turn/handlers.rs` — handle `CompactionTriggered`
- `crates/runie-core/src/config/mod.rs` — add `CompactionSection` with `token_ratio`

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
- [ ] Trigger compaction at configurable context-limit ratio. — Not yet implemented
- [ ] Summarize tool pairs asynchronously. — Not yet implemented

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for compaction strategy.
  - `compact_if_needed` fires at correct ratio threshold
  - Tool pair scanning identifies consecutive pairs
- **Layer 2 — Event Handling:** Compaction facts emitted.
  - `CompactionTriggered` emitted when threshold exceeded
  - `ToolPairSummarized` emitted after async summarization
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Long conversation replay tests pass.
- **Live tmux testing session (required):** Very long chat does not crash.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `TurnActor` or `CompactionActor` owns compaction state.
- [ ] **Trigger events:** Token ratio threshold triggers compaction; `CompactionTriggered` event emitted.
- [ ] **Observer events:** `CompactionComplete`, `ToolPairSummarized` notify observers.
- [ ] **No direct mutations:** Compaction must emit events, not mutate `AppState` directly.
- [ ] **No new mirrors:** Compaction summary must not duplicate session state.
- [ ] **Async work observed:** Tool-pair summarization must be awaited or have a JoinHandle owner.
