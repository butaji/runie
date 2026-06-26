# Remaining Tasks Roadmap

**Generated:** 2026-06-23
**Total remaining tasks:** 18
**Completed tasks removed:** 3 root files + full `tasks/archive/` directory

## Notes on dependency cleanup

The following dependencies point to completed/removed tasks and are treated as satisfied:
- `delete-config-reload-shim`
- `delete-dead-tool-runtime-trait`
- `extract-headless-cli-helper`
- `gate-or-implement-mcp-client`
- `r5-add-lifecycle-events`
- `simplify-event-vocabulary`

## Execution strategy

1. **Declarative DSL + actor foundation** — `actor-owned-state-ssot`, `declarative-actor-dsl`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `actor-lifecycle-and-handle-registry`, `test-actor-harness`.
2. **Config SSOT** — finish `config-ssot-via-configactor` and related UI consolidation.
3. **Core domain actors** — `SessionActor`, `TurnActor`, `InputActor`, `ViewActor`, `CompletionActor`.
4. **Cross-cutting actors** — `PermissionActor`, `NotificationActor`, `TrustActor`, `EnvActor`, `FffIndexerActor`, `UiControlActor`.
5. **DSL cleanup** — `unified-dsl-intents-for-state-mutations` rewrites all commands/dialogs using the declarative DSL.
6. **Final sweep** — `remove-direct-appstate-mutations` and R5 features after architecture is stable.

## Phases (dependency order)

### Phase 1 — 94 tasks

**Architecture / Actors**
- `actor-owned-state-ssot` — R4/P0 — 
- `arch-review-2026-06` — R4/P2 — Captures the findings of a top-down architecture review performed on 2026-06-21. Runie is structurally sound (typed event bus, immutable `Snapshot`, pure render function, actor-own
- `audit-simplify-reduce-roadmap` — R4/P0 — Master index of the findings from the YAGNI/stdlib/OS-features architecture and code review. Each finding maps to either an existing task (status noted) or a new task created by th
- `collapse-effect-payload-indirection` — R4/P2 — Side effects (clipboard, external editor, share, login validation, suspend) are computed in two steps: 1. `crates/runie-core/src/effect_payload.rs` (98 LOC) — pure `fn extract(even
- `collapse-provider-wrappers` — R4/P0 — Provider construction has multiple wrapper layers: `Provider` trait → `ProviderFactory` trait → `DynProvider` concrete wrapper → `DynProviderFactory` → `BuiltProvider` → retry wrap
- `consolidate-tiny-binary-crates` — R4/P2 — Three sub-300-line binary crates exist as separate workspace members: | Crate | LOC | Mode | |-------|-----|------| | `runie-print` | 97 | Non-interactive print mode | | `runie-jso
- `dedupe-yolo-sink-setup` — R4/P2 — `crates/runie-json/src/main.rs` and `crates/runie-server/src/main.rs` both parse `--yolo`, print the same warning, and construct `Arc::new(AutoAllowSink)` or `Arc::new(DenyAllSink)
- `delete-dead-tuple-actor-handles-fields` — R4/P1 — `crates/runie-tui/src/main.rs:95-101` declares: ```rust struct ActorHandles( runie_core::actor::ActorHandle, runie_core::actor::ActorHandle, runie_core::actor::ActorHandle, runie_c
- `drop-event-aliases` — R4/P2 — `crates/runie-core/src/event/aliases.rs` keeps four type aliases that all resolve to a canonical sub-enum: ```rust pub use super::control::ControlEvent as CommandEvent; pub use sup
- `drop-event-bus-replay-buffer` — R4/P2 — `crates/runie-core/src/bus.rs` wraps `tokio::sync::broadcast` AND adds its own `ReplayBuffer` (`parking_lot::Mutex<VecDeque<E>>`) so late subscribers can catch up on the last N eve
- `fix-headless-runtime-spinwait` — R3/P2 — `crates/runie-core/src/headless_runtime.rs:46-57` wraps a busy loop of `try_recv` + `tokio::task::yield_now()` in a `timeout`. It discards the timeout result and wastes CPU while w
- `fold-protocol-into-core` — R4/P2 — `runie-protocol` (557 LOC, 9 modules) is a separate workspace crate with exactly 2 consumers: - `crates/runie-server/src/main.rs` — the RPC server. - `crates/runie-core/src/ipc.rs`
- `fold-runie-engine-into-agent` — R4/P2 — `runie-engine` (2234 LOC, 17 files) has exactly one consumer: `runie-agent`. The crate boundary adds a `Cargo.toml`, workspace member, and re-export layer for no independence — too
- `generic-actor-reply` — R4/P1 — `runie-core/src/actors/config/messages.rs` defines `ConfigReply<T>` and `runie-core/src/actors/provider/messages.rs` defines `ProviderReply<T>`. Both are `Arc<Mutex<Option<oneshot:
- `move-actor-trait-into-actors-dir` — R4/P1 — The `Actor` trait lives at `crates/runie-core/src/actor.rs` (222 LOC) while every concrete actor lives under `crates/runie-core/src/actors/` (`actors/mod.rs` is 24 LOC and just dec
- `unify-event-vocabulary` — R4/P0 — Provider streams emit `LLMEvent`s, which are translated to `AgentEvent`s, some persisted as `DurableCoreEvent`s, and some become `proto::EventMsg`s. `runie-tui` then defines `Effec
- `unify-persistence-actors` — R4/P0 — Three actors today share ownership of durable state and all touch `SessionStore` / `SessionIndex`: | Actor | LOC | Owns | |-------|-----|------| | `actors/persistence/actor.rs` | 1
- `unify-provider-modules` — R4/P1 — Four provider-related modules live at the `runie-core` src root with overlapping names: `provider.rs` (the `Provider` trait + `Message` enum), `provider_registry.rs` + `provider_re

**Architecture / Refactoring**
- `split-files-at-limit-round-2` — R4/P2 — Three files sit exactly at the 500-line limit and one at 467 — any edit breaks the build per `crates/runie-core/build.rs` linter (no allow-lists, current violations: 0). `split-lar
- `tighten-build-lint-complexity-heuristic` — R4/P3 — `crates/runie-core/build_lint.rs::count_complexity` is the heuristic that enforces the 10-complexity ceiling. It counts `if`, `else if`, `match`, `while`, `for`, `&&`, `||`, `?`. I

**Architecture / Security**
- `unify-permission-gate` — R4/P0 — `PermissionGate` is implemented twice: once in `runie-core/src/permissions/gate.rs` and again in `runie-agent/src/permission_gate.rs`. The two files are byte-for-byte identical exc

**Architecture / Testing**
- `centralize-test-verification` — R4/P0 — Test-count verification logic is duplicated between `.github/workflows/ci.yml` and `scripts/verify-tests.sh`. The two copies are already out of sync: CI expects 1806 total tests wh
- `dedupe-fresh-state-test-helper` — R4/P2 — Tests are ~63% of `runie-core` LOC (14,539 in `tests/` + ~18,853 inline `#[cfg(test)]`). A handful of byte-identical helpers repeat across that mass: `fresh_state()` (20 sites — li
- `dedupe-messages-changed-ensure-fresh` — R4/P3 — The two-call sequence `state.messages_changed(); state.ensure_fresh();` appears 174 times across `runie-core/src/tests/*` and `runie-tui/src/tests/*`. Test helpers that push messag
- `fix-permission-gate-rename-in-testing` — R4/P1 — `crates/runie-testing/src/fixtures.rs:6` imports a type that no longer exists: ```rust use runie_core::permissions::{AutoAllowSink, PermissionManager, PermissionGate}; ``` `Permiss
- `rename-test-submission-id` — R4/P2 — `runie-core/src/proto/op.rs` defines `struct SubmissionId(pub u64)` and `runie-testing/src/runner.rs` defines `pub type SubmissionId = String`. The same name with different semanti
- `share-minimax-sse-fixtures` — R4/P1 — Seven MiniMax SSE fixture files are byte-identical in `crates/runie-agent/tests/fixtures/minimax/*` and `crates/runie-provider/tests/fixtures/minimax/*`. Updates must be copied man

**Configuration**
- `collapse-headless-binaries-into-one-cli` — R4/P1 — Three near-identical binary crates exist for non-interactive execution: | Crate | Binary | LOC | What it does | |-------|--------|-----|--------------| | `runie-print` | `runie-pri
- `consolidate-dual-path-modules` — R4/P1 — Seven modules in `crates/runie-core/src/` use the dual-path layout (`foo.rs` declaring `mod foo/` submodules) instead of the conventional `foo/mod.rs` style. The dual-path form is 
- `delete-dead-theme-async-loaders` — R4/P1 — `crates/runie-tui/src/theme/loader.rs` declares three async functions that are never used: - `load_theme_raw_async(name: String) -> opaline::Theme` (line 31) - `load_theme_async(na
- `delete-tui-ipc-reexport-shim` — R4/P0 — `crates/runie-tui/src/ipc.rs` is a 5-line pure re-export: `pub use runie_core::ipc::TuiIpc;`. Declared as `pub mod ipc;` in `runie-tui/src/lib.rs:21` but `rg "use runie_tui::ipc|cr
- `drop-thiserror-and-async-trait-deps` — R4/P2 — Two small deps are removable with trivial or zero code change, extending the `drop-small-stdlib-replaceable-deps` rationale: | Dep | Sites | Replacement | |-----|-------|----------
- `fix-duplicate-cargo-toml-keys` — R4/P1 — Two crate manifests declare the same dependency twice on separate lines. Cargo unions the features silently and uses the last declaration's metadata, so it builds — but the duplica
- `fold-testing-into-agent-tests` — R4/P3 — `runie-testing` (344 LOC, 5 files: `events.rs`, `fixtures.rs`, `lib.rs`, `macros.rs`, `mock_tool_runtime.rs`, `runner.rs`) is a workspace crate with exactly 1 external consumer: `c
- `generate-config-schema-from-code` — R4/P2 — `config.schema.json` is checked in and mirrors defaults declared in `crates/runie-core/src/config.rs` (`truncation.max_lines=2000`, `max_bytes=51200`, `telemetry.enabled=true`, `ui
- `hoist-cargo-workspace-deps` — R4/P1 — Several dependencies are declared inline in multiple crate manifests instead of using workspace inheritance: `serde = { version = "1.0", features = ["derive"] }` in 5 crates, `asyn
- `inline-or-document-core-ui-shim` — R4/P2 — `crates/runie-tui/src/core_ui/mod.rs` is an 8-line re-export shim: `pub use runie_core::ui::{Element, Feed, LazyCache, Post, PostBuilder, PostKind};`. The doc comment claims moving
- `inline-tui-ipc-reexport` — R4/P2 — `crates/runie-tui/src/ipc.rs` is a 5-line re-export: `pub use runie_core::ipc::TuiIpc;`. The doc comment justifies keeping both endpoints in `runie-core` (to avoid a circular dep),
- `reconsider-clipboard-image-deps` — R4/P2 — `arboard` + `png` were adopted in the done task `adopt-arboard-clipboard` to support pasting images from the clipboard into chat. Reversal argument under the YAGNI posture: - Two c
- `reconsider-notify-config-watcher` — R4/P2 — `notify` + `notify-debouncer-mini` were adopted in the done task `adopt-notify-config-watcher` to replace a 2s polling loop with OS file-watch events for `ConfigActor` hot-reload. 
- `reconsider-schemars-jsonschema` — R4/P2 — `schemars` + `jsonschema` were adopted in the done task `adopt-layered-config` to generate `config.schema.json` and validate config at load time (`Config::validate` / `load_strict`
- `reconsider-tiktoken-rs` — R4/P2 — `tiktoken-rs` was adopted in the done task `adopt-tiktoken-rs` for accurate BPE token counts. Reversal argument under the YAGNI / stdlib posture: - Only one consumer: `crates/runie
- `rename-update-dialog-panel` — R4/P3 — Two files named `panel.rs` exist in the dialog subsystem: `dialog/panel.rs` (500 LOC — the `Panel` struct, its builder methods, `PanelView`, `PanelItem`) and `update/dialog/panel.r
- `replace-single-use-deps-with-stdlib` — R4/P2 — Several workspace deps are used in only 1-3 files for functionality the stdlib or a few lines of code can provide. `remove-unused-cargo-deps` targets truly unused deps; this task t
- `shrink-tasks-planning-debt` — R4/P2 — `tasks/` has 76 `.md` files at the root (excluding TEMPLATE), 216 done, 48 todo, 3 superseded. The task system has become its own maintenance surface — finding active work requires
- `single-source-dev-commands` — R4/P3 — `cargo test --workspace`, `cargo clippy --workspace`, and `cargo fmt` appear in `README.md`, `.github/workflows/ci.yml`, `dev.sh`, and `bacon.toml`. These entry points drift out of
- `single-source-testing-docs` — R4/P1 — The 4-layer testing strategy (L1 state, L2 event handling, L3 TestBackend, L4 provider replay) is written out in full in `AGENTS.md` and `docs/Architecture.md`, summarized in `docs
- `use-workspace-deps-in-runie-testing` — R4/P3 — `crates/runie-testing/Cargo.toml` uses `runie-core = { path = "../runie-core" }` for internal crates, while every other crate uses `runie-core.workspace = true`. This inconsistency

**Core / State**
- `aggressive-event-consolidation` — R4/P1 — The `crates/runie-core/src/event/` module is 22 files for a single 109-variant `Event` enum: | File group | Count | Content | |-------------|-------|---------| | `variants.rs` + `v
- `canonicalize-chat-message` — R4/P0 — The codebase has multiple overlapping message types: `runie_core::provider::Message`, `runie_core::message::ChatMessage`, `runie_core::proto::messages::Message`, and `runie_protoco
- `canonicalize-tool-call` — R4/P0 — The same tool-invocation concept is represented by at least six types: `message::ToolCall`, `tool_runtime::ToolCall`, `tool_parser::ParsedToolCall`, `message::parts::Part::ToolCall
- `centralize-error-type` — R4/P2 — The codebase defines many overlapping error types: `ProviderError`, `LLMError`, `ToolError`, `ToolParseError`, `SubagentError`, `proto::Error`, and pervasive `anyhow::Error`. Sever
- `collapse-event-aliases` — R4/P2 — `crates/runie-core/src/event/` contains 12 single-purpose files: 11 that each declare one `pub type Foo = Event;` alias, plus `dialog.rs` which is a 1-LOC doc-only stub that contri
- `collapse-event-variants-subdir` — R4/P2 — `crates/runie-core/src/event/variants.rs` (413 LOC, the canonical `Event` enum) declares three sub-files via `mod constructors; mod name; mod to_durable;`. Each sub-file is just an
- `consolidate-login-flow-handlers` — R4/P1 — The login flow is split across two roots: `login_flow/` (state machine, panels, validation — the "what") and `update/login_flow.rs` (event handlers — the "how"). The handlers file 
- `consolidate-mark-dirty-calls` — R4/P2 — `state.mark_dirty()` is called after nearly every handler arm — 19 times in `update/input/text.rs`, 16 in `update/input/nav.rs`, 13 in `update/login_flow.rs`, 8 each in `update/dia
- `dedupe-scoped-model-toggle` — R4/P2 — `crates/runie-core/src/update/dialog/toggle.rs` defines `handle_scoped_model_enable_all` and `handle_scoped_model_disable_all`. The bodies are identical except for `model.enabled =
- `dedupe-turn-complete-last-calls` — R4/P2 — `crates/runie-core/src/update/agent/mod.rs` calls `state.ensure_turn_complete_last();` in 6 of 9 match arms when handling `AgentEvent`. It is easy to forget the call when adding a 
- `delete-dead-history-action-vimnav` — R4/P1 — `crates/runie-core/src/update/input/mod.rs` declares: ```rust enum HistoryAction { VimNav(bool), // line 116 — never constructed // ... } fn navigate_history(state: &mut AppState, 
- `delete-provider-message-enum` — R4/P2 — `crates/runie-core/src/provider.rs:11-50` defines `pub enum Message { System { content }, User { content }, Assistant { content, tool_calls }, ToolResult { tool_call_id, content } 
- `delete-runie-core-confirmation` — R4/P1 — `crates/runie-core/src/confirmation.rs` (217 LOC) defines `pub enum ConfirmationKind { None, Diff { preview }, Write { path, content, byte_count }, Bash { command, reason } }` (wit
- `delete-runie-core-skill-module` — R4/P1 — `crates/runie-core/src/skill/` (318 LOC, **singular** — distinct from the live `crates/runie-core/src/skills/`, plural) defines `pub struct SkillSummary`, `pub struct SkillRegistry
- `delete-runie-core-utils` — R4/P2 — `crates/runie-core/src/utils.rs` (53 LOC) exports `pub fn truncate(s: &str, n: usize) -> String` and `pub fn join_optional(...)`. `rg 'crate::utils|runie_core::utils|use crate::uti
- `extract-appstate-helpers` — R4/P3 — Two boilerplate patterns repeat in `update/login_flow.rs` and other handlers: (1) `state.login_flow.as_ref().map(|f| f.provider.clone()).unwrap_or_default()` appears 2-3x to resolv
- `fold-state-into-model-state` — R4/P2 — Two "state" modules coexist in `runie-core`: `state/` (5 files: `agent.rs`, `input.rs`, `session.rs`, `view.rs`, `mod.rs` — contains `AgentState`, `CommandUsage`, `InputState`, `Se
- `merge-tool-status-state` — R4/P2 — `runie-core/src/tool/state.rs` defines `ToolCallState { Pending, Running, Completed, Error }` and `runie-core/src/tool/context.rs` defines `ToolStatus { Success, Error, TimedOut, B
- `r5-conversation-sanitize-pipeline` — R5/P1 — After a streaming turn completes, the message history can contain edge cases that break subsequent provider calls: dangling `Part::ToolCall` blocks without matching `Part::ToolResu
- `r5-populate-parts-streaming` — R5/P1 — `ChatMessage` (`crates/runie-core/src/message.rs`) has both `content: String` (monolithic) and `parts: Vec<Part>` (structured). The streaming path in `crates/runie-core/src/update/
- `r5-think-filter` — R5/P1 — Many providers (DeepSeek, vLLM, OpenRouter, local Ollama models) stream reasoning content as plain text wrapped in `<tool_call>`/`</thinking>` or `<thinking>`/`</thinking>` tags in
- `remove-appstate-cfg-test-branches` — R4/P1 — `AppState` (in `model/state/app_state.rs`) carries 5 `Option<Sender>` actor handles (`config_tx`, `provider_tx`, `persistence_tx`, `session_store_tx`, `io_tx`) for test-vs-prod mod
- `remove-empty-match-arms` — R4/P3 — `_ => {}` empty catch-all match arms appear 7x in `update/dialog/panel.rs`, 3x in `markdown/blocks.rs`, 2x each in `update/system.rs` and `update/login_flow.rs`, and 1x in several 
- `remove-sleeps-from-automatic-tests` — R3/P1 — `AGENTS.md` forbids `sleep()` in automatic tests. Most sleeps have already been removed; only two sources remain: - `crates/runie-core/src/actors/fff_indexer/tests.rs` — three `tok
- `rename-model-snapshot-to-compaction` — R4/P2 — `crates/runie-core/src/model/snapshot.rs` (76 LOC) is not a snapshot — it implements `AppState::total_tokens()` and session-compaction helpers. The real UI `Snapshot` lives in `cra
- `sweep-allow-suppressions` — R4/P2 — 18 `#[allow(...)]` markers ship in production code. Each is either (a) dead code kept "for later" that ships in prod binaries — a YAGNI violation, or (b) a clippy lint suppression 
- `sweep-to-string-allocs` — R4/P3 — `.to_string()` appears 680x in the workspace vs `.to_owned()` 2x. Two waste patterns hide in that count: (1) `some_string.to_string()` on an already-owned `String` — a redundant cl
- `unify-build-system-prompt` — R4/P2 — `crates/runie-core/src/prompts.rs` defines `build_system_prompt` and `crates/runie-core/src/turn_setup.rs` also defines `build_system_prompt` (plus `build_system_prompt_with_tools`

**Input / Commands**
- `consolidate-slash-command-handlers` — R4/P2 — Slash commands are dispatched in two places that should be one: - `crates/runie-core/src/commands/registry.rs` builds a typed `CommandRegistry` of named commands and their argument
- `dedupe-update-input-history` — R4/P2 — `crates/runie-core/src/update/input/mod.rs` defines `handle_history_prev` and `handle_history_next` with mirrored structure: both check `vim_nav_mode`, then path-completion mode, t

**Sessions**
- `fix-session-store-load-alignment` — R3/P1 — `crates/runie-core/src/session_store.rs:180-193` reads session events from redb into two separate vectors (`keys` and `events`). When a row fails to deserialize, the event is silen
- `reconsider-redb-session-store` — R4/P2 — `redb` was adopted in the done task `adopt-redb-session-store` to replace JSONL + atomic rename for session persistence. Reversal argument under the YAGNI / stdlib posture: - The d

**TUI / Rendering**
- `fix-main-unix-epoch-unwrap` — R3/P2 — `crates/runie-tui/src/main.rs:45` uses `duration_since(UNIX_EPOCH).unwrap()` to generate a session ID. If the system clock is ever set before the Unix epoch, this panics. The fix i
- `fix-render-task-blocking-io` — R3/P1 — `render_task` in `crates/runie-tui/src/main.rs` calls `terminal.size()`, `terminal.clear()`, and `terminal.draw()` from within a Tokio task. These are synchronous terminal writes t
- `fix-runie-tui-dead-code-warnings` — R4/P0 — `cargo check --workspace` currently emits 4 `dead_code` warnings, all in `runie-tui`: ``` warning: function `load_theme_raw_async` is never used warning: function `load_theme_async
- `merge-diff-modules` — R4/P2 — Two `diff.rs` files exist: | File | LOC | Role | |------|-----|------| | `crates/runie-core/src/diff.rs` | — | Domain diff logic (line-level patch application / parsing) | | `crate
- `r5-markdown-healing-stream` — R5/P1 — `StreamingBuffer` (`crates/runie-core/src/streaming_buffer.rs:1-196`) splits streaming content into stable lines and a tail, and detects open code fences and tables. But it does no
- `r5-paced-text-rendering` — R5/P2 — Text currently appears in the TUI as it arrives from the 50ms debounced `StreamingBuffer::flush()`. When the provider sends a burst of deltas (e.g., 500 characters in one chunk), t
- `theme-color-accessor-macro` — R4/P3 — `crates/runie-tui/src/theme/colors.rs` contains ~15 functions of the form `pub fn color_X() -> Color { Color::from(crate::theme::current_theme().color("...")) }`. Adding a new sema
- `unify-markdown-rendering` — R4/P3 — Markdown handling is split across two crates with overlapping concerns: - `crates/runie-core/src/markdown/` — markdown parsing using `pulldown-cmark`. - `crates/runie-tui/src/markd

**Tools**
- `add-engine-tool-tests` — R4/P1 — `crates/runie-engine/src/lib.rs` is a single module file that re-exports the built-in tools (`read_file`, `write_file`, `edit_file`, `bash`, `grep`, `find`, `fetch_docs`, `list_dir
- `add-tool-output-constructors` — R4/P1 — Every tool implementation manually constructs `ToolOutput { tool_name, tool_args, content, bytes_transferred, duration, status }` in success paths, error paths, and lock-error path
- `collapse-tool-runtime-traits` — R4/P1 — After `delete-dead-tool-runtime-trait` removes the unused rich `tool::ToolRuntime`, two layers remain: - `tool::Tool` (concrete, `tool/registry.rs:13`) — implemented by every engin
- `introduce-tool-definition-macro` — R4/P1 — Every tool in `crates/runie-engine/src/tool/*.rs` hand-writes an `input_schema(&self) -> Value` implementation returning `json!({ "type": "object", "properties": { ... }, "required
- `r5-extract-tool-stream` — R5/P0 — Streaming tool-call accumulation is currently duplicated in two places: `StreamState` in `crates/runie-provider/src/openai/stream.rs:46-50` (a `BTreeMap<usize, Accumulator>` keyed 
- `share-fff-error-helpers` — R4/P1 — `crates/runie-engine/src/tool/search/core.rs` and `find_definitions.rs` duplicate `build_not_indexed_output`, `build_picker_not_initialized_output`, `with_picker`, and lock-guard e

### Phase 2 — 24 tasks

**Architecture / Actors**
- `actor-lifecycle-and-handle-registry` — R4/P0 — The new actors need a clear lifecycle: where they are spawned, how their handles reach `AppState`, and how the TUI/bootstrap wires them together. Current reality (`runie-tui/src/ma (depends on: `actor-owned-state-ssot`)
- `decouple-appstate-from-view-cache` — R4/P0 — `crates/runie-core/src/ui/` (`elements.rs`, `transform.rs`, `posts.rs`, `dsl_test.rs`) is the view-model: an `Element`/`Feed`/`Post` tree projected from `AppState` and cached via ` (depends on: `fold-state-into-model-state`)
- `move-provider-catalog-to-provider-crate` — R4/P0 — The domain crate `runie-core` carries ~840 LOC of provider/model knowledge that belongs in `runie-provider`: | File | LOC | Content | |------|-----|---------| | `provider_registry. (depends on: `unify-provider-modules`)
- `r5-per-channel-decoders` — R5/P2 — Currently all `Event`s flow through a single flat `EventBus` to the `UiActor`, which applies every event to `AppState` and publishes a full `Snapshot`. As the UI grows more complex (depends on: `r5-populate-parts-streaming`)
- `r5-provider-protocol-trait` — R5/P0 — The OpenAI SSE parser in `crates/runie-provider/src/openai/stream.rs` is a monolithic async generator (`openai_event_stream` at line 59) that hard-codes HTTP transport, SSE framing (depends on: `r5-extract-tool-stream`)
- `reduce-actor-handle-boilerplate` — R4/P1 — Every actor repeats: `pub enum XxxMsg { ... }`, `pub struct XxxActorHandle { tx: mpsc::Sender<XxxMsg> }`, `new`, a `tx` accessor, and a set of async request methods. The core actor (depends on: `generic-actor-reply`)
- `rename-core-ui-to-view` — R4/P1 — `crates/runie-core/src/ui/` (1041 LOC across `elements.rs`, `transform.rs`, `posts.rs`, `mod.rs`) is the **view-model**: pure domain projection (`AppState` → `Element`/`Feed`/`Post (depends on: `fold-state-into-model-state`)
- `simplify-actor-trait` — R4/P2 — The `Actor` trait in `crates/runie-core/src/actor.rs` has two friction points: 1. **Unused default `run_body`** (actor.rs:59-70). The default impl is `async move { let _ = (self, r (depends on: `unify-persistence-actors`)

**Architecture / Refactoring**
- `consolidate-update-dialog-files` — R4/P2 — `update/dialog/` has 11 handler files totaling 2,082 LOC — 29% of the entire `update/` directory. Several are thin (<150 LOC) toggle/picker helpers that fragment one coherent conce (depends on: `rename-update-dialog-panel`)

**Architecture / Security**
- `derive-confirmation-from-permissions` — R4/P2 — `runie-core/src/confirmation.rs` defines `ConfirmationKind` and `ConfirmationRouter`, classifying edit/write/bash/read-only tools. `runie-core/src/permissions/mod.rs` classifies th (depends on: `unify-permission-gate`)
- `unify-approval-decision` — R4/P1 — `runie-core/src/proto/op.rs` defines `ApprovalDecision { Approve, Reject }` and `runie-core/src/permissions/mod.rs` defines `PermissionAction { Allow, Ask, Deny }`. Both describe a (depends on: `unify-permission-gate`)

**Architecture / Testing**
- `consolidate-login-logout-tests` — R4/P3 — `tests/login_logout/` has 14 files totaling 1,798 LOC — 12% of all core test code. The four smallest (`multiple` 32, `edge_cases` 34, `model_select` 47, `core` 49 = 162 LOC) are th (depends on: `consolidate-login-flow-handlers`)

**Configuration**
- `drop-small-stdlib-replaceable-deps` — R4/P2 — Four small deps each have 1–3 use sites and a trivial stdlib replacement. Bundled into one task because each is small and they share the YAGNI / stdlib rationale: | Dep | Sites | s (depends on: `drop-event-bus-replay-buffer`)
- `extract-ci-setup-action` — R4/P1 — `.github/workflows/ci.yml` copy-pastes `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, and cargo cache steps across the `test`, `fmt`, `clippy`, and `e2e` jobs. Any toolcha (depends on: `centralize-test-verification`)
- `flatten-update-login-flow-tests-dir` — R4/P2 — `crates/runie-core/src/update/login_flow/` is a directory containing a single file `tests.rs` (45 LOC) — the tests for the sibling `update/login_flow.rs`. This is the dual-path pat (depends on: `consolidate-dual-path-modules`)
- `relocate-loose-tests-files` — R4/P2 — Eight `*_tests.rs` files sit beside the module they test, using the `_tests` suffix with either `#[path = "..."]` includes or `mod X_tests;` declarations. This breaks the two conve (depends on: `consolidate-dual-path-modules`)
- `relocate-stray-tool-parser-tests` — R4/P2 — `crates/runie-core/src/tool_parser_tests.rs` (106 LOC) is a stray top-level test file. The module it tests (`tool_parser`) lives at `tool_parser/mod.rs` after `consolidate-dual-pat (depends on: `consolidate-dual-path-modules`)

**Core / State**
- `audit-borrow-workarounds` — R4/P3 — `Option::take()`, `std::mem::take`, `std::mem::replace`, and `std::mem::swap` appear 19x across `update/` and `markdown/`. Many are legitimate (`Option::take` is idiomatic for "mov (depends on: `consolidate-login-flow-handlers`)
- `consolidate-login-flow-all-files` — R4/P1 — The login-flow feature is spread across 11 files in 5 different module roots: | File | Root | Role | |------|------|------| | `login_flow/mod.rs` | `login_flow/` | State machine mo (depends on: `consolidate-login-flow-handlers`)
- `event-taxonomy-for-actor-state-sync` — R4/P0 — The `Event` enum is a flat 109-variant bag. It mixes user intents, actor facts, UI control, and IO results. Define a clear taxonomy: **Intents** (requests to actors) and **Facts**  (depends on: `actor-owned-state-ssot`)
- `extract-streaming-tool-parser` — R4/P1 — `crates/runie-agent/src/stream_response.rs` and `crates/runie-agent/src/headless.rs` both define `ToolCallAccumulator { name, arguments }` and nearly identical state machines for h (depends on: `canonicalize-tool-call`)
- `r5-parts-canonical-migration` — R5/P2 — `ChatMessage` (`crates/runie-core/src/message.rs`) currently has both `content: String` and `parts: Vec<Part>`. The streaming path (after `r5-populate-parts-streaming`) keeps them  (depends on: `r5-populate-parts-streaming`)

**Sessions**
- `group-session-modules-into-dir` — R4/P1 — Six `session_*.rs` files live at the `crates/runie-core/src/` root (1,863 LOC), inflating the root file count to 60 and forcing readers to grep six separate prefixes. Grouping them (depends on: `consolidate-dual-path-modules`)

**Tools**
- `r5-partial-json-repair` — R5/P1 — When tool calls are parsed from plain text fallback (`parse_tool_calls_fallible` in `crates/runie-core/src/tool_parser.rs:36`), incomplete JSON arguments are silently dropped — the (depends on: `r5-extract-tool-stream`)

### Phase 3 — 5 tasks

**Architecture / Actors**
- `declarative-actor-dsl` — R4/P0 —  (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`)

**Configuration**
- `standardize-test-layout-inline` — R4/P2 — Three test layouts coexist in the workspace: | Layout | Count | Example | |--------|-------|---------| | Inline `#[cfg(test)] mod tests` | 125 | Idiomatic Rust | | Sibling `*_tests (depends on: `relocate-loose-tests-files`)

**Core / State**
- `app-state-read-only-projection` — R4/P0 — `AppState` is currently a bag of public mutable fields that anyone can write. Every field in `AppState` and every field in the inner state structs (`SessionState`, `InputState`, `V (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`)
- `dedupe-tool-execution-loop` — R4/P1 — `crates/runie-agent/src/turn.rs` (`execute_tools`) and `crates/runie-agent/src/headless.rs` (`execute_headless_tools`) both iterate over parsed tool calls, build a `ToolContext`, g (depends on: `extract-streaming-tool-parser`)

**TUI / Rendering**
- `generate-ui-elements-from-model` — R4/P2 — `runie-core/src/ui/elements.rs` defines `Element` variants (`UserMessage`, `AgentMessage`, `Thinking`, `ToolRunning`, `ToolDone`, `TurnComplete`) that are a UI projection of `ChatM (depends on: `rename-core-ui-to-view`)

### Phase 4 — 12 tasks

**Architecture / Actors**
- `dedupe-config-actor-mutations` — R4/P2 — `crates/runie-core/src/actors/config/actor.rs` has four methods (`save_provider`, `remove_provider`, `set_default_model`, `set_provider_models`) that each clone strings/path, call  (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)
- `ui-control-actor-owns-dialog-state` — R4/P0 — Several UI-control fields are mutated by many handlers but were not assigned an actor in the first pass: - `open_dialog` — which overlay is currently open. - `dialog_back_stack` —  (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)

**Architecture / Security**
- `permission-actor-owns-approvals` — R4/P0 — Permission approvals use a shared `Arc<Mutex<ApprovalRegistry>>` and a `permission_request` UI field that various handlers mutate directly. There is no `PermissionActor`. Create on (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)
- `trust-actor-owns-trust-decisions` — R4/P1 — Trust decisions and the derived `config.read_only` flag are mutated by system helpers and startup code. `PersistenceActor` already persists trust and emits `TrustLoaded`/`TrustChan (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)

**Architecture / Testing**
- `test-actor-harness` — R4/P0 — Once state is owned by actors, unit tests can no longer mutate `AppState` fields directly. Provide a `TestHarness` that can spawn a lightweight actor for a single domain, feed it i (depends on: `actor-lifecycle-and-handle-registry`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)

**Configuration**
- `config-ssot-via-configactor` — R4/P0 — `ConfigActor` is already intended to be the single writer of `~/.runie/config.toml`, but most of the app bypasses it. This task makes `ConfigActor` the true SSOT for all config-dri (depends on: `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)
- `env-actor-owns-git-cwd` — R4/P1 — `git_info` and `cwd_name` are set once during TUI bootstrap via blocking IO (`runie-tui/src/app_init.rs`). They never change after startup but they are still direct state mutations (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `actor-lifecycle-and-handle-registry`)

**Core / State**
- `notification-actor-owns-transient-messages` — R4/P1 — Transient notification state (`transient_message`, `transient_until`, `transient_level`) is mutated in at least six files, and expiration is currently triggered from the render-pat (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)
- `session-actor-owns-session-state` — R4/P0 — All `AppState.session` mutations are scattered across input handlers, agent handlers, system helpers, command handlers, and replay helpers. The existing `SessionActor` (`crates/run (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)

**Input / Commands**
- `input-actor-owns-input-state` — R4/P0 — `InputState` (input buffer, cursor, history, undo/redo, file-picker backups, prompt) is mutated synchronously by text handlers, navigation handlers, system helpers, dialog handlers (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`)
- `unified-dsl-intents-for-state-mutations` — R4/P0 — Slash commands, palette commands, dialog actions, and keybindings currently mix direct state mutation with event emission. Standardize every user-facing action on a small set of ty (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `declarative-actor-dsl`)

**Tools**
- `fff-indexer-owns-file-picker-results` — R4/P1 — `fff_file_results` and `fff_debounce` are mutated synchronously by dialog openers and file-picker input handlers. `FffIndexerActor` already exists and emits `Event::FffSearchResult (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `actor-lifecycle-and-handle-registry`)

### Phase 5 — 5 tasks

**Architecture / Actors**
- `remove-login-config-test-shim` — R4/P0 — `AppState` has 5 `#[cfg(test)]` branches that fall back to `login_config::` for config reads/writes when `config_tx` / `config_cache` are `None`: - `app_state.rs:267` `configured_p (depends on: `actor-owned-state-ssot`, `config-ssot-via-configactor`)
- `turn-actor-owns-agent-turn-state` — R4/P0 — `AgentState` turn flags, scheduling queues, and token accounting are mutated all over the UI layer and from the agent crate's handle. The `AgentActor` in `runie-agent` runs the pro (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `session-actor-owns-session-state`, `input-actor-owns-input-state`)

**Configuration**
- `consolidate-settings-providers-dialog` — R4/P1 — Settings and providers management UI logic is split across three modules with overlapping concerns: - `settings.rs` (77 LOC) — settings dialog data model. - `providers_dialog.rs` ( (depends on: `unify-provider-modules`, `actor-owned-state-ssot`, `config-ssot-via-configactor`)

**Input / Commands**
- `completion-actor-owns-completion-state` — R4/P1 — Path completion, `@` mention suggestions, ghost text, and tab-completion state are mutated by dialog handlers and legacy at-refs code. There is no `CompletionActor`. Create one and (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `input-actor-owns-input-state`)

**TUI / Rendering**
- `view-actor-owns-view-state` — R4/P0 — `ViewState` and the derived feed cache (`elements_cache`, `posts`, `line_counts`, `total_lines`, etc.) are written from dozens of places. `mark_dirty()` and `messages_changed()` ar (depends on: `actor-owned-state-ssot`, `event-taxonomy-for-actor-state-sync`, `app-state-read-only-projection`, `session-actor-owns-session-state`, `input-actor-owns-input-state`, `ui-control-actor-owns-dialog-state`)

### Phase 6 — 2 tasks

**Architecture / Actors**
- `finish-io-migration` — R4/P0 — `remove-io-from-runie-core` is marked "done" but the `arch_guardrails.rs` test still carries a `legacy_sync_io_files()` allow-list, and real sync IO remains in the domain crate: -  (depends on: `remove-login-config-test-shim`)
- `remove-direct-appstate-mutations` — R4/P0 — After each domain actor is introduced, do a final sweep to remove any remaining direct `AppState` field assignments outside the allowed actor/projection modules. This task is the g (depends on: `session-actor-owns-session-state`, `input-actor-owns-input-state`, `view-actor-owns-view-state`, `completion-actor-owns-completion-state`, `turn-actor-owns-agent-turn-state`, `permission-actor-owns-approvals`, `notification-actor-owns-transient-messages`, `trust-actor-owns-trust-decisions`, `env-actor-owns-git-cwd`, `fff-indexer-owns-file-picker-results`, `ui-control-actor-owns-dialog-state`, `config-ssot-via-configactor`, `actor-lifecycle-and-handle-registry`)

### Phase 7 — 3 tasks

**Architecture / Actors**
- `delete-async-io-bridge` — R4/P1 — `crates/runie-core/src/async_io.rs` provides two "tactical bridge" helpers: - `run_blocking_if_runtime` — fire-and-forget blocking work on a Tokio blocking thread. - `block_in_plac (depends on: `finish-io-migration`)
- `move-tui-effects-into-io-actor` — R4/P0 — `crates/runie-tui/src/effects/` (6 files, 339 LOC) performs direct side effects inside the *rendering* crate, violating the "UI pure / MVU" and "Async IO discipline" rules in `docs (depends on: `finish-io-migration`)
- `replace-git2-with-cli` — R4/P1 — `git2` is a workspace dep with `vendored-libgit2`, which pulls a C toolchain into the build. It is used in 6 files, primarily for git status / branch / worktree detection. The code (depends on: `finish-io-migration`)

### Phase 8 — 2 tasks

**Architecture / Actors**
- `split-runie-core-into-domain-and-io-crates` — R4/P0 — `runie-core` is a 51,524-line / 336-file god-crate with 30+ direct deps that mixes all three layers the `docs/Architecture.md:9-13` claims to separate: - **IO layer in core**: `act (depends on: `finish-io-migration`, `delete-async-io-bridge`, `fold-state-into-model-state`, `rename-core-ui-to-view`)

**Tools**
- `dedupe-git-status-formatter` — R4/P2 — Two copies of the same `git2::Status` → label formatter exist: - `crates/runie-core/src/update/dialog/fff.rs:85` — `format_fff_git_status(status: git2::Status) -> String` with `STA (depends on: `replace-git2-with-cli`)

### Phase 9 — 3 tasks

**Configuration**
- `consolidate-config-modules-into-dir` — R4/P2 — `runie-core` has seven config-related locations scattered across the src root and dirs: | Location | LOC | Role | |----------|-----|------| | `config.rs` | 467 | Main `Config` stru (depends on: `consolidate-login-flow-handlers`, `split-runie-core-into-domain-and-io-crates`)
- `gate-or-move-single-consumer-core-modules` — R4/P2 — `runie-core` carries modules that only one downstream consumer (always the TUI) actually uses. Every binary pays to compile them: | Module | LOC | Only consumer | Why it's in core  (depends on: `split-runie-core-into-domain-and-io-crates`)
- `unify-duplicate-module-names-core-tui` — R4/P2 — Five module names exist in **both** `runie-core` and `runie-tui` with different meanings, creating split-brain ownership: | Module | core | tui | Status | |--------|------|-----|-- (depends on: `split-runie-core-into-domain-and-io-crates`)

## Appendix: task inventory

- `fix-render-task-blocking-io` — R3 P1 — [TUI / Rendering] — `tasks/fix-render-task-blocking-io.md`
- `fix-session-store-load-alignment` — R3 P1 — [Sessions] — `tasks/fix-session-store-load-alignment.md`
- `remove-sleeps-from-automatic-tests` — R3 P1 — [Core / State] — `tasks/remove-sleeps-from-automatic-tests.md`
- `fix-headless-runtime-spinwait` — R3 P2 — [Architecture / Actors] — `tasks/fix-headless-runtime-spinwait.md`
- `fix-main-unix-epoch-unwrap` — R3 P2 — [TUI / Rendering] — `tasks/fix-main-unix-epoch-unwrap.md`
- `actor-lifecycle-and-handle-registry` — R4 P0 — [Architecture / Actors] — `tasks/actor-lifecycle-and-handle-registry.md` | depends: actor-owned-state-ssot | blocks: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state
- `actor-owned-state-ssot` — R4 P0 — [Architecture / Actors] — `tasks/actor-owned-state-ssot.md` | blocks: declarative-actor-dsl, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry, test-actor-harness, config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state, unified-dsl-intents-for-state-mutations, remove-direct-appstate-mutations
- `app-state-read-only-projection` — R4 P0 — [Core / State] — `tasks/app-state-read-only-projection.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync | blocks: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results
- `audit-simplify-reduce-roadmap` — R4 P0 — [Architecture / Actors] — `tasks/audit-simplify-reduce-roadmap.md`
- `canonicalize-chat-message` — R4 P0 — [Core / State] — `tasks/canonicalize-chat-message.md`
- `canonicalize-tool-call` — R4 P0 — [Core / State] — `tasks/canonicalize-tool-call.md` | blocks: extract-streaming-tool-parser
- `centralize-test-verification` — R4 P0 — [Architecture / Testing] — `tasks/centralize-test-verification.md` | blocks: extract-ci-setup-action
- `collapse-provider-wrappers` — R4 P0 — [Architecture / Actors] — `tasks/collapse-provider-wrappers.md`
- `config-ssot-via-configactor` — R4 P0 — [Configuration] — `tasks/config-ssot-via-configactor.md` | depends: event-taxonomy-for-actor-state-sync, app-state-read-only-projection
- `declarative-actor-dsl` — R4 P0 — [Architecture / Actors] — `tasks/declarative-actor-dsl.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync | blocks: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations
- `decouple-appstate-from-view-cache` — R4 P0 — [Architecture / Actors] — `tasks/decouple-appstate-from-view-cache.md` | depends: fold-state-into-model-state | blocks: rename-core-ui-to-view
- `delete-tui-ipc-reexport-shim` — R4 P0 — [Configuration] — `tasks/delete-tui-ipc-reexport-shim.md`
- `event-taxonomy-for-actor-state-sync` — R4 P0 — [Core / State] — `tasks/event-taxonomy-for-actor-state-sync.md` | depends: actor-owned-state-ssot | blocks: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations, app-state-read-only-projection
- `finish-io-migration` — R4 P0 — [Architecture / Actors] — `tasks/finish-io-migration.md` | depends: remove-login-config-test-shim | blocks: delete-async-io-bridge
- `fix-runie-tui-dead-code-warnings` — R4 P0 — [TUI / Rendering] — `tasks/fix-runie-tui-dead-code-warnings.md`
- `input-actor-owns-input-state` — R4 P0 — [Input / Commands] — `tasks/input-actor-owns-input-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection | blocks: turn-actor-owns-agent-turn-state, completion-actor-owns-completion-state
- `move-provider-catalog-to-provider-crate` — R4 P0 — [Architecture / Actors] — `tasks/move-provider-catalog-to-provider-crate.md` | depends: unify-provider-modules
- `move-tui-effects-into-io-actor` — R4 P0 — [Architecture / Actors] — `tasks/move-tui-effects-into-io-actor.md` | depends: finish-io-migration | blocks: collapse-effect-payload-indirection
- `permission-actor-owns-approvals` — R4 P0 — [Architecture / Security] — `tasks/permission-actor-owns-approvals.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
- `remove-direct-appstate-mutations` — R4 P0 — [Architecture / Actors] — `tasks/remove-direct-appstate-mutations.md` | depends: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state, config-ssot-via-configactor, actor-lifecycle-and-handle-registry
- `remove-login-config-test-shim` — R4 P0 — [Architecture / Actors] — `tasks/remove-login-config-test-shim.md` | depends: actor-owned-state-ssot, config-ssot-via-configactor
- `session-actor-owns-session-state` — R4 P0 — [Core / State] — `tasks/session-actor-owns-session-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection | blocks: turn-actor-owns-agent-turn-state
- `split-runie-core-into-domain-and-io-crates` — R4 P0 — [Architecture / Actors] — `tasks/split-runie-core-into-domain-and-io-crates.md` | depends: finish-io-migration, delete-async-io-bridge, fold-state-into-model-state, rename-core-ui-to-view | blocks: gate-or-move-single-consumer-core-modules, unify-duplicate-module-names-core-tui, consolidate-config-modules-into-dir
- `test-actor-harness` — R4 P0 — [Architecture / Testing] — `tasks/test-actor-harness.md` | depends: actor-lifecycle-and-handle-registry, event-taxonomy-for-actor-state-sync, app-state-read-only-projection | blocks: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions
- `turn-actor-owns-agent-turn-state` — R4 P0 — [Architecture / Actors] — `tasks/turn-actor-owns-agent-turn-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, session-actor-owns-session-state, input-actor-owns-input-state
- `ui-control-actor-owns-dialog-state` — R4 P0 — [Architecture / Actors] — `tasks/ui-control-actor-owns-dialog-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection | blocks: view-actor-owns-view-state, input-actor-owns-input-state, turn-actor-owns-agent-turn-state, remove-direct-appstate-mutations
- `unified-dsl-intents-for-state-mutations` — R4 P0 — [Input / Commands] — `tasks/unified-dsl-intents-for-state-mutations.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, declarative-actor-dsl
- `unify-event-vocabulary` — R4 P0 — [Architecture / Actors] — `tasks/unify-event-vocabulary.md`
- `unify-permission-gate` — R4 P0 — [Architecture / Security] — `tasks/unify-permission-gate.md` | blocks: unify-approval-decision, derive-confirmation-from-permissions
- `unify-persistence-actors` — R4 P0 — [Architecture / Actors] — `tasks/unify-persistence-actors.md`
- `view-actor-owns-view-state` — R4 P0 — [TUI / Rendering] — `tasks/view-actor-owns-view-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, session-actor-owns-session-state, input-actor-owns-input-state, ui-control-actor-owns-dialog-state
- `add-engine-tool-tests` — R4 P1 — [Tools] — `tasks/add-engine-tool-tests.md`
- `add-tool-output-constructors` — R4 P1 — [Tools] — `tasks/add-tool-output-constructors.md`
- `aggressive-event-consolidation` — R4 P1 — [Core / State] — `tasks/aggressive-event-consolidation.md`
- `collapse-headless-binaries-into-one-cli` — R4 P1 — [Configuration] — `tasks/collapse-headless-binaries-into-one-cli.md`
- `collapse-tool-runtime-traits` — R4 P1 — [Tools] — `tasks/collapse-tool-runtime-traits.md`
- `completion-actor-owns-completion-state` — R4 P1 — [Input / Commands] — `tasks/completion-actor-owns-completion-state.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, input-actor-owns-input-state
- `consolidate-dual-path-modules` — R4 P1 — [Configuration] — `tasks/consolidate-dual-path-modules.md`
- `consolidate-login-flow-all-files` — R4 P1 — [Core / State] — `tasks/consolidate-login-flow-all-files.md` | depends: consolidate-login-flow-handlers
- `consolidate-login-flow-handlers` — R4 P1 — [Core / State] — `tasks/consolidate-login-flow-handlers.md`
- `consolidate-settings-providers-dialog` — R4 P1 — [Configuration] — `tasks/consolidate-settings-providers-dialog.md` | depends: unify-provider-modules, actor-owned-state-ssot, config-ssot-via-configactor
- `dedupe-tool-execution-loop` — R4 P1 — [Core / State] — `tasks/dedupe-tool-execution-loop.md` | depends: extract-streaming-tool-parser
- `delete-async-io-bridge` — R4 P1 — [Architecture / Actors] — `tasks/delete-async-io-bridge.md` | depends: finish-io-migration
- `delete-dead-history-action-vimnav` — R4 P1 — [Core / State] — `tasks/delete-dead-history-action-vimnav.md`
- `delete-dead-theme-async-loaders` — R4 P1 — [Configuration] — `tasks/delete-dead-theme-async-loaders.md`
- `delete-dead-tuple-actor-handles-fields` — R4 P1 — [Architecture / Actors] — `tasks/delete-dead-tuple-actor-handles-fields.md`
- `delete-runie-core-confirmation` — R4 P1 — [Core / State] — `tasks/delete-runie-core-confirmation.md`
- `delete-runie-core-skill-module` — R4 P1 — [Core / State] — `tasks/delete-runie-core-skill-module.md`
- `env-actor-owns-git-cwd` — R4 P1 — [Configuration] — `tasks/env-actor-owns-git-cwd.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry
- `extract-ci-setup-action` — R4 P1 — [Configuration] — `tasks/extract-ci-setup-action.md` | depends: centralize-test-verification
- `extract-streaming-tool-parser` — R4 P1 — [Core / State] — `tasks/extract-streaming-tool-parser.md` | depends: canonicalize-tool-call | blocks: dedupe-tool-execution-loop
- `fff-indexer-owns-file-picker-results` — R4 P1 — [Tools] — `tasks/fff-indexer-owns-file-picker-results.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry
- `fix-duplicate-cargo-toml-keys` — R4 P1 — [Configuration] — `tasks/fix-duplicate-cargo-toml-keys.md`
- `fix-permission-gate-rename-in-testing` — R4 P1 — [Architecture / Testing] — `tasks/fix-permission-gate-rename-in-testing.md`
- `generic-actor-reply` — R4 P1 — [Architecture / Actors] — `tasks/generic-actor-reply.md` | blocks: reduce-actor-handle-boilerplate
- `group-session-modules-into-dir` — R4 P1 — [Sessions] — `tasks/group-session-modules-into-dir.md` | depends: consolidate-dual-path-modules
- `hoist-cargo-workspace-deps` — R4 P1 — [Configuration] — `tasks/hoist-cargo-workspace-deps.md`
- `introduce-tool-definition-macro` — R4 P1 — [Tools] — `tasks/introduce-tool-definition-macro.md`
- `move-actor-trait-into-actors-dir` — R4 P1 — [Architecture / Actors] — `tasks/move-actor-trait-into-actors-dir.md`
- `notification-actor-owns-transient-messages` — R4 P1 — [Core / State] — `tasks/notification-actor-owns-transient-messages.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
- `reduce-actor-handle-boilerplate` — R4 P1 — [Architecture / Actors] — `tasks/reduce-actor-handle-boilerplate.md` | depends: generic-actor-reply
- `remove-appstate-cfg-test-branches` — R4 P1 — [Core / State] — `tasks/remove-appstate-cfg-test-branches.md`
- `rename-core-ui-to-view` — R4 P1 — [Architecture / Actors] — `tasks/rename-core-ui-to-view.md` | depends: fold-state-into-model-state
- `replace-git2-with-cli` — R4 P1 — [Architecture / Actors] — `tasks/replace-git2-with-cli.md` | depends: finish-io-migration
- `share-fff-error-helpers` — R4 P1 — [Tools] — `tasks/share-fff-error-helpers.md`
- `share-minimax-sse-fixtures` — R4 P1 — [Architecture / Testing] — `tasks/share-minimax-sse-fixtures.md`
- `single-source-testing-docs` — R4 P1 — [Configuration] — `tasks/single-source-testing-docs.md`
- `trust-actor-owns-trust-decisions` — R4 P1 — [Architecture / Security] — `tasks/trust-actor-owns-trust-decisions.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
- `unify-approval-decision` — R4 P1 — [Architecture / Security] — `tasks/unify-approval-decision.md` | depends: unify-permission-gate
- `unify-provider-modules` — R4 P1 — [Architecture / Actors] — `tasks/unify-provider-modules.md`
- `arch-review-2026-06` — R4 P2 — [Architecture / Actors] — `tasks/arch-review-2026-06.md`
- `centralize-error-type` — R4 P2 — [Core / State] — `tasks/centralize-error-type.md`
- `collapse-effect-payload-indirection` — R4 P2 — [Architecture / Actors] — `tasks/collapse-effect-payload-indirection.md`
- `collapse-event-aliases` — R4 P2 — [Core / State] — `tasks/collapse-event-aliases.md`
- `collapse-event-variants-subdir` — R4 P2 — [Core / State] — `tasks/collapse-event-variants-subdir.md`
- `consolidate-config-modules-into-dir` — R4 P2 — [Configuration] — `tasks/consolidate-config-modules-into-dir.md` | depends: consolidate-login-flow-handlers, split-runie-core-into-domain-and-io-crates
- `consolidate-mark-dirty-calls` — R4 P2 — [Core / State] — `tasks/consolidate-mark-dirty-calls.md`
- `consolidate-slash-command-handlers` — R4 P2 — [Input / Commands] — `tasks/consolidate-slash-command-handlers.md`
- `consolidate-tiny-binary-crates` — R4 P2 — [Architecture / Actors] — `tasks/consolidate-tiny-binary-crates.md`
- `consolidate-update-dialog-files` — R4 P2 — [Architecture / Refactoring] — `tasks/consolidate-update-dialog-files.md` | depends: rename-update-dialog-panel
- `dedupe-config-actor-mutations` — R4 P2 — [Architecture / Actors] — `tasks/dedupe-config-actor-mutations.md` | depends: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection | blocks: config-ssot-via-configactor
- `dedupe-fresh-state-test-helper` — R4 P2 — [Architecture / Testing] — `tasks/dedupe-fresh-state-test-helper.md`
- `dedupe-git-status-formatter` — R4 P2 — [Tools] — `tasks/dedupe-git-status-formatter.md` | depends: replace-git2-with-cli
- `dedupe-scoped-model-toggle` — R4 P2 — [Core / State] — `tasks/dedupe-scoped-model-toggle.md`
- `dedupe-turn-complete-last-calls` — R4 P2 — [Core / State] — `tasks/dedupe-turn-complete-last-calls.md`
- `dedupe-update-input-history` — R4 P2 — [Input / Commands] — `tasks/dedupe-update-input-history.md`
- `dedupe-yolo-sink-setup` — R4 P2 — [Architecture / Actors] — `tasks/dedupe-yolo-sink-setup.md`
- `delete-provider-message-enum` — R4 P2 — [Core / State] — `tasks/delete-provider-message-enum.md`
- `delete-runie-core-utils` — R4 P2 — [Core / State] — `tasks/delete-runie-core-utils.md`
- `derive-confirmation-from-permissions` — R4 P2 — [Architecture / Security] — `tasks/derive-confirmation-from-permissions.md` | depends: unify-permission-gate
- `drop-event-aliases` — R4 P2 — [Architecture / Actors] — `tasks/drop-event-aliases.md`
- `drop-event-bus-replay-buffer` — R4 P2 — [Architecture / Actors] — `tasks/drop-event-bus-replay-buffer.md` | blocks: drop-small-stdlib-replaceable-deps
- `drop-small-stdlib-replaceable-deps` — R4 P2 — [Configuration] — `tasks/drop-small-stdlib-replaceable-deps.md` | depends: drop-event-bus-replay-buffer
- `drop-thiserror-and-async-trait-deps` — R4 P2 — [Configuration] — `tasks/drop-thiserror-and-async-trait-deps.md`
- `flatten-update-login-flow-tests-dir` — R4 P2 — [Configuration] — `tasks/flatten-update-login-flow-tests-dir.md` | depends: consolidate-dual-path-modules
- `fold-protocol-into-core` — R4 P2 — [Architecture / Actors] — `tasks/fold-protocol-into-core.md`
- `fold-runie-engine-into-agent` — R4 P2 — [Architecture / Actors] — `tasks/fold-runie-engine-into-agent.md`
- `fold-state-into-model-state` — R4 P2 — [Core / State] — `tasks/fold-state-into-model-state.md`
- `gate-or-move-single-consumer-core-modules` — R4 P2 — [Configuration] — `tasks/gate-or-move-single-consumer-core-modules.md` | depends: split-runie-core-into-domain-and-io-crates
- `generate-config-schema-from-code` — R4 P2 — [Configuration] — `tasks/generate-config-schema-from-code.md`
- `generate-ui-elements-from-model` — R4 P2 — [TUI / Rendering] — `tasks/generate-ui-elements-from-model.md` | depends: rename-core-ui-to-view
- `inline-or-document-core-ui-shim` — R4 P2 — [Configuration] — `tasks/inline-or-document-core-ui-shim.md`
- `inline-tui-ipc-reexport` — R4 P2 — [Configuration] — `tasks/inline-tui-ipc-reexport.md`
- `merge-diff-modules` — R4 P2 — [TUI / Rendering] — `tasks/merge-diff-modules.md`
- `merge-tool-status-state` — R4 P2 — [Core / State] — `tasks/merge-tool-status-state.md`
- `reconsider-clipboard-image-deps` — R4 P2 — [Configuration] — `tasks/reconsider-clipboard-image-deps.md`
- `reconsider-notify-config-watcher` — R4 P2 — [Configuration] — `tasks/reconsider-notify-config-watcher.md`
- `reconsider-redb-session-store` — R4 P2 — [Sessions] — `tasks/reconsider-redb-session-store.md`
- `reconsider-schemars-jsonschema` — R4 P2 — [Configuration] — `tasks/reconsider-schemars-jsonschema.md`
- `reconsider-tiktoken-rs` — R4 P2 — [Configuration] — `tasks/reconsider-tiktoken-rs.md`
- `relocate-loose-tests-files` — R4 P2 — [Configuration] — `tasks/relocate-loose-tests-files.md` | depends: consolidate-dual-path-modules
- `relocate-stray-tool-parser-tests` — R4 P2 — [Configuration] — `tasks/relocate-stray-tool-parser-tests.md` | depends: consolidate-dual-path-modules
- `rename-model-snapshot-to-compaction` — R4 P2 — [Core / State] — `tasks/rename-model-snapshot-to-compaction.md`
- `rename-test-submission-id` — R4 P2 — [Architecture / Testing] — `tasks/rename-test-submission-id.md`
- `replace-single-use-deps-with-stdlib` — R4 P2 — [Configuration] — `tasks/replace-single-use-deps-with-stdlib.md`
- `shrink-tasks-planning-debt` — R4 P2 — [Configuration] — `tasks/shrink-tasks-planning-debt.md`
- `simplify-actor-trait` — R4 P2 — [Architecture / Actors] — `tasks/simplify-actor-trait.md` | depends: unify-persistence-actors
- `split-files-at-limit-round-2` — R4 P2 — [Architecture / Refactoring] — `tasks/split-files-at-limit-round-2.md`
- `standardize-test-layout-inline` — R4 P2 — [Configuration] — `tasks/standardize-test-layout-inline.md` | depends: relocate-loose-tests-files
- `sweep-allow-suppressions` — R4 P2 — [Core / State] — `tasks/sweep-allow-suppressions.md`
- `unify-build-system-prompt` — R4 P2 — [Core / State] — `tasks/unify-build-system-prompt.md`
- `unify-duplicate-module-names-core-tui` — R4 P2 — [Configuration] — `tasks/unify-duplicate-module-names-core-tui.md` | depends: split-runie-core-into-domain-and-io-crates
- `audit-borrow-workarounds` — R4 P3 — [Core / State] — `tasks/audit-borrow-workarounds.md` | depends: consolidate-login-flow-handlers
- `consolidate-login-logout-tests` — R4 P3 — [Architecture / Testing] — `tasks/consolidate-login-logout-tests.md` | depends: consolidate-login-flow-handlers
- `dedupe-messages-changed-ensure-fresh` — R4 P3 — [Architecture / Testing] — `tasks/dedupe-messages-changed-ensure-fresh.md`
- `extract-appstate-helpers` — R4 P3 — [Core / State] — `tasks/extract-appstate-helpers.md`
- `fold-testing-into-agent-tests` — R4 P3 — [Configuration] — `tasks/fold-testing-into-agent-tests.md`
- `remove-empty-match-arms` — R4 P3 — [Core / State] — `tasks/remove-empty-match-arms.md`
- `rename-update-dialog-panel` — R4 P3 — [Configuration] — `tasks/rename-update-dialog-panel.md`
- `single-source-dev-commands` — R4 P3 — [Configuration] — `tasks/single-source-dev-commands.md`
- `sweep-to-string-allocs` — R4 P3 — [Core / State] — `tasks/sweep-to-string-allocs.md`
- `theme-color-accessor-macro` — R4 P3 — [TUI / Rendering] — `tasks/theme-color-accessor-macro.md`
- `tighten-build-lint-complexity-heuristic` — R4 P3 — [Architecture / Refactoring] — `tasks/tighten-build-lint-complexity-heuristic.md`
- `unify-markdown-rendering` — R4 P3 — [TUI / Rendering] — `tasks/unify-markdown-rendering.md`
- `use-workspace-deps-in-runie-testing` — R4 P3 — [Configuration] — `tasks/use-workspace-deps-in-runie-testing.md`
- `r5-extract-tool-stream` — R5 P0 — [Tools] — `tasks/r5-extract-tool-stream.md` | blocks: r5-provider-protocol-trait, r5-partial-json-repair
- `r5-provider-protocol-trait` — R5 P0 — [Architecture / Actors] — `tasks/r5-provider-protocol-trait.md` | depends: r5-extract-tool-stream
- `r5-conversation-sanitize-pipeline` — R5 P1 — [Core / State] — `tasks/r5-conversation-sanitize-pipeline.md`
- `r5-markdown-healing-stream` — R5 P1 — [TUI / Rendering] — `tasks/r5-markdown-healing-stream.md`
- `r5-partial-json-repair` — R5 P1 — [Tools] — `tasks/r5-partial-json-repair.md` | depends: r5-extract-tool-stream
- `r5-populate-parts-streaming` — R5 P1 — [Core / State] — `tasks/r5-populate-parts-streaming.md` | blocks: r5-per-channel-decoders, r5-parts-canonical-migration
- `r5-think-filter` — R5 P1 — [Core / State] — `tasks/r5-think-filter.md`
- `r5-paced-text-rendering` — R5 P2 — [TUI / Rendering] — `tasks/r5-paced-text-rendering.md`
- `r5-parts-canonical-migration` — R5 P2 — [Core / State] — `tasks/r5-parts-canonical-migration.md` | depends: r5-populate-parts-streaming
- `r5-per-channel-decoders` — R5 P2 — [Architecture / Actors] — `tasks/r5-per-channel-decoders.md` | depends: r5-populate-parts-streaming