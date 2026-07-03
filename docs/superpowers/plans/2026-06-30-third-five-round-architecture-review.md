# Third Five-Round Architecture & Code Review — Synthesis

**Date:** 2026-06-30  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Replace custom code with crates/libraries/OS features.

Five new read-only sub-agents drilled into:
1. Error handling / tracing / telemetry / logging / metrics
2. Protocol / IPC / serialization / transport / network leftovers
3. Declarative DSL / commands / slash parser / macros / codegen
4. Actor lifecycle / handle registry / state ownership / concurrency primitives
5. Provider/model/catalog/cache and agent turn/subagent internals

Findings were cross-checked with `ctx7` (metrics, jsonrpsee) and patterns from `~/Code/agents/{codex,goose,kimi-code,gptme,aider}`. No code was executed or modified.

**TUI design freeze:** behavior-only; visual element design and composition stay frozen.

---

## 1. Consolidated ranked findings

| P | Finding | Files / lines | Replacement | Est. LOC removed | Risk | Borrow from agents | Task |
|---|---------|---------------|-------------|------------------|------|--------------------|------|
| P0 | Agent turn runs inside `RactorAgentActor::handle`, blocking mailbox | `runie-agent/src/actor.rs:96-151` | spawn `run_agent_turn` as `tokio::task`; store `AbortHandle` in actor state | ~60 | High | Goose/Codex per-turn abort handle | **new** `offload-agent-turn-from-actor-handler-to-tokio-task.md` |
| P0 | DSL `Flow::map`/`filter`/`branch` are no-ops | `runie-core/src/dsl/flow.rs:169-214`, `runtime.rs`, `examples.rs` | delete broken DSL veneer | 800–900 | High | Goose/gptme plain handlers | **new** `delete-broken-dsl-flow-combinators-and-thread-local-runtime.md` |
| P0 | Declarative command registration dead code + `Box::leak` global `RwLock` | `runie-core/src/declarative/register.rs` | delete; route through `CommandRegistry` | 180–220 | Low | Goose single slash-command list | **new** `delete-dead-declarative-command-registration.md` |
| P0 | Command type tetris: `CommandSpec`/`CommandDef`/`DeclarativeCommandDef`/`NamedHandler` | `commands/dsl/spec.rs`, `registry.rs`, `declarative/types.rs` | single `Command` struct with `Action` enum | 550–600 | Medium | Goose `SlashCommandEntry` | **new** `collapse-command-types-to-single-command-action-enum.md` |
| P0 | Leader TCP drops split UTF-8 chunks | `runie-core/src/actors/leader/actor.rs:232-275` | `tokio_util::codec::LinesCodec` or `BufReader::lines()` | ~40 | Low | Goose `LinesCodec` | **new** `fix-leader-tcp-utf8-split-with-linescodec.md` |
| P0 | `AgentState` and `TurnState` are near-identical duplicates | `model/state/agent.rs`, `actors/turn/state.rs` | make `AgentState` a projection of `TurnState` | ~120–150 | Medium | Goose/Codex single source | **new** `merge-agentstate-into-turnstate-projection.md` |
| P1 | Headless turn re-implements streaming response parser | `runie-agent/src/headless/mod.rs:194-330`, `stream_response.rs` | single `stream_response_to<Publisher>` | 180–220 | Medium | Codex single inference context | **new** `unify-headless-and-tui-streaming-response-parser.md` |
| P1 | Per-actor handle wrappers still wrap `ractor::ActorRef` | `actors/ractor_adapter.rs`, `config/config_handle.rs`, `provider/ractor_provider.rs`, etc. | use `ractor::ActorRef` + `cast!`/`call!`; delete wrappers | ~250 | High | idiomatic ractor | **new** `collapse-remaining-actor-handle-wrappers-to-actorref.md` |
| P1 | `ApprovalRegistry` mutex map should live in actor state | `permissions/approval_registry.rs`, `actors/permission/ractor_permission.rs` | move `HashMap` into `PermissionActorState` | ~80 | Medium | ractor state ownership | **new** `move-approval-registry-into-permission-actor-state.md` |
| P1 | Custom `RpcReply<T>` duplicates `ractor::RpcReplyPort` | `actors/ractor_adapter.rs:99-127` | use `ractor::call!`/`call_t!` | ~30 | Medium | ractor built-ins | **new** `replace-rpcreply-with-ractor-rpcreplyport.md` |
| P1 | Abort detected via side event-bus subscription | `runie-agent/src/actor.rs:127-149` | handle `AgentMsg::Abort` directly after offload | ~25 | Medium | actor messages vs bus tap | **new** `route-agent-abort-through-actor-message-not-event-bus.md` |
| P1 | Slash parser uses `split_once(' ')` | `commands/registry.rs:264-271` | `shell-words` tokenization | ~50–80 | Low-Medium | gptme `shlex` | **new** `use-shell-words-for-slash-command-parsing.md` |
| P1 | Form submission dual paths (`submit_factory` vs `SubmitCommand`) | `update/dialog/form/mod.rs`, `dialog/dsl/form.rs`, `spec.rs` | route through command registry | 150–180 | Medium | Goose recipe forms | **new** `unify-form-submit-paths-through-command-registry.md` |
| P1 | `DeclarativeCommandYaml` still uses string `kind_type` | `declarative/types.rs:43-107` | `#[serde(tag = "type")]` enum | ~50 | Low | Goose typed recipe params | **new** `use-tagged-enum-for-declarative-command-kind.md` |
| P1 | Frontmatter still hand-rolled | `resource_loader.rs:105-151` | `pulldown-cmark-frontmatter` | ~200 | Low-Medium | Codex skills | **new** `actually-use-pulldown-cmark-frontmatter-in-resource-loader.md` |
| P1 | Error types fragmented; `anyhow` chains flattened | `error.rs`, `provider/provider_trait.rs`, `provider_event.rs`, etc. | standardize on `thiserror` with `#[source]` | ~80–120 | Medium | Goose `thiserror` | **new** `standardize-error-types-on-thiserror-with-source-chains.md` |
| P1 | CLI errors use plain `eprintln!` | `runie-cli/src/main.rs:100-102` | `color-eyre` + `human-panic` | ~5 | Low | standard CLI | **new** `use-color-eyre-and-human-panic-for-cli-errors.md` |
| P1 | Telemetry is ad-hoc `tracing::info!`; no metrics facade | `update/system/model.rs`, `update/agent/core/mod.rs`, `config/mod.rs` | `metrics` crate + exporter | ~10 | Low | Goose observation layer | **new** `adopt-metrics-facade-for-telemetry.md` |
| P1 | Durable/headless event vocabularies duplicate canonical `Event` | `event/durable.rs`, `event/headless.rs`, `provider_event.rs` | derive views from `Event` | 150–200 | Medium | Codex tiering | **new** `derive-durable-and-headless-events-from-canonical-event.md` |
| P1 | Retry split: `backon` for validation, `reqwest_eventsource` for streams | `provider/src/openai/stream.rs`, `provider/src/lib.rs` | unify on `backon` for stream establishment | ~20 | Low-Medium | `backon::Retryable` | **new** `unify-provider-retry-on-backon-including-streams.md` |
| P2 | Leader shutdown manually tracks joins instead of ractor supervision | `actors/leader/actor.rs`, `handle.rs` | ractor supervision tree | ~120 | Medium-High | Erlang/OTP via ractor | **new** `use-ractor-supervision-for-leader-shutdown.md` |
| P2 | `HeadlessRuntime` waits for config via event loop | `headless_runtime.rs:36-61` | direct `config_handle.get_config().await` with timeout | ~20 | Low-Medium | deterministic RPC | **new** `replace-headless-config-wait-with-direct-rpc.md` |
| P2 | `EventBus` tiny buffer drops lagging control events | `bus.rs:46-66` | larger buffer + dedicated control `mpsc` | ~10 | Medium | Goose bounded replay | **new** `harden-eventbus-control-plane-against-lagging-events.md` |
| P2 | FFF indexer global `OnceLock<RwLock<Option<SearchIndexState>>>` | `actors/fff_indexer/mod.rs`, `ractor_fff_indexer.rs` | actor state or `Arc<SearchIndex>` via messages | ~40 | Medium | no service locator | **new** `remove-fff-indexer-global-state-locator.md` |
| P2 | DSL `RealRuntime` thread-local global | `dsl/runtime.rs:229-270` | explicit `&mut dyn Runtime` context | ~40 | Medium | explicit context | **new** `make-dsl-runtime-context-explicit-instead-of-thread-local.md` |
| P2 | Declarative intent registry global `RwLock` + `Box::leak` | `declarative/register.rs:64-82` | startup-time immutable map or `phf` | ~30 | Low-Medium | bootstrap map | **new** `remove-global-intent-events-registry-leak.md` |
| P2 | Command handlers duplicate async/sync fallback | `commands/dsl/handlers/session/run.rs` | single helper or actor-only path | ~80–100 | Low | Goose `SessionManager` | **new** `centralize-session-command-async-sync-fallback.md` |
| P2 | Trigger parsing custom string heuristics | `declarative/types.rs:151-171`, `loader.rs:150-186` | typed YAML or `winnow` | ~50–70 | Low | Goose typed triggers | **new** `type-declarative-triggers-instead-of-string-heuristics.md` |
| P2 | Embedded command YAML 40 `include_str!` constants | `commands/dsl/embedded_commands.rs` | `include_dir!` | ~70–90 | Low | Codex bundled resources | **new** `use-include-dir-for-embedded-command-yaml.md` |
| P2 | Build-time agent manifest checksum validation | `runie-core/build.rs:201-231` | delete or `include_dir!` | ~30 | Low | standard | **new** `remove-build-time-agent-manifest-checksum-validation.md` |
| P2 | Permission rule glob matching recompiles `glob::Pattern` | `permissions/rules.rs`, `permissions/mod.rs` | `globset::GlobSet` | ~30 | Low | Goose `globset` | **new** `use-globset-for-permission-rule-matching.md` |
| P2 | `DynProvider` backward-compat shim | `runie-provider/src/lib.rs:45-124` | use `BuiltProvider` directly | ~80–100 | Low | no wrapper | **new** `delete-dynprovider-backward-compat-shim.md` |
| P2 | `MockProvider` keyword heuristics | `runie-provider/src/mock.rs:71-156` | fixture/replay provider | ~100–120 | Medium | kimi-code recorded events | **new** `replace-mock-provider-keyword-heuristics-with-fixtures.md` |
| P2 | Command palette custom scoring over `sublime_fuzzy` | `model/state/ranking.rs`, `helpers.rs` | `nucleo-matcher` or fuzzy-only | ~40–60 | Low | gptme fuzzy | **new** `simplify-command-palette-ranking-with-nucleo-matcher.md` |
| P2 | Tracing subscriber is dev-only; no JSON/file/spans | `runie-core/src/tracing_init.rs`, `runie-cli/src/main.rs`, `runie-tui/src/main.rs` | `tracing-subscriber` JSON + `tracing-appender` + spans | ~30–50 | Low | Codex/Goose | **new** `upgrade-tracing-subscriber-with-json-file-appender-and-spans.md` |
| P3 | `RunieError` transparent `anyhow` wrapper | `runie-core/src/error.rs:35-56` | use `anyhow::Error` directly or real `thiserror` enum | ~25 | Low | don't wrap wrapper | **new** `remove-runieerror-transparent-wrapper.md` |
| P3 | Agent emit closure wraps `EventBus` in `Mutex` | `runie-agent/src/actor.rs:153-160`, `stream_response.rs:24` | `Arc<dyn Fn(Event) + Send + Sync>` | ~10 | Low | no mutex around publish | **new** `remove-mutex-around-eventbus-publish-in-agent-emit.md` |
| P3 | `ProviderConfigBox` custom cloneable trait-object wrapper | `runie-core/src/proto/provider.rs:27-53` | `Arc<dyn ProviderConfig>` directly | ~30 | Low | standard | **new** `delete-providerconfigbox-wrapper.md` |
| P3 | ACP references linger in docs | `docs/Architecture.md`, `DSL_Vision.md`, `proto/mod.rs` | delete/update | trivial | None | n/a | minor doc cleanup |
| P3 | `dsl/examples.rs` documents broken DSL | `runie-core/src/dsl/examples.rs` | delete | 135 | None | n/a | covered by DSL deletion |
| P3 | `CommandCategory::FromStr` custom aliases | `commands/dsl/category.rs` | `strum::EnumString` | ~20 | Low | standard | existing strum task |
| P3 | Provider registry YAML hardcoded `vec!` of `include_str!` | `runie-core/src/provider/registry_data.rs` | `include_dir!` | ~30 | Low | Goose catalog | minor / include_dir task |
| P3 | `Role` duplicates strum methods | `proto/message/mod.rs:57-89` | use strum derives | ~30 | Low | standard | minor |

---

## 2. `ctx7` / crate documentation highlights

| Crate | Why it fits | Evidence |
|-------|-------------|----------|
| `metrics` | Lightweight facade for counters/histograms/gauges. | `counter!("x", "label" => "y").increment(1)`, `histogram!(...)` |
| `jsonrpsee` | Type-safe JSON-RPC server/client with `#[rpc]` macro. | `#[rpc(server, client, namespace = "state")]` + `#[method(name = "getKeys")]` |
| `tokio-util::codec::LinesCodec` | Already used by Goose for line-framed streams; fixes UTF-8 split bug. | standard pattern |

---

## 3. Cross-agent learnings

- **Codex:** layered tracing subscriber (JSON + env-filter), `color-eyre`/`human-panic`, schema-driven enums, bundled resources via `include_dir`, single inference context for CLI/TUI, explicit request contexts instead of globals.
- **Goose:** `thiserror` everywhere, single `SlashCommandEntry`, typed recipe parameters, `globset` for file/permission matching, ractor-style actor state ownership, SQLite sessions, process-group hygiene.
- **kimi-code:** recorded vs live event split, modal permission coordinator.
- **gptme:** `shlex` slash-arg tokenization, regex tool-marker stripping.

**Takeaways for Runie:**
- Offload long-running work from actor handlers first; it unlocks correct abort and simpler state.
- Delete the broken actor DSL before it spreads.
- Replace globals/thread-locals with explicit contexts.
- Use `thiserror` + source chains; don't flatten errors to strings.
- Adopt `metrics` facade now while telemetry surface is small.
- Use `LinesCodec` for all line-framed transports.

---

## 4. Recommended execution order

1. **P0 — offload agent turn** and **delete broken DSL/declarative dead code** — these unblock many other cleanups.
2. **P0 — fix leader TCP UTF-8 split** — correctness bug.
3. **P0 — collapse command types** and **merge AgentState/TurnState** — large simplifications.
4. **P1 — actor plumbing collapse** (handle wrappers, `RpcReply`, `ApprovalRegistry` in state, abort via message).
5. **P1 — headless/TUI streaming parser unification**.
6. **P1 — error/telemetry/tracing upgrades** (`thiserror`, `metrics`, JSON subscriber, `color-eyre`).
7. **P1 — slash/form/declarative DSL fixes** (shell-words, tagged enums, frontmatter, unified form submit).
8. **P2 — leader supervision, headless RPC, event bus hardening, globals removal, build script cleanup, permission globset, mock fixtures.**
9. **P3 — small wrapper deletions and doc cleanup.**

---

## 5. New tasks created in this pass (top 20 Pareto findings)

1. `offload-agent-turn-from-actor-handler-to-tokio-task.md`
2. `delete-broken-dsl-flow-combinators-and-thread-local-runtime.md`
3. `delete-dead-declarative-command-registration.md`
4. `collapse-command-types-to-single-command-action-enum.md`
5. `fix-leader-tcp-utf8-split-with-linescodec.md`
6. `merge-agentstate-into-turnstate-projection.md`
7. `unify-headless-and-tui-streaming-response-parser.md`
8. `collapse-remaining-actor-handle-wrappers-to-actorref.md`
9. `move-approval-registry-into-permission-actor-state.md`
10. `replace-rpcreply-with-ractor-rpcreplyport.md`
11. `route-agent-abort-through-actor-message-not-event-bus.md`
12. `use-shell-words-for-slash-command-parsing.md`
13. `unify-form-submit-paths-through-command-registry.md`
14. `use-tagged-enum-for-declarative-command-kind.md`
15. `actually-use-pulldown-cmark-frontmatter-in-resource-loader.md`
16. `standardize-error-types-on-thiserror-with-source-chains.md`
17. `use-color-eyre-and-human-panic-for-cli-errors.md`
18. `adopt-metrics-facade-for-telemetry.md`
19. `derive-durable-and-headless-events-from-canonical-event.md`
20. `unify-provider-retry-on-backon-including-streams.md`

Lower-ranked findings in the table above are documented for future task creation.

---

## 6. File inventory

- This synthesis: `docs/superpowers/plans/2026-06-30-third-five-round-architecture-review.md`
- Prior reviews: `2026-06-30-five-round-architecture-review.md`, `2026-06-30-crate-replacement-deep-dive.md`, `2026-06-30-second-five-round-architecture-review.md`
- Roadmap: `2026-06-28-runie-cleanup-roadmap.md`
- Tasks: `tasks/*.md`
