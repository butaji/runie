# Deep Crate-Replacement Analysis — Custom Code → Crates

**Date:** 2026-06-30  
**Goal:** Find every piece of custom Runie code that can be replaced by a crate, library, or OS feature, rank the replacements by Pareto impact, and plan them in `tasks/`.

**Method:** Seven read-only sub-agents scanned the workspace by domain (TUI, text/markdown, config/persistence, networking, process/IO, actors/concurrency, utilities). Findings were cross-checked with `ctx7` docs and with patterns in `~/Code/agents/{codex,goose,kimi-code,gptme,aider}`. No production code was changed.

**TUI design freeze:** Any replacement that touches rendering must keep the existing visual output — colors, layout, spacing, widgets, selection style, status bar shape, message bubbles, popup shape. Only behavior/implementation changes.

---

## 1. Top Pareto wins (consolidated ranked table)

| P | Finding | Files / lines | Replacement | Est. LOC removed | Risk | Borrow from agents | Task |
|---|---------|---------------|-------------|------------------|------|--------------------|------|
| P0 | Custom markdown AST (`MdInline`, `CodeBlock`, healer) | `crates/runie-core/src/markdown/{blocks,inline,heal,mod}.rs`, `crates/runie-tui/src/markdown_render.rs` | `pulldown-cmark` events + `tui-markdown` direct render | 700–900 | Medium | Codex/Goose route markdown through standard parsers | **new** `drop-custom-markdown-ast-for-pulldown-cmark-and-tui-markdown.md` |
| P0 | OpenAI-compatible provider stack (hand-rolled SSE, deltas, normalizer) | `crates/runie-provider/src/openai/{protocol,request,stream,normalize}.rs`, `crates/runie-provider/src/protocol.rs` | `async-openai` or `rig-core` with thin adapter | 1,200–1,500 | High | aider→`litellm`; Goose→typed streams | **new** `spike-evaluate-async-openai-or-rig-core-for-provider-stack.md` |
| P0 | Bash execution via `sh -c` string in `spawn_blocking` | `crates/runie-core/src/actors/io/ractor_io.rs:144-263`, `crates/runie-agent/src/tool/bash.rs` | `tokio::process::Command` + `shell-words` argv | 150 | Medium | Goose `subprocess.rs` | `parse-bash-commands-with-shell-words-in-ioactor.md` |
| P0 | Hand-rolled git repo/branch/origin detection | `crates/runie-core/src/actors/io/ractor_io.rs:278-373` | `git2` | 80–100 | Low | Goose/Codex use `git2` | `replace-custom-git-detection-with-git2.md` |
| P0 | Leader TCP line framing with raw 1024-byte buffer | `crates/runie-core/src/actors/leader/actor.rs:193-268` | `tokio_util::codec::LinesCodec` or `BufReader::lines()` | 50–80 | Low-Medium | Goose uses `FramedRead`+`LinesCodec` | `buffer-tcp-lines-in-leader-server.md` |
| P0 | Lossy `EventBus` with no backpressure/ACK | `crates/runie-core/src/bus.rs` | bounded fan-out / `watch` for control / acknowledgement | small, correctness | High | Codex lossless/best-effort tiering | `add-eventbus-backpressure-or-acknowledgement.md` |
| P0 | Config load-modify-save strips comments; process-level `RwLock` | `crates/runie-core/src/config/config_impl.rs`, `provider/config.rs`, `actors/config/file_helpers.rs` | `toml_edit::DocumentMut` + `fs2` advisory locks | 200–350 | Medium | Codex `ConfigEdit` + `DocumentMut` | `use-toml-edit-and-file-locks-for-config.md` |
| P1 | Hand-rolled layered config merge | `crates/runie-core/src/config/layers.rs` | `figment` | 60–100 | Low | Codex config layers | `replace-layered-config-merge-with-figment.md` |
| P1 | Session JSONL + `fs2` advisory-lock store | `crates/runie-core/src/session/{store,index,replay,tree}.rs` | `rusqlite` or `redb` | 600–900 | Medium-High | Goose `sqlx`+SQLite; Codex JSONL+SQLite mirror | existing tasks; consider **new** migration task |
| P1 | Credential resolver still re-parses `.env` and duplicates priority logic | `crates/runie-core/src/auth/credential.rs`, `provider/config.rs` | `figment::Env` / `envy` + keyring | 80–120 | Low | Goose `SecretStorage` | **new** `consolidate-provider-credential-resolver-duplication.md` |
| P1 | FFF indexer wrapper + `notify` version conflict | `crates/runie-core/src/actors/fff_indexer/` | `ignore`/`walkdir` + `nucleo` + workspace `notify` | 300 | Medium | Goose `ignore` | **new** `replace-fff-search-indexer-with-ignore-walkdir.md` |
| P1 | Custom path normalization | `crates/runie-core/src/path.rs:10-45` | `std::path::absolute` / `path-absolutize` / `dunce` | 60–90 | Low | Codex `absolute-path` | **new** `normalize-paths-with-path-absolutize-and-dunce.md` |
| P1 | Legacy diff parser / `HunkBuilder` | `crates/runie-core/src/diff/mod.rs:203-319` | `diffy::Patch::from_str` + normalization | 120 | Medium | aider `difflib` | **new** `drop-legacy-diff-parser-and-rely-on-diffy.md` |
| P1 | Hand-rolled frontmatter splitter | `crates/runie-core/src/resource_loader.rs:90-202` | `pulldown-cmark-frontmatter` | ~120 | Low | Codex skills use frontmatter libs | `use-pulldown-cmark-frontmatter-for-resource-loader.md` |
| P1 | Think-filter 347-line state machine | `crates/runie-agent/src/think_filter/mod.rs` | `regex` (compiled once) | ~300 | Low-Medium | gptme/aider use regex tags | `replace-think-filter-with-regex.md` |
| P1 | Duplicate tool-call accumulators (`ToolStream`, `ToolAccum`, `ToolAccumulator`) | `crates/runie-core/src/tool_stream.rs`, `runie-provider/src/openai/protocol.rs`, `runie-provider/src/protocol.rs` | single `HashMap<String, Accumulator>` | 100–150 | Low | Goose streaming parser | **new** `unify-tool-call-accumulators-into-single-state-machine.md` |
| P2 | Streaming-buffer stable-line classifier | `crates/runie-core/src/streaming_buffer.rs:89-205` | `pulldown-cmark` sliding window | ~120 | Medium | Codex re-renders full buffer | **new** `replace-streaming-buffer-classifier-with-pulldown-cmark.md` |
| P2 | `file:line:col` parser in `location.rs` | `crates/runie-core/src/location.rs:20-123` | `regex` | ~100 | Low | Codex regex fragment parser | **new** `replace-location-parser-with-regex.md` |
| P2 | `which` shells out to `/usr/bin/which` | `crates/runie-core/src/tool/format.rs:6-24` | `which` crate | ~25 | Low | Goose uses `which` | **new** `use-which-crate-for-executable-lookup.md` |
| P2 | Core keybinding combo parser | `crates/runie-core/src/keybindings/mod.rs:22-113` | `crokey` parsing | ~60 | Low | Goose/Codex use `crokey` | **new** `simplify-core-keybinding-parsing-with-crokey.md` |
| P2 | `format_bytes` / `format_duration` | `crates/runie-core/src/tool/format.rs:115-138` | `bytesize` / `humantime` | ~25 | Low | standard CLI pattern | **new** `use-bytesize-humantime-for-tool-formatting.md` |
| P2 | Input-history prefix/substring search | `crates/runie-core/src/input_history.rs:112-141` | `sublime_fuzzy` / `nucleo-matcher` | ~30 | Low | common fuzzy history | **new** `fuzzy-history-search-with-sublime-fuzzy.md` |
| P2 | Message sanitization pipeline + chars/4 token heuristic | `crates/runie-core/src/sanitize.rs`, `tokens.rs` | typed `ChatMessage` builder + `tiktoken-rs` | 200–250 | Medium | Codex model-specific tokenizers | `replace-token-chars4-heuristic-with-tiktoken-rs.md`, **new** `typed-chatmessage-builder-and-shrink-sanitize.md` |
| P2 | Lifecycle state machine synthesizes Start/End events | `crates/runie-core/src/lifecycle.rs` | fold into provider-event generation | 120–150 | Low | `rig-core` content deltas | **new** `fold-lifecycle-state-machine-into-provider-events.md` |
| P2 | File walking / glob matching in `file_refs.rs` | `crates/runie-core/src/file_refs.rs:120-205` | `ignore` / `globset` | ~100 | Low-Medium | Goose `ignore` | **new** `use-ignore-and-globset-for-file-refs.md` |
| P2 | Clipboard subprocess glue (pbcopy/wl-copy/xclip/clip) | `crates/runie-core/src/actors/io/effects/clipboard.rs`, `crates/runie-tui/src/terminal/clipboard.rs` | `arboard` + OSC 52 fallback | 180–220 | Low-Medium | Codex `arboard`+OSC52 | `replace-clipboard-dead-code-with-arboard-or-osc52.md` |
| P2 | Actor handle wrappers (`RactorHandle`, per-actor handles) | `crates/runie-core/src/actors/ractor_adapter.rs`, `leader/handle.rs` | direct `ractor::ActorRef` / typed map | 500–900 | Medium | Goose/Codex raw channels | `actually-collapse-actor-handles-to-typed-map.md`, `delete-dead-actor-handle-wrappers.md` |
| P2 | Generated event taxonomy (`event_enum.rs`, `kind.rs`, etc.) | `crates/runie-core/src/event/generated/*.rs` | `strum` derives + real codegen | 1,000–1,500 | Medium | standard strum pattern | `collapse-event-intent-kind-taxonomies.md`, `use-strum-for-event-intent-names.md` |
| P2 | Static tool dispatch table | `crates/runie-agent/src/tool_runner.rs`, `headless/mod.rs`, `turn/mod.rs` | `inventory`/`linkme` or `Vec<Box<dyn ToolDef>>` | 80–120 | Low-Medium | Goose MCP dynamic reg | consider **new** task |
| P3 | `Token` wrapper over `secrecy::SecretString` | `crates/runie-core/src/auth/storage.rs` | `secrecy::SecretString` directly | ~35 | Low | Codex `keyring-store` | `replace-token-wrapper-with-secrecy-secretstring.md` |
| P3 | Custom nanoid in test runner | `crates/runie-testing/src/runner.rs` | `uuid` or `nanoid` crate | ~10 | Low | standard | `replace-custom-nanoid-in-runie-testing.md` |
| P3 | Theme ANSI-256→16 lookup table | `crates/runie-tui/src/quantize.rs` | nearest ANSI-16 distance or terminal down-conversion | ~85 | Low | standard | `adopt-palette-for-theme-color-math.md` (wontfix) |
| P3 | `format_bytes`/`format_duration` (above) | — | — | — | — | — | — |

---

## 2. `ctx7` / crate documentation highlights

| Crate | Why it fits | Key API / evidence |
|-------|-------------|--------------------|
| `figment` | Semi-hierarchical config layering; supports `Toml`, `Env`, `Serialized` providers and `Jail` for tests. | `Figment::from(Serialized::defaults(...)).merge(Toml::file(...)).merge(Env::prefixed("RUNIE_"))` (ctx7 confirmed). |
| `toml_edit` | Comment-preserving, formatting-preserving TOML edits. | Rust ecosystem standard; Codex uses an analogous `ConfigEdit` + `DocumentMut` pattern. |
| `tiktoken-rs` | Accurate OpenAI token counts. | `bpe_for_model(model)?.encode_with_special_tokens(text)` and `num_tokens_from_messages`. |
| `arboard` | Cross-platform clipboard (text + image). | `Clipboard::new()?.set_text(...)` / `get_text()`; widely used by Codex. |
| `git2` | Reliable repo/branch/origin detection. | `Repository::discover(path)`, `repo.head()?.shorthand()`, `find_remote("origin")`. |
| `sqlx` (SQLite) | Rich sessions with migrations, WAL, legacy import. | Goose `session_manager.rs` is the reference implementation. |
| `ignore` | Gitignore-aware file walking; replaces FFF indexer and `file_refs` recursion. | `WalkBuilder::new(path)` with glob/ignore overrides. |
| `async-openai` | Typed OpenAI API + SSE streaming. | `ChatCompletionRequestMessage`, streaming `ChatCompletionChunk`. |
| `rig-core` | Provider-agnostic agents/completions; multi-provider. | `CompletionModel`, `Agent` abstractions; supports reasoning content. |

---

## 3. Cross-agent patterns to copy

- **Codex:**
  - `ConfigEdit` enum + `toml_edit::DocumentMut` for atomic, comment-preserving config writes.
  - `keyring-store` trait backed by `secrecy` for testable secrets.
  - Lossless vs best-effort event tiering for the TUI/core bus.
  - `FrameRequester` to coalesce redraws and clamp FPS.
- **Goose:**
  - `sqlx` + SQLite for sessions with migrations + legacy JSONL import.
  - `tokio::process::Command` + process-group hygiene for subprocesses.
  - `ignore` for gitignore-aware file discovery.
  - `git2` for repo metadata.
- **kimi-code:**
  - Recorded vs live-only event split for deterministic replay.
  - Rich permission UI display blocks (diff, file content, shell).
- **gptme / aider:**
  - Regex-based tool-marker / thinking-tag stripping.
  - Standard diff libraries for patch application.

---

## 4. What NOT to replace

| Area | Reason |
|------|--------|
| `tokio::sync::broadcast` | The primitive is right; the wrapper/backpressure is the problem. |
| Small domain state machines (`LifecycleState`, `AgentPhase`) | No crate knows Runie’s turn lifecycle. |
| DSL runtime / hook registry | Domain-specific; not readily replaceable. |
| `indextree` arena | Already a standard crate; switching arenas is not a reduction. |
| Theme palette color math | Existing `wontfix` task: helpers are small and correct. |

---

## 5. Recommended execution order

1. **Low-risk correctness fixes first:** `git2`, `tokio-util::LinesCodec` leader framing, `shell-words` bash parsing, `which` lookup.
2. **Config/auth consolidation:** `figment`, `toml_edit`+`fs2`, `secrecy`/`keyring`, env/dotenv cleanup.
3. **Text/markdown simplification:** `pulldown-cmark-frontmatter`, regex think-filter, drop custom markdown AST (huge LOC win but needs snapshot parity).
4. **Provider stack spike:** evaluate `async-openai`/`rig-core` behind an adapter; keep existing provider-replay E2E tests as the contract.
5. **Big persistence refactor:** migrate sessions to SQLite/`redb` with a JSONL import path.
6. **TUI widget replacements:** `arboard`, `tui-textarea`/`tui-input`, `ratatui::List` — all behind visual-freeze snapshot tests.
7. **Cleanup:** actor-handle collapse, event taxonomy `strum`, remaining `std::sync` locks → `parking_lot`.

---

## 6. New tasks created in this pass

1. `drop-custom-markdown-ast-for-pulldown-cmark-and-tui-markdown.md`
2. `normalize-paths-with-path-absolutize-and-dunce.md`
3. `drop-legacy-diff-parser-and-rely-on-diffy.md`
4. `use-ignore-and-globset-for-file-refs.md`
5. `replace-streaming-buffer-classifier-with-pulldown-cmark.md`
6. `replace-location-parser-with-regex.md`
7. `use-which-crate-for-executable-lookup.md`
8. `simplify-core-keybinding-parsing-with-crokey.md`
9. `use-bytesize-humantime-for-tool-formatting.md`
10. `fuzzy-history-search-with-sublime-fuzzy.md`
11. `unify-tool-call-accumulators-into-single-state-machine.md`
12. `fold-lifecycle-state-machine-into-provider-events.md`
13. `typed-chatmessage-builder-and-shrink-sanitize.md`
14. `consolidate-provider-credential-resolver-duplication.md`
15. `spike-evaluate-async-openai-or-rig-core-for-provider-stack.md`
16. `replace-fff-search-indexer-with-ignore-walkdir.md`

---

## 7. File inventory

- This analysis: `docs/superpowers/plans/2026-06-30-crate-replacement-deep-dive.md`
- Prior synthesis: `docs/superpowers/plans/2026-06-30-five-round-architecture-review.md`
- Roadmap: `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md`
- Tasks: `tasks/*.md`
