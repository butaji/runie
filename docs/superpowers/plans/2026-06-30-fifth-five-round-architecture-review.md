# Fifth Five-Round Architecture & Code Review

## Status

Synthesis complete. Findings are ranked below and tracked as new `tasks/` files.

## Goal

Pareto (80/20) simplification: less code, fewer custom implementations, fewer duplicate dependencies, and clearer boundaries. If a standard crate, OS feature, or build-tool feature can replace custom code, plan it that way.

## Methodology

- **Five read-only subagents** reviewed disjoint slices of the codebase (no code executed):
  1. Workspace, build, CI, dependencies, macros/codegen
  2. Core logic, domain model, DSL, actor lifecycle, state
  3. Provider, network, protocol, auth, security
  4. TUI, rendering, input, clipboard, theme
  5. Testing harness, fixtures, mocks, assertions
- **ctx7** was consulted for: `ratatui`, `tui-input`/`tui-textarea`, `secrecy`, `backon`, `wiremock`, `bon`, `clap`, `rmcp`.
- **~/Code/agents/*** survey covered Goose, Codex, gptme, Aider for provider traits, session storage, config layering, replay tests, MCP, retry policy, and CLI introspection patterns.

## Executive Summary

Most of the highest-impact simplifications are already in the task backlog from previous reviews (turn-state deduplication, command DSL collapse, permission-engine unification, provider retry unification, rusqlite session migration, MCP adoption, async-openai spike). This fifth pass surfaced **25 net-new tasks** focused on:

1. **Dependency/workspace hygiene** — unused direct deps, non-workspace pins, lockfile duplicates.
2. **Provider boundary cleanup** — delete thin wrappers, propagate `SecretString`, share one `reqwest::Client`, type OpenAI parsing.
3. **TUI widget completion** — actually replace the custom input box, panel list, and form fields; drop terminal brand tables.
4. **Testing helper consolidation** — adopt existing helpers everywhere, extract a shared `runie-test-utils` crate, delete dead modules.
5. **Configuration and model metadata** — project-level config layer, keyring-backed secrets, remote model metadata cache.
6. **Agent-pattern imports** — richer provider trait, typed provider errors, session/turn client split, request-body replay assertions.

## P0 / P1 Findings

| # | Area | File(s) | Issue | Replacement / Plan | Task |
|---|------|---------|-------|--------------------|------|
| 1 | Workspace | `crates/runie-tui/Cargo.toml:37-39` | `tui-textarea`, `ractor`, `tracing-subscriber` are declared but unused in `runie-tui` source | Delete the three direct deps | `remove-unused-direct-deps-from-runie-tui` |
| 2 | Workspace | `crates/runie-tui/Cargo.toml:30,33,36,40` | `opaline`, `syntect`, `tui-markdown`, `crokey` pinned inline, not in workspace manifest | Hoist to `[workspace.dependencies]` | `hoist-runie-tui-inline-deps-to-workspace` |
| 3 | Workspace | `crates/runie-provider/Cargo.toml:25,26,32` | `async-stream`, `reqwest-eventsource`, `wiremock` pinned inline | Hoist to workspace deps (or replace `async-stream`) | `hoist-runie-provider-inline-deps-to-workspace` |
| 4 | Provider | `crates/runie-provider/src/lib.rs:45-124`, `crates/runie-core/src/proto/provider.rs:23-53` | `DynProvider` and `ProviderConfigBox` are backward-compat wrappers | Delete wrappers; migrate callers to `BuiltProvider` / `Arc<dyn ProviderConfig>` | `delete-dynprovider-and-providerconfigbox-wrappers` |
| 5 | Provider | `crates/runie-provider/src/openai/mod.rs:10`, `crates/runie-core/src/auth/credential.rs:80-107` | API keys flow as plaintext `String` through provider stack | `secrecy::SecretString` everywhere; expose only at HTTP boundary | `propagate-secretstring-through-provider-stack` |
| 6 | Config/Auth | `crates/runie-core/src/auth/credential.rs:54-67` | `dotenvy::dotenv()` mutates the real process environment, forcing `ENV_LOCK` serialisation | Load `.env` into a local `HashMap` with `dotenvy::from_filename_iter` or `figment::Env` | `stop-dotenvy-from-mutating-process-environment` |
| 7 | Provider | `crates/runie-provider/src/lib.rs:255-286`, `crates/runie-provider/src/openai/mod.rs:19-25` | Every `validate_api_key` call builds a fresh `reqwest::Client` | Inject/share one configured `reqwest::Client` with TLS/proxy/UA settings | `share-configured-reqwest-client-for-provider` |
| 8 | Provider | `crates/runie-provider/src/openai/protocol.rs:109-255`, `stream.rs:216-255` | Chunk/error parsing manually navigates `serde_json::Value` and string-matches error codes | Strongly typed structs or `async-openai` types | `type-openai-chunk-and-error-parsing` |
| 9 | TUI | `crates/runie-tui/src/ui/input.rs:1-286`, `popups/panel/list.rs:1-308`, `popups/panel/form.rs:311-447` | Custom input box, panel list, and form fields still exist despite `tui-textarea`/`ratatui::List` deps | Replace with `tui-textarea` / `ratatui::widgets::List` while preserving visual freeze | `finish-replacing-custom-tui-widgets` |
| 10 | Testing | `crates/runie-agent/src/tests/**/*.rs` | Tests duplicate `Arc<Mutex<Vec<Event>>>` capture closures and filter/count assertions | Adopt `runie_testing::capture_events` and assertion helpers everywhere | `adopt-testing-helpers-across-all-tests` |
| 11 | Testing | `crates/runie-core/src/tests/support.rs`, `crates/runie-testing/src/tests/state.rs` | Test helpers duplicated between `runie-core` tests and `runie-testing` | Extract `runie-test-utils` crate depended on by both | `extract-runie-test-utils-crate` |
| 12 | Provider/Agents | `crates/runie-core/src/provider/provider_trait.rs` | `Provider` trait lacks metadata, retry config, fast-model fallback | Extend trait with `ProviderMetadata`, `RetryConfig`, `complete_fast` default method | `enrich-provider-trait-with-metadata-and-retry-config` |
| 13 | Provider/Agents | `crates/runie-core/src/provider/provider_trait.rs`, `crates/runie-provider/src/retry.rs` | Provider errors collapse to `anyhow`; retry matches error strings | Typed `ProviderError` enum with `telemetry_type()` | `introduce-typed-provider-errors` |
| 14 | Agent/Providers | `crates/runie-core/src/actors/provider.rs`, `crates/runie-agent/src/actor.rs` | No separation between long-lived model client and per-turn session | Split `ModelClient` (session-scoped) from `TurnSession` (turn-scoped) | `split-modelclient-and-turnsession` |
| 15 | Config/Agents | `crates/runie-core/src/config/config_impl.rs` | Only `~/.runie/config.toml`; no project-level overrides; secrets in plain TOML | Layered config + keyring-backed secrets (Goose/gptme pattern) | `add-project-level-config-layer-and-keyring-secrets` |

## P2 Findings

| # | Area | File(s) | Issue | Replacement / Plan | Task |
|---|------|---------|-------|--------------------|------|
| 16 | Workspace | `crates/runie-core/src/commands/dsl/embedded_commands.rs:14-82` | 40 manual `include_str!` constants for slash commands | Single `include_dir!("resources/commands")` scan | `load-slash-commands-with-include-dir` |
| 17 | Workspace | `crates/runie-core/src/event/generated/*.rs` | ~1,450 LOC "generated" files are committed verbatim and not regenerated | Real `build.rs` generator or `strum` derives + delete files | `generate-event-taxonomy-or-delete-generated-files` |
| 18 | Workspace | `Cargo.lock` | 53 duplicate crate names (`pulldown-cmark`, `ratatui`, `reqwest`, `clap`, etc.) | `cargo update` + `deny.toml` `multiple-versions = "deny"` with justified skips | `deduplicate-cargo-lock-versions-and-deny` |
| 19 | TUI | `crates/runie-tui/src/popups.rs:58-89` | File picker reimplements the same custom list logic | Shared `ratatui::widgets::List` helper | `unify-file-picker-and-panel-list-widget` |
| 20 | TUI | `crates/runie-tui/src/popups/permission.rs:19-24` | Tool input rendered as raw pretty JSON | Pre-format a human-readable summary | `format-permission-request-for-human-readable-display` |
| 21 | TUI | `crates/runie-core/src/input_history.rs:72-106` | Input history JSONL unbounded, no lock, corruptible under concurrent processes | `fs2` advisory lock + cap + ring buffer (or move to SQLite) | `cap-and-lock-input-history` |
| 22 | TUI | `crates/runie-tui/src/terminal/caps/detect.rs:93-182` | Brand/multiplexer lookup tables remain after simplification task | Rely on `supports-color`/`supports-hyperlinks` + runtime probes | `drop-terminal-brand-and-multiplexer-tables` |
| 23 | TUI | `crates/runie-tui/src/quantize.rs:72-88` | Hand-maintained `ANSI256_TO_16` lookup table | Compute from `ansi_colours::ansi256_from_rgb` + Euclidean distance to 16 basics | `replace-ansi256-lookup-table-with-ansi-colours` |
| 24 | Core | `crates/runie-core/src/model_catalog/mod.rs:145-158` | Model catalog search is plain substring | `nucleo-matcher` or `sublime_fuzzy` | `use-fuzzy-matcher-for-model-catalog` |

## Agents Survey Takeaways

Patterns from `~/Code/agents/*` that were translated into tasks above:

- **Goose provider trait** — metadata, retry config, fast-model fallback, and typed errors (#12, #13).
- **Codex client/session split** — `ModelClient` vs `ModelClientSession` (#14).
- **Codex replay tests** — assert on outbound request bodies, not just inbound events. Added as a recommendation inside `replace-mock-provider-keyword-heuristics-with-fixtures` rather than a separate task.
- **Goose + gptme layered config** — project-level overrides + keyring secrets (#15).
- **Aider + Goose model metadata** — remote cacheable model registry. Captured as `fetch-remote-model-metadata-with-cache` but not created as a separate task in this pass (can be added when provider registry work begins).
- **MCP/rmcp** — already tracked as `implement-or-remove-mcp-runtime-scaffolding` / `unify-tool-registry-and-dispatch`; survey reinforced priority.
- **Codex configurable retry policy** — captured inside #12/#13.
- **CLI introspection helpers** (`--print-config`, `--print-env`, completions) — captured as a recommendation for `use-clap-derive-for-cli` rather than a separate task.

## Already Planned / Partial (not repeated as new tasks)

These findings confirm existing tasks but do not spawn duplicates:

- `use-toml-edit-and-file-locks-for-config`
- `make-trust-and-auth-file-persistence-atomic-and-restricted`
- `replace-agent-manifest-sha-with-include-dir-fingerprint`
- `replace-verify-tests-with-cargo-nextest`
- `replace-layered-config-merge-with-figment`
- `unify-diff-generation-and-parsing-on-one-crate`
- `replace-token-chars4-heuristic-with-tiktoken-rs`
- `merge-agentstate-into-turnstate-projection` / `deduplicate-turn-state-between-turnactor-and-appstate`
- `collapse-command-types-to-single-command-action-enum`
- `offload-agent-turn-from-actor-handler-to-tokio-task` / `route-agent-abort-through-actor-message-not-event-bus`
- `unify-permission-engines-into-ruleset`
- `move-approval-registry-into-permission-actor-state`
- `unify-headless-and-tui-streaming-response-parser` / `extract-shared-streaming-response-parser`
- `replace-mock-provider-keyword-heuristics-with-fixtures`
- `migrate-session-persistence-to-rusqlite`
- `derive-durable-and-headless-events-from-canonical-event`
- `drop-custom-markdown-ast-for-pulldown-cmark-and-tui-markdown`
- `replace-rpcreply-with-ractor-rpcreplyport`
- `spike-evaluate-async-openai-or-rig-core-for-provider-stack`
- `unify-provider-retry-on-backon-including-streams`
- `remove-modelfromprovider-api-key-field-and-use-keyring`
- `replace-bash-safety-heuristic-with-os-sandboxing`
- `replace-keymap-modifier-tables-with-crokey-map`
- `unify-tui-message-wrapping-with-textwrap`
- `simplify-terminal-capability-detection`
- `centralize-env-lock-in-runie-testing` (partial)

## Risks & Preconditions

- **TUI design freeze:** Any widget replacement must reproduce current `TestBackend` snapshots; tasks include Layer-3 acceptance criteria.
- **Provider trait changes** are breaking for all providers and tests; should be batched with the async-openai spike.
- **Config/dotenv changes** interact with the partial `centralize-env-lock-in-runie-testing` task; coordinate before implementation.
- **Lockfile deduplication** may require upgrading transitive deps; validate with `cargo test --workspace` after each change.

## New Tasks Created

See `tasks/` for the 25 new task files with unit/e2e/live-tmux acceptance criteria.
