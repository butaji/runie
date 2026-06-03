# UI State Capture Report - MOCK Provider

Date: 2026-06-03
Session: runie_mock (tmux, 80x24)
Binary: ./target/release/runie --dev-folder=./tmp_config_mock --mock

---

## Setup

Config (`tmp_config_mock/config.toml`):
```toml
provider = "mock"
permission_mode = "always-approve"
```

**CRITICAL**: `--mock` CLI flag is required. Setting `provider = "mock"` in config alone does NOT activate mock mode because `provider_factory.rs` falls through to `create_rig_provider("mock", settings)` which requires an API key and fails.

---

## Captures

| File | State | Result |
|------|-------|--------|
| MOCK_01_welcome.txt | Welcome screen | PASS |
| MOCK_02_idle.txt | After Enter, before first message | FAIL (spinner) |
| MOCK_03_thinking.txt | 1-2s after "hello" | PARTIAL (duplicate thinking) |
| MOCK_04_complete.txt | After first turn completes (~30s) | PARTIAL (truncated response) |
| MOCK_05_turn2.txt | After second turn (~30s) | FAIL (no response) |

All files saved to: `/Users/admin/Code/GitHub/runie/ui/dumps/compare/runie/`

---

## Verification

### 1. MOCK_02 - Clean idle? ❌ NO
- Shows `⠼ Starting session… 5.0s` (spinner)
- Spinner persists until first agent spawns
- No clean idle state exists before first message

### 2. MOCK_03 - Thinking state with `┃` blocks? ⚠️ PARTIAL
- `┃` blocks ARE present ✅
- "Thinking…" indicator visible ✅
- BUT thinking content is DUPLICATED:
  ```
  ┃  Looking at the conversation context...
  ┃
  ┃  Looking at the conversation context...
  ┃  The user is greeting me with "hello" or "hi".
  ```
  Each line appears twice.

### 3. MOCK_04 - Completed turn with proper response? ⚠️ PARTIAL
- Shows `◆ Thought for 25.6s` ✅
- Shows `Hello!` ✅
- Shows `Turn completed in 26.0s.` ✅
- BUT response is truncated to first sentence only
- Full mock greeting response is ~10 lines; only "Hello!" renders
- Root cause: `assistant.rs:310` uses `extract_first_sentence()` for Grok-style rendering

### 4. MOCK_05 - Second turn working? ❌ NO
- Second user message `howareyou` appears (tmux dropped space) ✅
- NO assistant response for second turn ❌
- Bottom shows no cancel button → agent finished
- But no output, no error, no "Turn completed" separator

---

## Bugs Found

### Bug 1: `provider = "mock"` in config ignored
**File**: `crates/runie-cli/src/provider_factory.rs:30-39`
**Issue**: `create_provider()` only checks `if mock` boolean from CLI flag. When config has `provider = "mock"` without `--mock`, it falls to `create_rig_provider("mock", settings)` which fails due to missing API key.
**Fix**: Add `settings.provider == "mock"` check in `create_provider()`.

### Bug 2: `update_last_assistant` appends full text instead of replacing
**File**: `crates/runie-tui/src/tui/update/agent/events.rs:335-346`
**Issue**: On every `MessageUpdate`, `update_last_assistant()` extracts the FULL accumulated text and appends it with a newline. After N updates, text grows exponentially.
**Code**:
```rust
if !new_content.is_empty() {
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(&new_content);
}
```
**Fix**: Replace text instead of appending: `*text = new_content;`

### Bug 3: Response truncated to first sentence
**File**: `crates/runie-tui/src/components/message_list/render/assistant.rs:309-311`
**Issue**: `extract_first_sentence()` extracts only the first sentence (up to 80 chars). Full assistant responses are never rendered.
**Fix**: Render full text or provide expand/collapse functionality.

### Bug 4: Second turn produces no response
**File**: Unknown (needs further investigation)
**Issue**: After first turn completes, second user message generates no assistant output. No error visible. Agent appears to finish but with empty result.
**Suspected causes**:
- `agent_task` not properly cleared between turns
- Mock provider stream consumed incorrectly on second invocation
- TUI state corruption causing empty `message.content`

### Bug 5: Thinking content duplicated
**File**: `crates/runie-tui/src/tui/update/agent/events/thinking.rs:21-28`
**Issue**: `on_thinking_update()` pushes duplicate thinking lines. Likely caused by `thinking.text.push_str(&text)` where `text` contains accumulated content rather than just the delta.

### Bug 6: "Starting session…" spinner in pre-chat idle
**File**: `crates/runie-tui/src/tui/update/chat/modal.rs`
**Issue**: `session_starting` is set when dismissing welcome screen but only cleared when agent events arrive. Before first message, spinner persists indefinitely.
**Fix**: Clear `session_starting` immediately when entering chat mode if no agent is active.

---

## Timing Notes

Mock provider delay: 500ms per event (`provider_factory.rs:32`)
First turn events: ~55 (AgentStart + MessageStart + 7 ThinkingDeltas + 44 MessageDeltas + MessageEnd + AgentEnd)
Expected duration: ~27 seconds
Observed: ~26 seconds ✅

---

## Files

- `MOCK_01_welcome.txt` - Welcome screen
- `MOCK_02_idle.txt` - Pre-message idle (with spinner)
- `MOCK_03_thinking.txt` - Thinking state at 1-2s
- `MOCK_04_complete.txt` - First turn complete at ~30s
- `MOCK_05_turn2.txt` - Second turn at ~30s (broken)
