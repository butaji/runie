# Fourth-Pass Architecture & Code Review — 2026-06-28

This document captures the findings from the fourth five-round architecture and code review. The goal is the same as the previous passes: less code, Pareto (80/20) choices, unification, and replacing custom code with mature crates or OS features where possible.

The fourth pass concentrated on five areas:

1. **Provider / model catalog / cache**
2. **Session, store, index, and replay**
3. **Agent turn, subagent, tool, and search**
4. **TUI capabilities, diff, message, and markdown rendering**
5. **DSL, view, dialog, commands, and testing**

It builds on the earlier three passes recorded in [`2026-06-28-runie-cleanup-roadmap.md`](2026-06-28-runie-cleanup-roadmap.md), [`2026-06-28-less-code-crate-replacements.md`](2026-06-28-less-code-crate-replacements.md), and [`2026-06-28-third-pass-crate-review.md`](2026-06-28-third-pass-crate-review.md).

---

## 1. Provider / model catalog / cache

### Findings

- Provider and model configuration is largely **stringly typed**. Model ids, context windows, tokenizer assumptions, and pricing are parsed repeatedly in each provider.
- Each provider reimplements **SSE/JSON streaming parsing**, delta accumulation, and tool-call fragment assembly.
- Retry logic is custom (`runie-provider/src/retry.rs`) despite `backon`/`reqwest-retry` being available.
- Token storage is XOR-obfuscated instead of using the OS keyring.
- Config validation is hand-rolled instead of validating against the generated JSON schema.
- Credential resolution loads `.env` in multiple places and falls back through inconsistent env-var chains.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Stringly typed provider/model config | Typed `Provider`/`Model` structs + one model catalog | `type-and-unify-provider-model-layer` |
| Per-provider streaming parsers | Shared streaming response parser in `runie-provider` | `extract-shared-streaming-response-parser` |
| Custom retry module | `backon` / `reqwest-retry` | `replace-custom-retry-with-backon` |
| XOR-obfuscated tokens | `keyring` + headless fallback | `replace-xor-auth-with-keyring` |
| Custom config validator | `jsonschema` against `schemars` output | `replace-config-validator-with-jsonschema` |
| Scattered `.env` loading | `dotenvy` once in `ConfigActor` | `unify-provider-credential-resolution-with-dotenvy` |

### Key design points

- The model catalog should be **plain data** (YAML/JSON) embedded with `include_str!` so adding a model does not require a code change.
- Provider-specific behavior (MiniMax XML delimiters, Anthropic thinking blocks, etc.) stays in `runie-provider`, but it consumes a shared parser and typed catalog.
- Retry must cover pre-stream HTTP failures; once the SSE stream starts, retry is handled at the turn level, not the byte level.

---

## 2. Session, store, index, and replay

### Findings

- Session persistence is split between **ad-hoc JSON file dumps** and a custom replay index.
- The replay index is hand-managed rather than queried.
- Session actor, store, and index logic live in separate places but operate on the same data.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| JSON session store + custom replay index | Single headered JSONL file with `fs2` advisory locks (SQLite deferred) | `unify-session-store-and-index` |

### Key design points

- Use `rusqlite` with the `bundled` feature.
- Schema migrations are versioned and applied automatically.
- Sessions, messages, checkpoints, and the replay index all live in one database.
- Replay becomes a SQL query plus event replay, not a custom index walk.

---

## 3. Agent turn, subagent, tool, and search

### Findings

- The turn queue and event dispatch layer have **overlapping buffering/dedup logic**, producing stale indices, duplicate `TurnComplete` events, and leaked in-flight turns.
- Subagent results are collected via callbacks or polling, which complicates lifetimes and races.
- Tool parsing is mostly shimmed, but think-block filtering still uses custom streaming string matching.
- The `inspect` command duplicates the tool-call display pipeline.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Overlapping queue/dispatch | Single queue with explicit delivery ids | `dedupe-turn-queue-delivery-logic` |
| Callback/polling subagent results | `tokio::sync::mpsc` / `oneshot` channels | `use-channels-for-subagent-result-collection` |
| Custom `<think>` matcher | `regex` (and eventually the shared markdown stream) | `replace-think-filter-with-regex` |
| Separate inspector tool pipeline | Merge with shared tool loop or delete | `delete-or-merge-inspector-tool-pipeline` |

### Key design points

- The queue should be actor-independent and testable without `ractor`.
- Use bounded channels for subagent backpressure; drop/timeout must be explicit.
- Think-block stripping should happen after markdown processing, in one place.

---

## 4. TUI capabilities, diff, message, and markdown rendering

### Findings

- Markdown is parsed in multiple places with custom regex/string slicing: tool-marker stripping, frontmatter, diff/message rendering, and think blocks.
- Terminal capability detection (`NO_COLOR`, color level, hyperlinks) is custom and duplicated.
- Line wrapping / line-count logic exists in both `runie-core` and `runie-tui`.
- `DialogState` has overlapping variants and ad-hoc routing.
- Custom TUI widgets (`Stylize`, input box, popup list, terminal setup, ANSI quantization) should already be handled by the third-pass TUI widget task, but the review confirmed they are still present and should be removed.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Scattered markdown parsers | Shared `pulldown-cmark` event stream | `unify-markdown-processing-around-pulldown-cmark` |
| Custom terminal capability detection | `supports-color` + `supports-hyperlinks` | `simplify-terminal-capability-detection` |
| Duplicate line wrapping | Single helper in `runie-core` or `textwrap` | `unify-core-and-tui-line-count-computation` |
| Overlapping `DialogState` variants | Small state machine with ≤4 variants | `collapse-dialogstate-variants` |
| Custom TUI widgets | `tui-textarea`, `tui-input`, `ratatui::widgets::List`, `ansi_colours` | `replace-custom-tui-widgets-with-ratatui-ecosystem` |

### Key design points

- The markdown helper in `runie-core` is the single source of truth; TUI imports it.
- `TermCaps` is computed once at startup and passed down; no env parsing in render code.
- `DialogState` transitions should be a pure function, not scattered `if` branches.

---

## 5. DSL, view, dialog, commands, and testing

### Findings

- The declarative resource loader still has two frontmatter/body scanners; unification is already planned in earlier passes.
- `CommandSpec`/`CommandDef` duplication is already tracked.
- Permission rule engines (`PermissionSet`/`PermissionManager`) are still separate.
- Test fixtures are scattered across crates; mock providers and replay helpers should live in `runie-testing`.
- The custom build linter is still in place; replacing it with Clippy/CI is tracked.

### Recommended unification

These items are covered by existing tasks:

- `unify-declarative-resource-loader`
- `use-pulldown-cmark-frontmatter-for-resource-loader`
- `simplify-slash-command-dsl`
- `unify-permission-system-rules`
- `centralize-test-fixtures-and-mocks`
- `replace-build-linter-with-clippy-ci`

No new tasks were added in this area, but the fourth pass confirms they remain high-value.

---

## New task list

The fourth pass added the following tasks to `tasks/index.json`:

| Task | Priority | Area |
|------|----------|------|
| `unify-session-store-and-index` | P0 | Sessions — single headered JSONL + `fs2` locks; SQLite deferred. |
| `unify-markdown-processing-around-pulldown-cmark` | P0 | Core / State |
| `dedupe-turn-queue-delivery-logic` | P1 | Architecture / Actors |
| `type-and-unify-provider-model-layer` | P0 | Configuration |
| `extract-shared-streaming-response-parser` | P1 | Core / State |
| `delete-or-merge-inspector-tool-pipeline` | P1 | Tools |
| `simplify-terminal-capability-detection` | P2 | TUI / Rendering |
| `unify-core-and-tui-line-count-computation` | P2 | Core / State |
| `collapse-dialogstate-variants` | P2 | TUI / Rendering |
| `replace-think-filter-with-regex` | P1 | Core / State |
| `use-channels-for-subagent-result-collection` | P0 | Architecture / Actors |

One previously marked-done task, `replace-custom-helpers-with-crates`, was reopened because its acceptance criteria (actual deletion of `glob.rs`, `fuzzy.rs`, `path.rs`, and custom keybinding parsing) are not yet satisfied on disk.

---

## Execution notes

- The new tasks are independent of the big actor/bootstrap sequence and can land once the earlier crate-replacement foundation is in place.
- `unify-markdown-processing-around-pulldown-cmark` should land before `replace-think-filter-with-regex` so think-block stripping can reuse the shared markdown stream.
- `type-and-unify-provider-model-layer` should land before `extract-shared-streaming-response-parser` so the parser can operate on typed model metadata.
- `dedupe-turn-queue-delivery-logic` should land before `use-channels-for-subagent-result-collection` so subagent results feed a clean queue.
- Every task must include the 4-layer test coverage required by `AGENTS.md`.

## Cross-agent lessons

Peer codebases (`goose`, `jcode`, `OpenFang`, `thClaws`) reinforce the same choices:

- Use `rusqlite` or JSONL with file locking for session persistence; SQLite gives indexing and replay for free.
- Keep provider/model metadata as typed data, not strings.
- Centralize streaming parsing; do not let every provider grow its own state machine.
- Use OS keyrings for secrets; fall back to environment or headless files only in CI.
- Use standard terminal capability crates rather than env-variable heuristics.
