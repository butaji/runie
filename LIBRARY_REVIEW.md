# Library Review: runie

## Section 1: Library Inventory

### Workspace Dependencies (Cargo.toml root)

| Library | Version | Purpose |
|---------|---------|---------|
| `ratatui` | 0.30 | TUI rendering framework |
| `crossterm` | 0.28 | Terminal input/output |
| `tokio` | 1.40 | Async runtime |
| `serde` / `serde_json` | 1.0 / 1.0 | Serialization |
| `reqwest` | 0.12 | HTTP client |
| `async-trait` | 0.1 | Async trait support |
| `futures` | 0.3 | Async utilities |
| `thiserror` | 1.0 | Error handling |
| `tracing` | 0.1 | Structured logging |
| `chrono` | 0.4 | Date/time |
| `uuid` | 1.10 | UUID generation |
| `anyhow` | 1.0 | Error context |
| `async-stream` | 0.3 | Async streams |
| `clap` | 4.5 | CLI argument parsing |
| `walkdir` | 2.5 | Directory traversal |
| `tempfile` | 3.13 | Temp file handling |
| `tui-pantry` | 0.4 | TUI pantry integration |
| `rand` | 0.8 | Random number generation |
| `regex` | 1.10 | Regex operations |

### Per-Crate Dependencies

| Crate | Key External Dependencies |
|-------|--------------------------|
| `runie-tui` | `ratatui 0.30`, `ratatui-textarea 0.9`, `crossterm 0.28`, `opaline 0.1` |
| `runie-ai` | `genai 0.5`, `rig-core 0.37`, `reqwest 0.12` |
| `runie-agent` | `tokio 1.40` (full features) |
| `runie-orchestrator` | `tokio 1.40` (sync features) |
| `runie-cli` | `tokio 1.40`, `crossterm 0.28`, `clap 4.5`, `dirs 5.0`, `toml 0.8` |
| `runie-tools` | `walkdir 2.5` |
| `runie-core` | (no external runtime deps) |
| `pantry` | `ratatui 0.30`, `tui-pantry 0.4`, `crossterm 0.28` |

---

## Section 2: Usage Review by Library

### ratatui (v0.30.0)

#### Widget vs StatefulWidget Usage

**Finding: Uses `Widget` trait correctly with `render_ref` pattern.**

The project uses the `Widget` trait correctly. Most widgets implement `Widget` and are rendered via `render_ref()` which borrows `&self` rather than consuming it:

```rust
// crates/runie-tui/src/components/message_list.rs:29
pub fn render_ref(vm: &MessageListViewModel, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    // ...
}
```

**However:** The project does NOT use `StatefulWidget` trait despite having render state. Instead, `RenderState::from()` clones ALL data every frame (see Issue #1 below).

#### RenderState Cloning Issue (MAJOR)

**Location:** `crates/runie-tui/src/tui/state.rs:501-527`

```rust
impl RenderState {
    pub fn from(state: &AppState) -> Self {
        Self {
            messages: state.messages.clone(),           // Vec clone
            textarea: state.textarea.clone(),          // TextArea clone
            input_right_info: state.input_right_info.clone(),
            mode: state.mode.clone(),
            top_bar: state.top_bar.clone(),            // All nested structs cloned
            permission_modal: state.permission_modal.clone(),
            scroll: state.scroll.clone(),
            animation: state.animation.clone(),
            diff_viewer: state.diff_viewer.clone(),
            session_token_usage: state.session_token_usage.clone(),
            session_tree: state.session_tree.clone(),
            background_jobs: state.background_jobs.clone(),
            onboarding: state.onboarding.clone(),
            // ...
        }
    }
}
```

**Impact:** Every frame (80ms tick + every event) triggers 15+ clone operations. This includes:
- `Vec<MessageItem>` - can be hundreds of messages
- `ratatui_textarea::TextArea<'static>` - internal buffer clone
- `DiffViewer` - potentially large diff content
- `SessionTreeNavigator` - tree structure clone
- `Onboarding` - wizard state clone

**Recommended Fix:** Implement `StatefulWidget` for `MessageList`, `DiffViewer`, etc. Pass `&mut RenderState` to `render()` instead of cloning.

#### WrapCache LRU Implementation

**Location:** `crates/runie-tui/src/components/message_list/render.rs:16-68`

```rust
pub struct WrapCache {
    cache: HashMap<(String, usize), Vec<String>>,
    access_order: Vec<(String, usize)>,
    max_size: usize,
}

impl WrapCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_size: 100,  // LRU limit
        }
    }
}
```

**Assessment:** Correctly implements LRU eviction with `max_size = 100`. However:
- `new()` is called every render in `message_list.rs:55`: `let mut wrap_cache = render::WrapCache::new();`
- This means cache is recreated every frame, losing effectiveness
- Should be stored in `MessageListViewModel` or similar persistent state

#### Terminal Lifecycle

**Finding: Uses manual raw mode management via `crossterm`.**

Location: `crates/runie-tui/src/tui.rs:107-127`
```rust
pub fn new(config: TuiConfig) -> io::Result<Self> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    // ...
}
```

**Issue:** Does NOT use `ratatui::run()` (new high-level API from v0.28+). Manual cleanup required. Also note the panic hook at line 90-105 properly restores terminal on panic.

---

### tokio (v1.40)

#### Channel Backpressure

**Finding: Most channels use bounded buffers correctly (100 capacity), but some gaps exist.**

**Good patterns:**
```rust
// crates/runie-cli/src/tui_run.rs
let (raw_tx, mut raw_rx) = mpsc::channel::<crossterm::event::Event>(100);  // Line 135
let (agent_tx, mut agent_rx) = mpsc::channel::<AgentEvent>(100);             // Line 160
let (model_fetch_tx, mut model_fetch_rx) = mpsc::channel::<Result<...>>(1);  // Line 173
let (fresh_perm_tx, _fresh_perm_rx) = mpsc::channel::<PermissionDecision>(100); // Line 232
```

**Raw terminal event thread backpressure** (Lines 137-157):
```rust
std::thread::spawn(move || {
    loop {
        if let Ok(event) = crossterm::event::read() {
            let mut sent = false;
            for _ in 0..10 {
                if raw_tx.try_send(event.clone()).is_ok() {  // try_send, not send
                    sent = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));  // Backoff
            }
            if !sent {
                // Drop event if channel full for >10ms - acceptable for keyboard events
                continue;
            }
        }
    }
});
```

**Assessment:** Uses `try_send` with retry loop for backpressure. Drops events after 10ms. This is acceptable for keyboard events but could miss rapid typing.

**Permission channels** (`crates/runie-agent/src/permission.rs:16,97,116,141,157`):
```rust
let (tx, rx) = mpsc::channel(10);  // Buffer of 10, not 100
```

**Issue:** Permission channels use buffer of 10 while other channels use 100. If many permission requests queue up faster than user can respond, they'll be dropped. Consider increasing to match other channels.

#### biased Selection

**Finding: Correctly uses `biased;` to control tokio::select! polling order.**

**Location:** `crates/runie-cli/src/tui_run.rs:379-495`

```rust
while tui.state.running {
    tokio::select! {
        biased;

        // Raw terminal events — HIGHEST PRIORITY
        Some(event) = raw_rx.recv() => { /* ... */ }

        // Agent events — SECOND PRIORITY
        Some(event) = agent_rx.recv() => { /* ... */ }

        // Cursor blink (500ms) — THIRD PRIORITY
        _ = cursor_interval.tick() => { /* ... */ }

        // Model fetch results
        Some(result) = model_fetch_rx.recv() => { /* ... */ }

        // Animation tick (80ms) — LOWEST PRIORITY
        _ = tick_interval.tick() => { /* ... */ }
    }
}
```

**Assessment:** Correct priority ordering ensures keyboard events don't starve when agent is busy. Good pattern.

#### Task Cleanup

**Finding: Agent task cleanup exists but could be improved.**

**Location:** `crates/runie-cli/src/tui_run.rs:497-501`
```rust
if let Some(task) = agent_task.take() {
    task.abort();
    let _ = task.await;  // Await to propagate any cancellation errors
}
```

**Good:** Awaits task after abort to ensure cleanup completes.

**Issue:** `Cmd::Interrupt` at line 349-353 also aborts, but only clears the task handle:
```rust
Cmd::Interrupt => {
    if let Some(handle) = agent_task.take() {
        handle.abort();
    }
    vec![]
}
```

This does NOT await the task, potentially leaving cleanup incomplete.

---

### crossterm (v0.28 vs 0.29)

#### Version Mismatch

**Finding: MAJOR - crossterm version mismatch detected.**

- Workspace defines `crossterm = "0.28"`
- `ratatui-textarea = "0.9"` internally uses `crossterm 0.29`

**Impact:** Manual `KeyEvent` → `ratatui_textarea::Input` conversion required.

**Location:** `crates/runie-tui/src/tui.rs:484-516`
```rust
/// Manual conversion needed because project crossterm (0.28) differs
/// from ratatui-textarea's internal crossterm (0.29) via ratatui-crossterm.
pub fn key_to_textarea_input(key: crossterm::event::KeyEvent) -> ratatui_textarea::Input {
    // ... manual mapping of KeyCode to ratatui_textarea::Key
}
```

**Issue:** This conversion is brittle. Any API differences between versions could cause key handling bugs. Upgrading workspace crossterm to 0.29 would eliminate this hack.

**Note:** Comment at line 421-422 explicitly states this:
```rust
// Convert crossterm KeyEvent to ratatui-textarea Input.
// Manual conversion needed because project crossterm (0.28)
// differs from ratatui-textarea's crossterm (0.29).
```

---

### serde / serde_json

**Assessment: No significant issues found.**

- Used for JSON serialization of AI API requests/responses
- `reqwest` with `json` feature handles serialization for HTTP bodies
- `serde_json::Value` used for dynamic JSON manipulation

**Performance Note:** No apparent misuse. JSON parsing is lazy where possible.

---

### reqwest / HTTP Client Patterns

**Finding: Correct usage in AI provider implementations.**

**OpenAI** (`crates/runie-ai/src/providers/openai.rs:31-35`):
```rust
let client = Client::builder()
    .timeout(std::time::Duration::from_secs(120))
    .connect_timeout(std::time::Duration::from_secs(30))
    .build()
```

**Anthropic** (`crates/runie-ai/src/providers/anthropic.rs:29-33`):
```rust
let client = Client::builder()
    .timeout(std::time::Duration::from_secs(300))  // Longer timeout for streaming
    .connect_timeout(std::time::Duration::from_secs(30))
    .build()
```

**Assessment:**
- Timeouts are appropriate (120s for OpenAI, 300s for Anthropic)
- Uses SSE streaming correctly with `bytes_stream()`
- Retry logic exists for rate limiting but not for network errors

---

### genai / rig-core

**Finding: genai 0.5 and rig-core 0.37 are used for Google AI provider integration.**

**genai provider** (`crates/runie-ai/src/providers/genai.rs`):
- Uses `Client::default()` - no explicit configuration
- Converts messages to genai format
- `_tools: Vec<ToolSchema>` ignored in `chat()` - tools not supported

**Issue:** Tools parameter is ignored for genai:
```rust
async fn chat(
    &self,
    messages: Vec<Message>,
    _tools: Vec<ToolSchema>,  // IGNORED
) -> Result<BoxStream<'static, Event>, ProviderError>
```

---

## Section 3: Recommendations from ctx7

### ratatui: Modern Patterns

1. **Consider StatefulWidget for complex components.** The current `render_ref` pattern clones state every frame. StatefulWidget passes `&mut state` to `render()`, eliminating clones.

2. **WrapCache should persist across frames.** Currently recreated every render at `message_list.rs:55`. Store it in ViewModel or AppState to maintain cache effectiveness.

3. **Consider ratatui::run()** for automatic terminal lifecycle. Current manual implementation is correct but more verbose.

### tokio: Best Practices

1. **Await aborted tasks** in `Cmd::Interrupt` handler:
```rust
Cmd::Interrupt => {
    if let Some(handle) = agent_task.take() {
        handle.abort();
        let _ = handle.await;  // Add this
    }
    vec![]
}
```

2. **Increase permission channel buffer** from 10 to 100 to match other channels.

3. **Consider watch channel** for state that doesn't need buffering (e.g., animation tick).

### crossterm: Upgrade Path

**Upgrade crossterm to 0.29** in workspace to:
- Eliminate manual key conversion hack
- Align with ratatui-textarea's crossterm version
- Avoid potential key handling bugs from version mismatch

### Error Handling

1. **Agent task errors** at `tui_run.rs:277` silently dropped:
```rust
Err(e) => eprintln!("Agent error: {}", e),
```
Consider propagating to UI for user visibility.

2. **Network errors in chat_with_retry** only retry on `RateLimited`. Consider adding retry for connection errors with exponential backoff.

---

## Section 4: Specific Issues Found

### Issue #1: RenderState Clones All Data Every Frame (HIGH)

**Severity:** High  
**Location:** `crates/runie-tui/src/tui/state.rs:501-527` + `crates/runie-tui/src/tui.rs:182`

**Problem:** `RenderState::from()` clones 15+ fields every frame. For a chat with 100 messages averaging 500 chars each:
- 100 messages × 500 chars = 50KB per clone
- At 80ms tick = 12.5 clones/second = ~625KB/s just for messages
- Plus textarea, diff viewer, etc.

**Impact:** Performance degradation as conversation grows. Potential UI lag during streaming.

**Fix:** Implement `StatefulWidget` trait for `MessageList`, `DiffViewer`, etc. Pass `&mut RenderState` to render functions instead of cloning.

### Issue #2: WrapCache Recreated Every Frame (MEDIUM)

**Severity:** Medium  
**Location:** `crates/runie-tui/src/components/message_list.rs:55`

**Problem:** `WrapCache::new()` called every render, losing cached wrap computations.

**Impact:** Text wrapping recalculated every frame instead of once per content change.

**Fix:** Store `WrapCache` in `MessageListViewModel` or as mutable state in the render call chain.

### Issue #3: crossterm Version Mismatch (MEDIUM)

**Severity:** Medium  
**Location:** Workspace `Cargo.toml:38` vs `ratatui-textarea 0.9`

**Problem:** Workspace uses crossterm 0.28, but ratatui-textarea 0.9 uses crossterm 0.29 internally.

**Impact:** Manual key conversion required (`tui.rs:487-516`). Brittle, could break on API changes.

**Fix:** Upgrade workspace crossterm to 0.29.

### Issue #4: Permission Channel Buffer Too Small (LOW)

**Severity:** Low  
**Location:** `crates/runie-agent/src/permission.rs:16,97,116,141,157`

**Problem:** Permission channels use buffer of 10 vs 100 for other channels.

**Impact:** If many permission requests queue during rapid tool execution, excess requests dropped.

**Fix:** Increase to 100 to match other channels.

### Issue #5: Interrupted Task Not Awaited (LOW)

**Severity:** Low  
**Location:** `crates/runie-cli/src/tui_run.rs:349-353`

**Problem:** `Cmd::Interrupt` aborts task but doesn't await it.

**Impact:** Task cleanup may not complete before new task spawns.

**Fix:** Add `let _ = handle.await;` after `handle.abort();`.

### Issue #6: genai Tools Not Supported (INFO)

**Severity:** Info  
**Location:** `crates/runie-ai/src/providers/genai.rs:47`

**Problem:** `chat()` ignores `tools` parameter.

**Impact:** Google AI provider cannot use tool calling, limiting its utility.

**Fix:** Implement tool support via genai's function calling API if available.

---

## Summary Table

| Issue | Severity | Library | Type |
|-------|----------|---------|------|
| RenderState clones every frame | High | ratatui | Performance |
| WrapCache recreated every frame | Medium | ratatui | Performance |
| crossterm version mismatch | Medium | crossterm | Maintenance |
| Permission channel buffer size | Low | tokio | Correctness |
| Interrupted task not awaited | Low | tokio | Correctness |
| genai tools not supported | Info | genai | Feature gap |
