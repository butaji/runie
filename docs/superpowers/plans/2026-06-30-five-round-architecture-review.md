# Five-Round Architecture & Code Review — Synthesis

**Date:** 2026-06-30  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Replace custom code with crates/libraries/OS features where it clearly reduces maintenance.

Five read-only review agents covered the workspace, plus `ctx7` lookups and a read-only survey of `~/Code/agents/{kimi-code,goose,codex}` for cross-pollination. No code was executed or modified.

---

## 1. Per-round scope and top findings

### Round 1 — TUI / rendering / input / keymap / widgets / themes
**Highest-value:**
1. `UiActor` still mutates `AppState` directly (input, dialogs, submit, autocomplete). Route through `InputActor`/`TurnActor`/`DialogMsg`.
2. TUI subscribes to the event bus after `Leader::start`, dropping `ConfigLoaded`/`TrustLoaded`/`HistoryLoaded`.
3. Permission dialog keys are routed to the input box; arrows/Esc accidentally deny.
4. Custom input box while `tui-textarea`/`tui-input` are already in `Cargo.toml`.
5. Custom list/popup rendering instead of `ratatui::widgets::List`.
6. `app_init::bootstrap` blocks on skill/auth loading and writes directly to `AppState`.
7. Theme is set via global mutable state on every frame.
8. `terminal/clipboard.rs` is dead code; clipboard effects go through `IoActor`.

### Round 2 — Actors / event bus / state / commands / dialogs
**Highest-value:**
1. Direct `AppState` mutation across command/update handlers and `UiActor`.
2. Late TUI subscription drops initial facts (same as R1, confirmed cross-round).
3. `EventBus` is lossy broadcast with no backpressure/ACK.
4. `RactorProviderActor` blocks its mailbox on network IO (`validate_key`, `list_models`).
5. `LeaderHandle::shutdown()` can panic via `Arc::try_unwrap().expect` and does not await all actors.
6. Turn state is duplicated between `TurnActor` and `AppState::AgentState`.
7. Command/update handlers still perform blocking file IO.
8. Permission approval sink has no cancellation path.

### Round 3 — Provider / agent loop / streaming / tools / permissions / headless
**Highest-value:**
1. `AgentActor` runs the entire provider turn inside the ractor `handle`.
2. Headless turn reimplements streaming/tool-assembly logic already in `stream_response`.
3. OpenAI protocol maintains its own tool accumulator instead of `ToolStream`.
4. `EmitApprovalSink` blocks the turn with no cancellation path.
5. Subagents and headless bypass user permission rules (`AutoAllowSink`).
6. `EventBus` lossy broadcast (same as R2).
7. `LeaderHandle::shutdown` panic + incomplete await (same as R2).
8. Leader TCP server uses raw 1024-byte buffer reads and can split UTF-8.

### Round 4 — CLI / config / sessions / persistence / auth / inspect
**Highest-value:**
1. Direct `AppState` mutation + blocking disk IO inside session slash commands.
2. `Config::save_nonblocking` runs blocking `std::fs` inside `tokio::spawn`.
3. Custom `.env` re-parser in `CredentialResolver` duplicates `dotenvy`.
4. Config edits are load-modify-save with `toml`, stripping comments; use `toml_edit` + `fs2` file locks.
5. Custom layered-config merge can be replaced by `figment` or `config`.
6. `runie-headless server` creates a fresh `HeadlessRuntime` per request.
7. Leader TCP server reimplements framing instead of reusing CLI transport.
8. Session index path is computed from the wrong base directory.

### Round 5 — Workspace / deps / build / CI / helpers / MCP leftovers
**Highest-value:**
1. `crates/runie-tui/src/ui_actor.rs` exceeds the advertised 500-line limit.
2. Multiple unused direct dependencies (`cargo machete`): `tui-textarea`, `ractor` in TUI, `tracing-subscriber` in TUI, `runie-provider` in CLI, etc.
3. `build.rs` is a hand-rolled regex linter that does not enforce the advertised function/complexity limits.
4. `scripts/tmux-smoke-test.sh` is a shell/tmux test full of `sleep`, violating AGENTS.md anti-patterns.
5. MCP runtime is scaffolding-only: config persistence with no actual client/server runtime.
6. Event taxonomy is hand-written "generated" code with no real generator.
7. `resource_loader.rs` reimplements YAML frontmatter parsing instead of using `pulldown-cmark-frontmatter`.
8. `ThinkFilter` is a 347-line custom tag state machine; can be a regex transform.

---

## 2. Consolidated ranked list

| P | Finding | Simplification / crate replacement | Task file |
|---|---------|------------------------------------|-----------|
| P0 | Direct `AppState` mutation in TUI/command handlers | Route through actor messages/facts | `remove-direct-appstate-mutation-from-tui-handlers.md` |
| P0 | Late TUI subscription drops initial facts | Subscribe before `Leader::start` or add replay buffer | `subscribe-tui-to-initial-facts-before-leader-start.md` |
| P0 | Agent/provider/network work inside actor handlers | Spawn `tokio` tasks inside `handle()` | `offload-provider-network-calls-from-actor-handler.md` |
| P0 | `LeaderHandle::shutdown` panic/incomplete await | Shutdown signal + await all join handles | `fix-leader-shutdown-to-await-all-actors.md` |
| P1 | Lossy `EventBus` with no backpressure | Add ACK or bounded fan-out | `add-eventbus-backpressure-or-acknowledgement.md` |
| P1 | Permission approval sink no cancellation | `tokio_util::sync::CancellationToken` + `CancelPermission` | `shorten-approval-sink-timeout-and-wire-cancellation.md` |
| P1 | Blocking IO in command/update handlers | Route through `IoActor`/`SessionActor` | `move-blocking-io-out-of-command-handlers.md` |
| P1 | `Config::save_nonblocking` blocks runtime | `spawn_blocking` consistently, delete sync fallback | `route-cli-config-through-configactor.md` / new |
| P1 | Custom `.env` parser | `dotenvy::from_filename_iter` / `envy` | `unify-provider-credential-resolution-with-dotenvy.md` |
| P1 | Config edits strip comments | `toml_edit` + `fs2` advisory locks | **new** `use-toml-edit-and-file-locks-for-config.md` |
| P1 | Hand-rolled layered config merge | `figment` (recommended) or `config` | **new** `replace-layered-config-merge-with-figment.md` |
| P1 | Headless server per-request runtime | Shared `HeadlessRuntime` | `unify-headless-runtime-bootstrap.md` |
| P1 | Leader TCP raw buffer / manual framing | `tokio::io::BufReader::lines()` or `jsonrpsee` | `buffer-tcp-lines-in-leader-server.md`, `unify-cli-json-rpc-transport-and-remove-dead-acp.md` |
| P1 | Session index base-dir bug | Pass `dirs::data_dir()` / delete index reads | `compare-session-persistence-resumption-and-fix-gaps.md` |
| P1 | Duplicate turn state (`TurnActor` vs `AgentState`) | Make `TurnActor` SSOT, projection read-only | **new** `deduplicate-turn-state-between-turnactor-and-appstate.md` |
| P1 | Headless/subagents bypass permission rules | Pass `PermissionSet` into gate | `wire-user-permission-rules-into-agent-gate.md` |
| P2 | Unused deps / duplicate crate versions | `cargo machete`, align `notify`, drop `tui-textarea` | `remove-unused-dependencies-and-normalize-workspace-deps.md` |
| P2 | `ui_actor.rs` over 500 lines | Split or finish actor refactor | `fix-500-line-file-limit-violations.md` |
| P2 | Hand-rolled `build.rs` linter | `clippy` lints + `xtask`/CI file-limit check | `replace-build-linter-with-clippy-ci.md`, `enforce-advertised-file-function-complexity-limits.md` |
| P2 | MCP scaffolding with no runtime | Wire a real MCP client or delete the CLI/config | `implement-or-remove-mcp-runtime-scaffolding.md` |
| P2 | Hand-written event taxonomy | Real codegen or `strum` derives | `collapse-event-intent-kind-taxonomies.md`, `use-strum-for-event-intent-names.md` |
| P2 | Custom input box | `tui-textarea` / `tui-input` | `replace-custom-tui-widgets-with-ratatui-ecosystem.md` |
| P2 | Custom list rendering | `ratatui::widgets::List` | `replace-custom-tui-widgets-with-ratatui-ecosystem.md` |
| P2 | Custom frontmatter parser | `pulldown-cmark-frontmatter` | `use-pulldown-cmark-frontmatter-for-resource-loader.md` |
| P2 | `ThinkFilter` custom state machine | Streaming regex | `replace-think-filter-with-regex.md` |
| P2 | `Token` wrapper over `secrecy` | Use `secrecy::SecretString` directly | **new** `replace-token-wrapper-with-secrecy-secretstring.md` |
| P2 | Theme global static + per-frame set | Cache in `Snapshot`, apply on facts | `avoid-setting-global-theme-every-frame.md` |
| P2 | Dead `terminal/clipboard.rs` | `arboard` or `crossterm` osc52 | **new** `replace-clipboard-dead-code-with-arboard-or-osc52.md` |
| P2 | Token estimation chars/4 | `tiktoken-rs` / `tokenizers` (optional) | **new** `replace-token-chars4-heuristic-with-tiktoken-rs.md` |
| P2 | Keymap modifier dispatch tables | `HashMap<crokey::KeyCombination, CoreEvent>` | **new** `replace-keymap-modifier-tables-with-crokey-map.md` |
| P2 | Bash `sh -c` string | `shell-words` + `tokio::process` in `IoActor` | **new** `parse-bash-commands-with-shell-words-in-ioactor.md` |
| P2 | Git detection custom parser | `git2` | **new** `replace-custom-git-detection-with-git2.md` |
| P2 | Custom ANSI quantization tables | `ansi_colours` | `adopt-palette-for-theme-color-math.md` |
| P2 | Permission dialog raw JSON render | Pre-format in `Snapshot` | `refine-permission-dialog-key-handling.md` |
| P3 | `CommandCategory` hand-written `FromStr` | `strum::EnumString` | `replace-remaining-custom-parsers-and-macros-with-strum.md` |
| P3 | `McpServer.scope` String | `ConfigScope` + `clap::ValueEnum` | **new** `use-configscope-enum-for-mcp-server-scope.md` |
| P3 | `RactorHandle::send`/`send_message` alias | Delete alias | **new** `remove-ractor-send-message-alias.md` |
| P3 | `InspectReport::discover_config_sources` HashSet | Two `Option` comparisons | `route-cli-config-through-configactor.md` |
| P3 | Custom nanoid in testing | `uuid` or `nanoid` crate | **new** `replace-custom-nanoid-in-runie-testing.md` |
| P3 | `grep_find.rs` root test module | Move to `tests/` | **new** `move-grep-find-tests-to-tests-directory.md` |

---

## 3. Crate-replacement decisions (ctx7 + ecosystem)

| Custom code | Candidate | Verdict | Notes |
|-------------|-----------|---------|-------|
| Layered config merge | `figment` | **Adopt** | Semi-hierarchical, typed, preserves source precedence, supports TOML + env. ctx7 confirmed idiomatic API. |
| TUI input box | `tui-textarea` / `tui-input` | **Evaluate** | `tui-textarea` is already a dependency but unused; decide whether its UX matches Runie or remove it. `tui-input` gives a lower-level backend. |
| Config edits | `toml_edit` | **Adopt** | Preserves comments/formatting; pair with `fs2` file locks for cross-process safety. |
| Session store | `rusqlite` / `redb` | **Defer** | JSONL + `fs2` is simpler and sufficient for local use; revisit only when search/listing at scale becomes a requirement. Codex/Goose both use SQLite, but that is a large migration. |
| Token counting | `tiktoken-rs` | **Optional adopt** | Only if token counts are used for truncation/cost; keep chars/4 for UI estimates with documentation. |
| Markdown streaming | `pulldown-cmark` events | **Already planned** | Continue unifying markdown processing. |
| ANSI quantization | `ansi_colours` / palette | **Evaluate** | `adopt-palette-for-theme-color-math.md` exists; verify it covers ANSI256→16 round-trip. |
| Clipboard | `arboard` | **Adopt** | Cross-platform; replace dead `terminal/clipboard.rs`. `crossterm` osc52 is fine as fallback. |
| JSON-RPC transport | `jsonrpsee` / `tarpc` | **Defer** | First unify leader TCP on `BufReader::lines()` and the CLI envelope; only migrate to `jsonrpsee` if the protocol grows. |
| Git detection | `git2` | **Adopt** | Already a workspace transitive dependency; removes custom `.git/HEAD` parser. |
| Bash parsing | `shell-words` | **Adopt** | Already in dependency tree; use with `tokio::process`. |
| Fuzzy palette | `sublime_fuzzy` | **Adopt** | Already a workspace dependency. |
| Event names | `strum::IntoStaticStr` | **Adopt** | Already a dependency; collapse manual name tables. |
| Locking | `parking_lot` | **Continue** | Task exists to normalize remaining `std::sync` locks. |

---

## 4. Cross-agent learnings

### Kimi Code
- First-class **goal mode** with budgets and auto-continuation.
- Split events into **recorded vs live-only** for deterministic replay.
- Rich permission UI with display blocks (`diff`, `file_content`, `shell`) and session-scoped approvals.
- Modal coordinator for stacked dialogs.
- Kitty-keyboard printable-key guard.

### Goose
- Single `Agent` + stream consumer, simpler than Runie's actor graph.
- `ToolPermissionStore` with per-call arg hashes and expiry.
- Explicit tool allow/ask/deny lists in YAML.
- LLM-based read-only classifier (`permission_judge.rs`).
- `keyring` for secrets; `sqlx` for sessions; `tokio-cron-scheduler` for scheduling.

### Codex
- TUI/core decoupled via JSON-RPC/WebSocket app-server protocol.
- Lossless vs best-effort event tiering with backpressure markers.
- `FrameRequester` coalesces redraws and clamps FPS.
- `execpolicy` DSL + Guardian LLM review for permissions.
- SQLite-backed sessions, but with JSONL mirror/backfill complexity.

**Takeaways for Runie:**
- Do not build a full app-server protocol now, but adopt Codex's lossless/best-effort event tiering for the bus.
- Do not migrate to SQLite now, but consider a `SessionStore` trait so the backend can be swapped later.
- Borrow Kimi's goal/permission UI concepts once core actor state is clean.
- Borrow Goose's per-call permission cache and keyring usage.

---

## 5. Pareto action plan

1. **Stop direct `AppState` mutation** — unblocks most live TUI bugs and enables trustworthy tests.
2. **Subscribe TUI before leader start** (or add replay buffer) — fixes empty config/history at launch.
3. **Move network and blocking IO out of actor handlers** — fixes stuck mailboxes and frozen UI.
4. **Harden shutdown and event-bus reliability** — no more panics or silent event loss.
5. **Delete/consolidate before adding** — remove unused deps, dead MCP scaffolding, dead clipboard module, duplicated turn state, and hand-written generated code.
6. **Adopt crates for clear wins** — `figment`, `toml_edit`, `arboard`, `git2`, `shell-words`, `strum`.
7. **Defer heavy migrations** — SQLite session store, `jsonrpsee`, full app-server protocol.

---

## 6. File inventory

- This plan: `docs/superpowers/plans/2026-06-30-five-round-architecture-review.md`
- Detailed per-round raw notes: agent outputs above (not committed as separate files).
- New tasks: `tasks/*.md` created in this pass (see section 7).
- Roadmap: `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md`

---

## 7. New tasks created in this pass

1. `use-toml-edit-and-file-locks-for-config.md`
2. `replace-layered-config-merge-with-figment.md`
3. `deduplicate-turn-state-between-turnactor-and-appstate.md`
4. `replace-token-wrapper-with-secrecy-secretstring.md`
5. `replace-clipboard-dead-code-with-arboard-or-osc52.md`
6. `replace-token-chars4-heuristic-with-tiktoken-rs.md`
7. `replace-keymap-modifier-tables-with-crokey-map.md`
8. `parse-bash-commands-with-shell-words-in-ioactor.md`
9. `replace-custom-git-detection-with-git2.md`
10. `use-configscope-enum-for-mcp-server-scope.md`
11. `remove-ractor-send-message-alias.md`
12. `replace-custom-nanoid-in-runie-testing.md`
13. `move-grep-find-tests-to-tests-directory.md`
