# Fix `think_filter.rs` build guardrail violations

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-agent/src/think_filter.rs` currently exceeds the strict structural limits enforced by `crates/runie-core/build.rs`:

- File is **532 lines** (limit 500).
- `feed_text` is **~197 lines** (limit 40).
- `feed_text` complexity score is **26** (limit 10).

This makes `cargo build` fail before any functional work can be merged. The fix is to extract small, named helpers and move the inline test module to a proper test file so the production file stays under the limit and every production function is under 40 lines with complexity ≤ 10.

## Acceptance Criteria

- [ ] `cargo build --workspace` passes with zero lint violations.
- [ ] `cargo test --workspace` passes.
- [ ] All existing `ThinkFilter` behavior is preserved.
- [ ] Production file `think_filter.rs` is ≤ 500 lines.
- [ ] Every production function in `think_filter.rs` is ≤ 40 lines and complexity ≤ 10.

## Tests

### Layer 1 — State/Logic
- [ ] Keep every existing unit test from the inline `mod tests` (moved to `tests/think_filter.rs`).
- [ ] Add a test for nested `<tool_call>` / `</thinking>` cycles to exercise the refactored helpers.

### Layer 2 — Event Handling
- N/A — `ThinkFilter` is a pure state transformer.

### Layer 3 — Rendering
- N/A — no Ratatui code.

### Layer 4 — Provider Replay / E2E
- N/A — no provider/tool loop involved.

## Files touched

- `crates/runie-agent/src/think_filter.rs` — refactor production code.
- `crates/runie-agent/src/tests/think_filter.rs` — create and move tests here.

## Implementation

### Step 1: Move tests out of the production file

Create `crates/runie-agent/src/tests/think_filter.rs` with the current inline test module contents. Then delete the `#[cfg(test)] mod tests { ... }` block from `think_filter.rs`.

### Step 2: Decompose `feed_text`

Replace the single `feed_text` with a dispatcher and helpers. Keep the exact same public API (`feed`, `flush`, `feed_text`, `flush_buffer`).

Introduce these private helpers, each ≤ 40 lines and complexity ≤ 10:

```rust
fn emit_text(out: &mut Vec<LLMEvent>, text: String) {
    out.push(LLMEvent::TextDelta(text));
}

fn emit_thinking(out: &mut Vec<LLMEvent>, text: String) {
    out.push(LLMEvent::ThinkingDelta(text));
}

fn emit_thinking_start(out: &mut Vec<LLMEvent>) {
    out.push(LLMEvent::ThinkingStart { id: "inline".into() });
}

fn emit_thinking_end(out: &mut Vec<LLMEvent>) {
    out.push(LLMEvent::ThinkingEnd { id: "inline".into() });
}
```

Split the state machine into two focused functions:

```rust
fn consume_outside(
    text: &str,
    start_pos: usize,
    depth: &mut usize,
    in_thinking: &mut bool,
    buffer: &mut String,
    out: &mut Vec<LLMEvent>,
) -> usize {
    let rest = &text[start_pos..];
    if let Some((tag, open_pos)) = find_next_opening_tag(rest) {
        if open_pos > 0 {
            emit_text(out, text[start_pos..start_pos + open_pos].to_string());
        }
        let tag_end = start_pos + open_pos + tag.len();
        if tag_end < text.len() {
            emit_thinking_start(out);
            *depth = 1;
            *in_thinking = true;
            return tag_end;
        }
        *buffer = tag.to_string();
        *in_thinking = true;
        return text.len();
    }
    emit_text(out, text[start_pos..].to_string());
    text.len()
}
```

`consume_inside` handles the four `match` arms currently inside `if depth > 0`. Extract each arm into a helper (`close_block`, `open_nested`, `close_consecutive_check`, `buffer_partial_close`). Ensure each helper returns the new `pos`.

`feed_text` then becomes a short dispatcher:

```rust
fn feed_text(&mut self, delta: String) -> Vec<LLMEvent> {
    let text = prepend_buffer(self.buffer.drain(..).collect(), delta);
    let mut out = Vec::new();
    let mut pos = 0;
    let mut depth = self.depth;

    while pos < text.len() {
        pos = if depth > 0 {
            consume_inside(&text, pos, &mut depth, &mut self.in_thinking, &mut self.buffer, &mut out)
        } else {
            consume_outside(&text, pos, &mut depth, &mut self.in_thinking, &mut self.buffer, &mut out)
        };
    }

    self.depth = depth;
    out
}
```

(Adjust helper signatures so `buffer` can be updated for partial-tag buffering.)

### Step 3: Verify limits

Run:

```bash
cargo build --workspace
cargo test -p runie-agent think_filter
```

Expected: build passes, all tests pass.

### Step 4: Commit

```bash
git add crates/runie-agent/src/think_filter.rs crates/runie-agent/src/tests/think_filter.rs tasks/fix-think-filter-guardrails.md tasks/index.json
git commit -m "fix(agent): refactor think_filter to satisfy build guardrails"
```

## Notes

- Do not change event semantics; the existing tests are the spec.
- `find_next_opening_tag` is already small enough to keep.
- If moving tests still leaves the file near the limit, also remove duplicated doc comments (`/// Process a text delta...` appears twice at lines 66–69).
