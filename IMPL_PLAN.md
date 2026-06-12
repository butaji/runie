# Implementation Plan: Thinking Collapse + Output Truncation

Based on analysis of pi, crush, codex, aider, goose, and opencode.

---

## 1. Thinking / Tool Collapse

### Problem
Runie always renders full thinking text and full tool output. On long chains (deep reasoning + many tools), the chat becomes unreadable.

### Prior Art

**crush (Charm) — Best-in-class implementation:**
- Three-state toggle: `collapsed → tail-window → full-expanded → collapsed`
- Collapsed: shows last 10 rendered lines + "… (N lines hidden) [click or space to expand]"
- Tail-window: shows last 200 rendered lines + "… N earlier lines hidden [click for full view]"
- Full: everything, no truncation
- Skips tail-window step when content fits within 200 lines (short blocks toggle in 2 clicks)
- Per-section render cache: thinking, content, error each have separate caches keyed by (srcHash, extras)
- Click target detection via `thinkingBoxHeight` tracking
- Rendered AFTER glamour/markdown so code blocks aren't split mid-block

**pi:**
- Binary toggle: collapsed ↔ expanded via Ctrl+T
- Collapsed: one-line summary "◆ Thought 1.2s [+]"
- Simple but less flexible than crush's three-state

**codex:**
- No thinking collapse; shows raw reasoning inline

### Recommended Implementation (crush-style, simplified)

```rust
// Element addition
enum Element {
    // ... existing ...
    ThoughtMarker {
        content: String,
        collapsed: bool,     // NEW
    },
    ToolDone {
        name: String,
        duration_secs: f64,
        collapsed: bool,     // NEW
    },
}
```

**Event:** `Event::ToggleCollapse { element_index: usize }`

**TUI behavior:**
1. Collapsed thought: `◆ Thought 1.2s [+]` (single line, dark gray)
2. Expanded thought: full content (no truncation for now — pi doesn't truncate thoughts)
3. Collapsed tool: `◆ Ran list_files 0.5s [+]`
4. Expanded tool: full output

**Keybind:** `Ctrl+T` for thoughts, `Ctrl+O` for tools (matches pi)

**Cache impact:** None. Collapse is a render-time decision; the Element content doesn't change.

**Tests:**
- `thought_collapsed_renders_single_line` — verify collapsed shows only header
- `thought_expanded_renders_full_content` — verify expanded shows all lines
- `toggle_collapse_flips_state` — verify event toggles collapsed flag
- `tool_collapsed_renders_single_line`
- `tool_expanded_renders_full_output`

---

## 2. Output Truncation

### Problem
Runie passes full tool output (grep, bash, read_file) straight into the message history. A large `grep` can produce 50K+ lines, blowing up the context window.

### Prior Art

**pi — Most mature:**
- Two independent limits: **lines** (default 2000) and **bytes** (default 50KB)
- Whichever is hit first wins
- `truncateHead()` — keeps first N lines/bytes (for file reads where beginning matters)
- `truncateTail()` — keeps last N lines/bytes (for bash where errors are at end)
- Never returns partial lines (except bash tail edge case)
- `OutputAccumulator` — incrementally tracks streaming output with bounded memory:
  - Rolling tail buffer (2x max bytes)
  - Temp file when output exceeds limits
  - `snapshot()` returns truncated view + metadata

**codex — Middle truncation:**
- `truncate_middle_chars()` — preserves beginning AND end, removes middle
- Budget split 50/50 between head and tail
- UTF-8 safe (never splits multi-byte characters)
- Token-aware variant: `truncate_middle_with_token_budget()`
- Shows marker: `…{N} tokens truncated…`

**crush — No truncation in tool output:**
- Tool results rendered as-is
- But thinking blocks have tail-window (last N lines) in expanded view

**goose — Conversation truncation:**
- `truncate(len)` on conversation history
- Not tool-output-specific

### Recommended Implementation (pi + codex hybrid)

**For tool output storage (before adding to messages):**

```rust
pub struct TruncationPolicy {
    pub max_lines: usize,      // default: 2000
    pub max_bytes: usize,      // default: 50_000
}

pub struct TruncatedOutput {
    pub content: String,
    pub was_truncated: bool,
    pub truncated_by: Option<TruncatedBy>,  // Lines | Bytes
    pub total_lines: usize,
    pub total_bytes: usize,
}
```

**Algorithm per tool type:**

| Tool | Strategy | Rationale |
|------|----------|-----------|
| `read_file` | `truncate_head` | Beginning of file matters |
| `bash` | `truncate_tail` | Errors are at the end |
| `grep` | `truncate_head` | First matches are most relevant |
| `find` | `truncate_head` | Directory listing order |
| `ls` | `truncate_head` | Top entries first |

**Implementation:**

```rust
pub fn truncate_head(content: &str, policy: &TruncationPolicy) -> TruncatedOutput {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let total_bytes = content.len();

    if total_lines <= policy.max_lines && total_bytes <= policy.max_bytes {
        return TruncatedOutput::full(content);
    }

    // Collect complete lines that fit both limits
    let mut output = Vec::new();
    let mut output_bytes = 0;

    for (i, line) in lines.iter().enumerate() {
        if i >= policy.max_lines { break; }
        let line_bytes = line.len() + 1; // +1 for newline
        if output_bytes + line_bytes > policy.max_bytes {
            break;
        }
        output.push(*line);
        output_bytes += line_bytes;
    }

    TruncatedOutput::truncated(
        output.join("\n"),
        total_lines, total_bytes,
        output.len(), output_bytes,
    )
}

pub fn truncate_tail(content: &str, policy: &TruncationPolicy) -> TruncatedOutput {
    // Similar but works backwards from end
    // ...
}
```

**For collapsed tool display (TUI-level):**

When a tool result is collapsed in the TUI, show a **tail-window** (last N lines) similar to crush's thinking block:

```
◆ Ran grep 2.3s [+] (truncated: 4,832 of 12,450 lines)
```

On expand, show the truncated content (already stored in the message) with a warning header:

```
[Output truncated: 4,832 of 12,450 lines shown (50KB limit)]
---
<actual truncated content>
```

**Tests:**
- `truncate_head_keeps_beginning` — first lines preserved
- `truncate_tail_keeps_end` — last lines preserved
- `truncate_respects_line_limit` — stops at max_lines
- `truncate_respects_byte_limit` — stops when bytes exceeded
- `truncate_no_partial_lines` — never splits a line mid-content
- `truncate_small_input_unchanged` — no truncation when under limits
- `tool_result_shows_truncation_warning` — message includes truncation notice

---

## 3. Unified Timer Architecture (already done)

**Status:** ✅ Fixed in `da830d66`

Store `std::time::Instant` in `Element::Thinking` and `Element::ToolRunning`, compute `elapsed()` at render time. No cache rebuild needed for animation.

---

## 4. Thinking Levels (future, not in this plan)

**pi:** Low/Medium/High reasoning via Shift+Tab. Changes both prompt and display.
**opencode:** Model variants with `reasoningEffort: "high"` in request options.

This requires prompt template changes + provider request option support. Out of scope for now.

---

## Implementation Order

1. **Collapse** (simpler, no data model changes beyond a bool flag)
   - Add `collapsed: bool` to `ThoughtMarker` and `ToolDone`
   - Add `Event::ToggleCollapse`
   - Wire `Ctrl+T` / `Ctrl+O` in input reader
   - Render collapsed as single-line summary

2. **Truncation** (more complex, touches tool execution)
   - Add `TruncationPolicy` and `TruncatedOutput`
   - Implement `truncate_head` and `truncate_tail`
   - Apply to each tool's `execute()` result
   - Store truncation metadata in `Tool` message content
   - Render truncation warning in TUI

---

## Files to Touch

| File | Change |
|------|--------|
| `crates/runie-core/src/ui/elements.rs` | Add `collapsed: bool` to `ThoughtMarker`, `ToolDone` |
| `crates/runie-core/src/event.rs` | Add `ToggleCollapse { index: usize }` |
| `crates/runie-core/src/update.rs` | Handle `ToggleCollapse` event |
| `crates/runie-core/src/model.rs` | Track collapse state per element (or in Element itself) |
| `crates/runie-core/src/ui/transform.rs` | Pass collapse state through to elements |
| `crates/runie-tui/src/ui.rs` | Render collapsed/expanded views |
| `crates/runie-term/src/main.rs` | Wire `Ctrl+T`, `Ctrl+O` keybinds |
| `crates/runie-agent/src/lib.rs` | Add `TruncationPolicy`, apply to tool results |
| `crates/runie-agent/src/tests.rs` | Truncation tests |
| `crates/runie-core/src/tests/` | Collapse + truncation TUI tests |
