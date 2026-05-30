# Error Handling Review Report

## Executive Summary

**Status: FIXED** — All CRITICAL and HIGH severity issues have been patched.

| Severity | Count | Fixed |
|----------|-------|-------|
| CRITICAL | 4 | 4 |
| HIGH | 6 | 6 |
| MEDIUM | 5 | 0 (acceptable) |
| LOW | 12 | 0 (acceptable) |

---

## CRITICAL Issues (Fixed)

### 1. `crates/runie-tools/src/retry.rs:160` — Unwrap on Option after loop
**Before:** `Err(_last_error.unwrap())` with `#[allow(clippy::unwrap_used)]`
**Risk:** If loop logic ever changes, `_last_error` could be `None`, causing panic.
**Fix:** Restructured loop to return `Err(e)` directly when max retries exceeded, eliminating the `Option` entirely.

### 2. `crates/runie-cli/src/event_stream.rs:289` — Panic on disk full
**Before:** `.expect("Failed to create fallback log file")` in `open_log_file()`
**Risk:** If primary log path fails AND fallback fails (disk full, read-only FS), entire application crashes during event logging.
**Fix:** Changed `open_log_file` to return `Option<File>`, changed `EventStreamLogger::new` to return `Option<Self>`, and made `log_event` silently skip if files unavailable. Event logging should never crash the app.

### 3. `crates/runie-tui/src/components/message_list/render/markdown.rs:326` — Syntect panic
**Before:** `highlighter.highlight_line(line, &SYNTAX_SET).unwrap()`
**Risk:** Syntect can panic on malformed/edge-case input. TUI rendering should never crash.
**Fix:** Changed to `match` with error handling — falls back to plain text `Line::raw()` on highlighting failure.

### 4. `crates/runie-tui/src/components/syntax_highlight.rs:25` — Syntect panic (duplicate)
**Before:** `highlighter.highlight_line(line, &SYNTAX_SET).unwrap()`
**Risk:** Same as #3 — another syntax highlighter with unwrap.
**Fix:** Same pattern — falls back to plain text on highlighting failure.

---

## HIGH Issues (Fixed)

### 5. `crates/runie-tools/src/rig_tools/bash.rs:166` — Runtime build panic
**Before:** `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()`
**Risk:** Runtime creation can fail (resource exhaustion). This is in the bash tool execution path.
**Fix:** Added `BashError::RuntimeCreationFailed` variant and propagated the error with `map_err`.

### 6. `crates/runie-tui/src/plugins/examples.rs` — RwLock poisoning (5 unwraps)
**Before:** Multiple `self.counts.read().unwrap()`, `self.state.write().unwrap()`, etc.
**Risk:** If any thread panics while holding the lock, the RwLock is poisoned and all subsequent accesses panic. This is in the TUI plugin system.
**Fix:** Used `unwrap_or_else(|poisoned| poisoned.into_inner())` for read locks, and `if let Ok(mut guard) = ...` for write locks to gracefully handle poisoning.

### 7. `crates/runie-tools/src/workspace.rs` — Mutex poisoning (3 unwraps)
**Before:** `self.locked.lock().unwrap()`, `self.file_locks.lock().unwrap()`, `lock.lock().unwrap()`
**Risk:** Lock poisoning in workspace file locking mechanism can crash file operations.
**Fix:** Used `match lock_result` with `poisoned.into_inner()` to recover from poisoned locks.

### 8. `crates/runie-cli/src/event_logger.rs:193` — Mutex poisoning
**Before:** `EVENT_LOGGER.lock().unwrap()`
**Risk:** If logger mutex is poisoned, initialization panics.
**Fix:** Changed to `if let Ok(mut global) = EVENT_LOGGER.lock()` with a warning trace on poison.

### 9. `crates/runie-ai/src/providers/faux.rs` — Mutex poisoning (3 unwraps)
**Before:** `self.responses.lock().unwrap()` in `set_responses`, `append_response`, and `stream`
**Risk:** Test provider can panic on lock poisoning.
**Fix:** Used `if let Ok(mut guard)` for mutating methods and `.ok().and_then(|mut guard| ...)` for the stream method.

### 10. `crates/runie-tui/src/components/command_palette/mod.rs:224` — NaN comparison panic
**Before:** `b.1.partial_cmp(&a.1).unwrap()`
**Risk:** `f32::partial_cmp` returns `None` for NaN values. If scores ever become NaN, the TUI crashes.
**Fix:** Changed to `.unwrap_or(std::cmp::Ordering::Equal)` to safely handle NaN.

---

## MEDIUM Issues (Acceptable / Not Fixed)

### 11. `crates/runie-agent/src/rig_loop/mod.rs:179` — Redundant unwrap after check
**Pattern:** `rig_messages.last().cloned().unwrap()` immediately after `is_empty()` check.
**Assessment:** Technically safe but poor style. Should use `expect()` or restructure. Left as-is since the empty check guarantees safety.

### 12. `crates/runie-tui/src/tui/update/permission.rs:68` — Redundant unwrap after check
**Pattern:** `tool_call_id.unwrap()` immediately after `is_some()` check.
**Assessment:** Same as #11 — safe but ugly. Left as-is.

### 13. `crates/runie-tui/src/tui.rs:184-186` — Triple unwrap after is_some guards
**Pattern:** `map_navigation_key(code).unwrap()` etc. after `is_some()` check.
**Assessment:** Safe due to guard. Could be cleaner with `if let Some(...)`.

### 14. `crates/runie-tui/build.rs:30` — Build script expect
**Pattern:** `fs::read_to_string(path).expect("Failed to read file")`
**Assessment:** Build-time only. Acceptable since build failure is the correct behavior if source files are unreadable.

### 15. `crates/runie-router/src/strategies/cost.rs:39` — NaN in cost comparison
**Pattern:** `cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)`
**Assessment:** Already uses `unwrap_or` — this is the correct pattern. Not an issue.

---

## LOW Issues (Acceptable)

### Regex unwraps in markdown.rs
**Lines:** 179, 184, 189, 194, 199, 204
**Pattern:** `regex::Regex::new("static_pattern").unwrap()`
**Assessment:** Static compile-time valid regex patterns. These are guaranteed to succeed and are idiomatic Rust.

### `unwrap_or_default()` / `unwrap_or(0)` / `unwrap_or("")` patterns
**Files:** `event_stream.rs`, `view_models.rs`, `pipe/view_model.rs`, `pipe/render.rs`, `settings.rs`, `search.rs`, `hook.rs`, `context.rs`, `loop_engine/run.rs`, `genai.rs`, `onboarding/render.rs`, `message_list/render/tool.rs`, `message_list/render/mod.rs`
**Assessment:** These are safe fallback patterns. They provide sensible defaults and do not panic.

### `strip_prefix(...).unwrap_or(path)` in search.rs
**Assessment:** Graceful fallback when stripping prefix fails. Safe.

---

## Error Propagation Assessment

### Good Patterns Found
- `crates/runie-agent/src/loop_engine/streaming.rs:33` — `match provider.chat(...).await { Ok(stream) => ..., Err(e) => ... }` with retry logic
- `crates/runie-agent/src/loop_engine/run.rs:143` — `match start_chat_with_retry(...) { Ok(s) => s, Err(e) => { tracing::error!(...); return false; } }`
- `crates/runie-ai/src/providers/rig.rs:420-423` — `match chunk { Ok(...) => ..., Err(e) => yield Event::Error { ... } }` — errors propagated as stream events
- `crates/runie-ai/src/providers/genai.rs` — errors propagated through mpsc channel with `.ok()`
- `crates/runie-cli/src/settings.rs` — `if let Ok(val) = std::env::var(...)` for graceful env var handling

### Swallowed Errors Found
- `crates/runie-cli/src/settings.rs:59-63` — `if let Ok(content) = fs::read_to_string(path) { if let Ok(file_settings) = toml::from_str(...) { ... } }` — silently ignores malformed config files. **MEDIUM:** Should at least warn.
- `crates/runie-tools/src/search.rs:22` — `filter_map(|e| e.ok())` — silently ignores filesystem traversal errors. **MEDIUM:** Should log skipped entries.
- `crates/runie-ai/src/providers/genai.rs:105,114,117,125` — `tx.send(...).await.ok()` — silently drops events if channel closed. **LOW:** Acceptable for streaming.

---

## Panic Paths Assessment

| Path | Can User Input Trigger? | Can Network Error Trigger? | Fixed? |
|------|------------------------|---------------------------|--------|
| Syntect highlighting | Yes (malformed code blocks) | No | **YES** |
| Event stream file creation | No | No | **YES** |
| Bash tool runtime creation | No | No | **YES** |
| Retry last_error unwrap | No | Yes (if retry logic bug) | **YES** |
| Mutex/RwLock poison | Yes (if tool panics) | No | **YES** |
| Command palette NaN sort | Yes (if NaN in scores) | No | **YES** |
| Regex::new static | No | No | N/A (safe) |

---

## Recovery Assessment

### After error, does the system recover?

| Component | Error Handling | Recovery |
|-----------|---------------|----------|
| Agent loop | `start_chat_with_retry` with exponential backoff | **YES** — retries on rate limit, returns error on fatal |
| Tool execution | `process_after_hooks` catches hook errors | **YES** — returns error output, continues |
| TUI event stream | Now returns `Option`, skips on failure | **YES** — after fix |
| Syntax highlighting | Now falls back to plain text | **YES** — after fix |
| File locking | Now handles poisoned locks | **YES** — after fix |
| Event logger | Now handles poisoned mutex | **YES** — after fix |

---

## Files Modified

1. `crates/runie-tools/src/retry.rs` — Removed `Option<E>`, restructured loop
2. `crates/runie-cli/src/event_stream.rs` — Made `new()` return `Option`, made `log_event` handle missing files
3. `crates/runie-cli/src/main.rs` — Removed `Some()` wrapper around `EventStreamLogger::new()`
4. `crates/runie-tui/src/components/message_list/render/markdown.rs` — Added error handling for syntect
5. `crates/runie-tui/src/components/syntax_highlight.rs` — Added error handling for syntect
6. `crates/runie-tools/src/rig_tools/bash.rs` — Added `RuntimeCreationFailed` error variant
7. `crates/runie-tools/src/workspace.rs` — Handle poisoned mutexes
8. `crates/runie-cli/src/event_logger.rs` — Handle poisoned mutex
9. `crates/runie-ai/src/providers/faux.rs` — Handle poisoned mutex
10. `crates/runie-tui/src/plugins/examples.rs` — Handle poisoned RwLocks
11. `crates/runie-tui/src/components/command_palette/mod.rs` — Handle NaN in partial_cmp

All changes compile successfully.
