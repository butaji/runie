# Architecture & Code Review — Runie

**Branch:** `agent-impl` @ `273a80ce`  
**Tests:** 477 passing  
**Lines:** ~10,600 across 5 crates

---

## 🔴 P0 — Critical

### 1. `update.rs` is 623 lines — exceeds 500-line lint limit
**File:** `crates/runie-core/src/update.rs`  
**Impact:** Violates project's own AGENTS.md constraint. Hard to navigate, test, review.  
**Fix:** Split into `update/` module:
```
update/
  mod.rs       — pub fn dispatch + Event match
  input.rs     — push_input, pop_input, submit, @-ref logic
  agent.rs     — thinking, thought, tool, response, done
  slash.rs     — all /commands
  queue.rs     — follow_up, abort, deliver_queued
```

### 2. `AppState` has 28 fields — violates single-responsibility
**File:** `crates/runie-core/src/model.rs`  
**Impact:** God object. Tight coupling between input, scroll, agent lifecycle, animation, @-refs.  
**Fix:** Split into focused structs with composition:
```rust
struct AppState {
    input: InputState,
    chat: ChatHistory,
    agent: AgentState,
    ui: UiState,
}
```

### 3. `finish_turn()` does 6 unrelated things + 2 side-effect reorders
**File:** `crates/runie-core/src/update.rs:290-330`  
**Impact:** One function strips tools, retains empty, clears IDs, decrements inflight, delivers queued, reorders agent after tools, moves TurnComplete.  
**Fix:** Extract each step into a private method with a name.

---

## 🟡 P1 — Important

### 4. `append_response` searches entire `messages` vec on every chunk
**File:** `crates/runie-core/src/update.rs:260-280`  
**Impact:** O(n) per streaming chunk. With 1000 messages and fast provider, this adds up.  
**Fix:** Keep `last_assistant_index: Option<usize>` in state, update on append.

### 5. `messages_changed()` rebuilds entire element cache synchronously
**File:** `crates/runie-core/src/model.rs:145-155`  
**Impact:** `ensure_fresh()` does O(n) work per event (transform all messages → elements → line counts). Under fast streaming, this blocks the event loop.  
**Fix:** Defer cache rebuild to just before snapshot, or incremental update.

### 6. `hint_text()` recomputed on every snapshot
**File:** `crates/runie-core/src/update.rs:70-90`  
**Impact:** Allocates strings and runs 5+ branches every frame.  
**Fix:** Cache in state, invalidate only on events that change it (input, turn_active, suggestions).

### 7. Clippy warnings in production code
```
manual checked division              (2x)
manually reimplementing div_ceil     (1x)
stripping a prefix manually          (1x)
called Iterator::last on DoubleEnded (2x)
redundant closure                     (1x)
```
**Fix:** `cargo clippy --fix` + review.

### 8. `Snapshot` clones `elements_cache` (Vec<Element>) every frame
**File:** `crates/runie-core/src/model.rs`  
**Impact:** Under fast streaming, clones grow linearly with chat history.  
**Fix:** Use `Arc<[Element]>` or send `Arc<Snapshot>` to render task.

---

## 🟢 P2 — Polish / Debt

### 9. Test files are scattered across 3 crates with no naming convention
**Impact:** Hard to find related tests. `tests/` dirs in both `runie-core` and `runie-term`.  
**Fix:** Co-locate tests with features. E.g., all scroll tests in one module.

### 10. `VisibleRegion` and `visible_scroll()` still exist but are unused by render
**File:** `crates/runie-core/src/model.rs`  
**Impact:** Dead code. Render now uses `scroll_offset()` + `Paragraph::scroll()`.  
**Fix:** Delete `VisibleRegion`, `visible_scroll()`, and `visible()` methods.

### 11. `action_text()` takes `char` for spinner but `Element::Thinking` stores `Instant`
**Impact:** Render computes `spinner_frame` from `Snapshot`, but `action_text()` recomputes nothing. Slight impedance mismatch.  
**Fix:** Pass `spinner_frame: char` consistently everywhere (already mostly done).

### 12. `dev.sh` uses `cargo watch -x run` which rebuilds on any file change
**Impact:** Slow feedback loop during TDD — rebuilds on test file changes too.  
**Fix:** `cargo watch -x "run --bin runie" --ignore "*/tests/*"` or use `cargo-watch` flags.

### 13. No integration test for real provider end-to-end
**Impact:** `OpenAiProvider` is compile-tested but never run against a real API.  
**Fix:** Add `#[ignore]` test that runs with `OPENAI_API_KEY` env var.

### 14. `ScrollUp` / `ScrollDown` names are inverted from convention
**Impact:** `ScrollUp` increases `scroll` (moves content down, shows older). This is correct for our model but counter-intuitive.  
**Fix:** Rename to `ScrollBack` / `ScrollForward` or add comment.

---

## ✅ What's Done Well

| Area | Verdict |
|------|---------|
| **Render actor pattern** | Clean separation — event loop never blocks on I/O |
| **Snapshot immutability** | Pure render path, no mutable state in draw |
| **Native ratatui scroll** | `Paragraph::scroll()` + `ScrollbarState` — canonical |
| **Semantic reorder** | Final agent after tools — fixes the >1 page bug |
| **Global collapse** | `all_collapsed: bool` — simple, correct |
| **Test coverage** | 477 tests, 3-layer TDD (state → event → render) |
| **No `unsafe`** | None used |
| **No `unwrap` in hot path** | Only in tests and fallback defaults |

---

## TL;DR

| Rank | Issue | File | Fix Effort |
|------|-------|------|------------|
| 🔴 P0 | `update.rs` 623 lines, over limit | `update.rs` | Medium (split to module) |
| 🔴 P0 | `AppState` has 28 fields (god object) | `model.rs` | Medium (composition) |
| 🔴 P0 | `finish_turn()` does 8 things | `update.rs` | Small (extract methods) |
| 🟡 P1 | `append_response` is O(n) per chunk | `update.rs` | Small (cache index) |
| 🟡 P1 | Cache rebuild every event | `model.rs` | Medium (lazy/incremental) |
| 🟡 P1 | `hint_text()` recomputed every frame | `update.rs` | Small (cache) |
| 🟡 P1 | Clippy warnings in prod | various | Small (`cargo clippy --fix`) |
| 🟡 P1 | `Snapshot` clones Vec every frame | `model.rs` | Small (`Arc`) |
| 🟢 P2 | `VisibleRegion` is dead code | `model.rs` | Trivial (delete) |
| 🟢 P2 | Test scattering | `tests/` | Small (reorganize) |

**Top 3 to fix now:** split `update.rs`, shrink `AppState`, cache `hint_text`.
