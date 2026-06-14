# Crate Replacement Decisions

This document records which pieces of Runie's custom code can be replaced
with maintained crates, based on `ctx7` CLI research and code inspection.

Methodology:
1. Identify custom modules in `crates/`.
2. Use `ctx7 library <name>` to find Context7 IDs.
3. Use `ctx7 docs <id> <query>` to fetch API examples.
4. Compare crate capability with Runie's actual needs.
5. Decide: **Adopt**, **Evaluate**, **Partial Adopt**, or **Keep Custom**.

## Summary Table

| Custom Module | Crate | Context7 ID | Decision | Priority |
|---|---|---|---|---|
| Syntax highlighting (`runie-tui/src/syntax/`) | `syntect` | `/trishume/syntect` | **Adopt** | High |
| Diff generation (`runie-agent/src/diff.rs`) | `similar` | `/mitsuhiko/similar` | **Adopt** | High |
| Markdown parser (`runie-tui/src/markdown.rs`) | `pulldown-cmark` | `/pulldown-cmark/pulldown-cmark` | **Adopt parser, keep renderer** | Medium |
| Token estimation (`runie-core/src/tokens.rs`) | `tiktoken-rs` | `/zurawiki/tiktoken-rs` | **Adopt for known models, keep fallback** | Medium |
| Fuzzy matching (`runie-core/src/fuzzy.rs`) | `nucleo` | (cargo only) | **Evaluate** | Low |
| Text input (`runie-core/src/state.rs` `InputState`) | `tui-input` | `/sayanarijit/tui-input` | **Keep Custom** | Low |
| Channels / EventBus | `flume` | `/zesterer/flume` | **Keep tokio** | Low |
| OpenAI provider | `async-openai` | `/64bit/async-openai` | **Keep Custom** | Low |
| Output accumulator | — | — | **Keep Custom** | — |
| Path completion | — | — | **Keep Custom** | — |
| Keybindings | — | — | **Keep Custom** | — |
| Tool parser (`runie-agent/src/parser.rs`) | — | — | **Retire after LLMEvent** | Medium |

---

## 1. Syntax Highlighting → `syntect`

**Custom code:** `crates/runie-tui/src/syntax/`
- Hand-rolled keyword lists for Rust, Python, JS, TS, Go, Java, C, C++, SQL, Bash.
- ~10 language modules + a custom tokenizer.
- No real grammar support; misses strings/comments/types in many cases.

**Crate:** `syntect` — `/trishume/syntect`
- Uses Sublime Text syntax definitions.
- `SyntaxSet::load_defaults_newlines()` + `ThemeSet::load_defaults()`.
- `HighlightLines::highlight_line()` returns styled ranges.

**ctx7 snippet:**
```rust
let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
```

**Decision:** **ADOPT** `syntect`.

**Rationale:**
- Massively better language coverage and correctness.
- Already used by `bat`, `delta`, and many Rust TUIs.
- Runie's theme system can map syntect `Style` foreground colors to Ratatui `Color`.
- Custom syntax module is pure maintenance overhead.

**Migration notes:**
- Remove `crates/runie-tui/src/syntax/` entirely.
- Replace `highlight_code(code, lang)` with `syntect::easy::HighlightLines`.
- Map syntect 24-bit colors to the closest Ratatui color or use truecolor when terminal supports it.

**Follow-up task:** Create `tasks/adopt-syntect.md`.

---

## 2. Diff Generation → `similar`

**Custom code:** `crates/runie-agent/src/diff.rs`
- LCS-based diff with O(n·m) DP table.
- Builds unified diff hunks manually.
- ~400 lines.

**Crate:** `similar` — `/mitsuhiko/similar`
- `TextDiff::from_lines(old, new)`.
- Supports Myers, Patience, Hunt-McIlroy algorithms.
- Produces `ChangeTag::Delete/Insert/Equal`, unified diff, deadlines for large inputs.
- Dependency-free.

**ctx7 snippet:**
```rust
use similar::{ChangeTag, TextDiff};
let diff = TextDiff::from_lines(old, new);
for change in diff.iter_all_changes() {
    let sign = match change.tag() {
        ChangeTag::Delete => "-",
        ChangeTag::Insert => "+",
        ChangeTag::Equal => " ",
    };
}
```

**Decision:** **ADOPT** `similar`.

**Rationale:**
- Better algorithms, deadlines, and battle-tested (used by `insta`).
- Less code to maintain.
- Custom LCS diff is memory-hungry on large files.

**Migration notes:**
- Replace `generate_unified_diff` with `TextDiff::from_lines`.
- Keep `crates/runie-tui/src/diff.rs` parsing if we still need to render externally-produced diffs; otherwise use `similar`'s output directly.

**Follow-up task:** Create `tasks/adopt-similar.md`.

---

## 3. Markdown Parsing → `pulldown-cmark`

**Custom code:** `crates/runie-tui/src/markdown.rs`
- Inline parser for `**bold**`, `*italic*`, `` `code` ``.
- Block extractor for code fences, lists, blockquotes.
- ~325 lines.

**Crate:** `pulldown-cmark` — `/pulldown-cmark/pulldown-cmark`
- Event-based CommonMark + GFM parser.
- Supports tables, task lists, strikethrough, footnotes.

**ctx7 snippet:**
```rust
use pulldown_cmark::{Event, Parser, Tag, Options};
let mut options = Options::empty();
options.insert(Options::ENABLE_STRIKETHROUGH);
let parser = Parser::new_ext(markdown, options);
for event in parser { match event { Event::Text(t) => ... } }
```

**Decision:** **ADOPT `pulldown-cmark` for parsing; KEEP custom Ratatui renderer.**

**Rationale:**
- Runie needs to interleave tool-call cards inside the message stream. A
  generic markdown-to-ratatui crate cannot know about Runie tool states.
- `pulldown-cmark` gives us correct parsing; we keep the widget mapping layer.

**Migration notes:**
- Replace hand-rolled inline/block parsing with `Parser` events.
- Map events to existing `MdSpan` / `CodeBlock` types or new renderer types.
- Enables tables and GFM features for free.

**Follow-up task:** Create `tasks/adopt-pulldown-cmark.md`.

---

## 4. Token Estimation → `tiktoken-rs`

**Custom code:** `crates/runie-core/src/tokens.rs` `estimate_tokens`
- Approximation: `chars.div_ceil(4)`.
- Very inaccurate for code and non-English text.

**Crate:** `tiktoken-rs` — `/zurawiki/tiktoken-rs`
- Implements OpenAI tiktoken tokenizers.
- Supports `cl100k_base` (GPT-4 / GPT-3.5-turbo), `o200k_base` (GPT-4o / o-series).

**ctx7 snippet:**
```rust
use tiktoken_rs::cl100k_base;
let bpe = cl100k_base()?;
let tokens = bpe.encode_with_special_tokens(text);
let count = tokens.len();
```

**Decision:** **ADOPT `tiktoken-rs` for known OpenAI-compatible models; keep `chars/4` as fallback for unknown providers.**

**Rationale:**
- Accurate token counts are needed for context compaction and cost tracking.
- tiktoken-rs only covers OpenAI tokenizers; other providers (Anthropic, Gemini, local) need fallback heuristics.

**Migration notes:**
- Add `tiktoken-rs` to `runie-core` dependencies.
- Map provider/model to tokenizer name; fallback to approximation.
- Update `TokenTracker` to use real counts when available.

**Follow-up task:** Create `tasks/adopt-tiktoken-rs.md`.

---

## 5. Fuzzy Matching → `nucleo` (Evaluate)

**Custom code:** `crates/runie-core/src/fuzzy.rs`
- Simple scoring: char order, start bonus, word-boundary bonus, length penalty.
- ~75 lines.

**Crate:** `nucleo` (cargo: `nucleo = "0.5.0"`)
- Helix editor's fuzzy matcher.
- High performance, custom scoring, multi-threaded.

**ctx7:** `ctx7 library nucleo` returned the security scanner, not the Rust crate. The Rust crate exists on crates.io but is not well-indexed by Context7.

**Decision:** **EVALUATE** `nucleo`.

**Rationale:**
- Runie's fuzzy needs are small (command palette, model selector, @-refs). The custom matcher is adequate and testable.
- `nucleo` would improve performance for very large lists (e.g., 1000+ models) but adds complexity.

**Migration notes:**
- Try replacing `fuzzy_filter` with `nucleo` in a spike.
- Keep custom matcher if `nucleo` API is too invasive.

**Follow-up task:** Add evaluation step to `tasks/crate-replacement-audit.md` or create `tasks/spike-nucleo-fuzzy.md`.

---

## 6. Text Input → `tui-input` (Keep Custom)

**Custom code:** `crates/runie-core/src/state.rs` `InputState`
- Supports multi-line input, undo/redo, history, tab completion ghost text,
  bracketed paste.

**Crate:** `tui-input` — `/sayanarijit/tui-input`
- Provides `Input` + `InputRequest` for single-line editing.
- Supports cursor movement, word deletion, yank, crossterm/ratatui backends.

**ctx7 snippet:**
```rust
use tui_input::{Input, InputRequest};
let mut input: Input = "...".into();
input.handle(InputRequest::DeletePrevWord);
```

**Decision:** **KEEP CUSTOM** for now.

**Rationale:**
- `tui-input` is single-line only and has no history/multi-line support.
- Rebuilding Runie's multi-line + history behavior on top of `tui-input`
  would require almost as much code as the current implementation.
- Re-evaluate if a multi-line textarea crate matures.

---

## 7. Channels / EventBus → `flume` (Keep tokio)

**Custom code:** Will build `EventBus` on `tokio::sync::broadcast`.

**Crate:** `flume` — `/zesterer/flume`
- Multi-producer/multi-consumer channel, bounded/unbounded, sync/async.

**Decision:** **KEEP `tokio::sync::broadcast`**.

**Rationale:**
- tokio channels already cover all needs: broadcast, mpsc, watch.
- `flume` is excellent but adds a dependency without a compelling feature gap.

---

## 8. OpenAI Provider → `async-openai` (Keep Custom)

**Custom code:** `crates/runie-provider/src/openai.rs`

**Crate:** `async-openai` — `/64bit/async-openai`
- Full OpenAI API client.

**Decision:** **KEEP CUSTOM** provider layer.

**Rationale:**
- Runie supports 35+ providers with varying response shapes (Anthropic tool
  blocks, Ollama, Gemini, etc.).
- The `LLMEvent` normalization task requires one adapter that maps all of them
  to a single event vocabulary. `async-openai` cannot help with non-OpenAI
  shapes.

---

## 9. Output Accumulator (Keep Custom)

**Custom code:** `crates/runie-agent/src/accumulator.rs`
- Rolling tail buffer, temp-file fallback, line/byte counting, head/tail truncation.

**Decision:** **KEEP CUSTOM**.

**Rationale:**
- Highly application-specific. No crate provides exactly this combination of
  bounded memory, line-aware truncation, and temp-file spillover.

---

## 10. Path Completion (Keep Custom)

**Custom code:** `crates/runie-core/src/path_complete.rs`
- Directory reading, prefix filtering, hidden-file rules, sorting.

**Decision:** **KEEP CUSTOM**.

**Rationale:**
- Small, well-tested, and tailored to Runie's input behavior.

---

## 11. Keybindings (Keep Custom)

**Custom code:** `crates/runie-core/src/keybindings.rs`
- JSON loading, combo string parsing, default binding table.

**Decision:** **KEEP CUSTOM**.

**Rationale:**
- No small, focused crate found that maps combo strings to semantic events.
- Current implementation is adequate.

---

## 12. Tool Parser (Retire)

**Custom code:** `crates/runie-agent/src/parser.rs`
- Parses JSON tool calls and legacy `TOOL:name:arg` lines.

**Decision:** **RETIRE after `llm-event-normalization`**.

**Rationale:**
- Once providers emit `LLMEvent::ToolCallStart/InputDelta/End`, tool calls are
  first-class objects and no text parsing is needed.
- No crate replacement needed.

---

## Immediate Action Items

1. ~~Create adoption tasks for high-priority crates.~~ ✅ Done:
   - `tasks/adopt-syntect.md` — done
   - `tasks/adopt-similar.md` — done
   - `tasks/adopt-pulldown-cmark.md` — done
   - `tasks/adopt-tiktoken-rs.md` — done

2. ~~Update `Cargo.toml` files when implementing each adoption.~~ ✅ Done.

3. ~~Update `docs/SPEC.md` reference implementations table to mention adopted crates.~~ ✅
   `syntect`, `similar`, `pulldown-cmark`, and `tiktoken-rs` are now in use.

4. **Spike `nucleo`** before deciding on fuzzy matching replacement (tracked in
   `tasks/spike-nucleo-fuzzy.md`).

---

*Generated using `npx ctx7` against Context7 on 2026-06-13.*
