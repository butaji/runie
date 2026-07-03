# Sixth Five-Round Architecture & Code Review

## Status

Synthesis complete. Findings ranked and tracked as new `tasks/` files.

## Goal

Pareto (80/20) simplification: less code, fewer custom implementations, fewer duplicate dependencies, and clearer boundaries. If a standard crate, OS feature, or build-tool feature can replace custom code, plan it that way.

## Methodology

- **Five read-only subagents** reviewed disjoint slices (no code executed):
  1. Verification of recently `done` tasks for leftovers and stale artifacts.
  2. Deep-dive into the highest-risk remaining `todo` architecture work.
  3. Design of the Grok Build comparison harness and fixture pipeline.
  4. Remaining crate replacements in custom code.
  5. Developer experience, documentation, schema, and CLI simplifications.
- **ctx7** was consulted for: `derive_builder`, `figment`, `rusqlite`, `clap`, `async-openai`, `rmcp`.
- **~/Code/agents/* survey** focused on MCP cache/manager, session/thread-store traits, keyring abstraction, provider error tables, project-config denylist, and CLI early validation.

## Executive Summary

The codebase has advanced significantly since the fifth review: 205 tasks are now `done`, 81 `todo`, 5 `blocked`, 4 `wontfix`, 2 `partial`. This sixth pass therefore focused on **quality of done-task completions**, **concrete Grok Build harness design**, and **smaller remaining crate/DX cleanups** rather than re-stating the large architectural backlog.

Net-new task themes:

1. **Done-task cleanup** — delete stale aliases, fix leftover `DynProvider` naming, route `config=None` through the unified resolver, consolidate duplicate task files, fix docs/examples.
2. **Grok Build harness** — fixture loader/normalizer, provider-event translator, headless replay, TUI keystroke replay, tool-name mapping.
3. **Crate replacements** — `tempfile` for atomic writes, `derive_builder` for model/panel builders, `strum` for hook parsing, `ansi_colours` for ANSI256 mapping.
4. **DX / docs / schema** — `clap` derive for the TUI binary, binary-name alignment, fix `PermissionAction` serde case, sample config/env/completions, Architecture/EXECUTE doc audit.
5. **Agents-pattern imports** — MCP schema cache + connection manager, keyring store trait with mock, project-config denylist.

## P0 / P1 Findings

| # | Area | File(s) | Issue | Replacement / Plan | Task |
|---|------|---------|-------|--------------------|------|
| 1 | Done-task cleanup | `crates/runie-core/src/proto/provider.rs:24`, `crates/runie-provider/src/factory.rs`, `AGENTS.md:65` | `ProviderConfigBox` alias still exported; factory still named `DynProviderFactory`; docs example uses deleted `DynProvider` | Delete alias, rename factory, fix docs | `delete-providerconfigbox-alias-and-rename-dynproviderfactory` |
| 2 | Provider | `crates/runie-provider/src/lib.rs:54-80`, `crates/runie-core/src/auth/credential.rs:79-107` | `config=None` branch reads `std::env::var` directly, bypassing unified resolver | Route through `CredentialResolver` | `route-provider-confignone-through-credentialresolver` |
| 3 | Docs/CLI | `crates/runie-cli/Cargo.toml`, `README.md`, `docs/Architecture.md` | CLI binary is `runie-headless`; docs/scripts refer to `runie print`/`json`/`server` | Rename binaries or update all references | `align-cli-binary-name-with-docs` |
| 4 | Config/schema | `crates/runie-core/src/permissions/mod.rs`, `config.schema.json`, `docs/Configuration.md` | `PermissionAction` serializes PascalCase (`Allow`) but docs show lowercase (`allow`) | Add `#[serde(rename_all = "snake_case")]`, regen schema, fix docs | `fix-permissionaction-serde-case-and-docs` |
| 5 | TUI/CLI | `crates/runie-tui/src/main.rs`, `crates/runie-tui/src/dry_run_cmd.rs` | TUI flags parsed by hand; unknown flags silently ignored | `clap` derive `Cli` struct | `add-clap-derive-to-tui-binary` |
| 6 | Grok harness | `crates/runie-testing/src/fixtures/` | No Grok fixture loader or normalizer for non-deterministic IDs/timestamps/paths | Add `fixtures/grok_build.rs` with sanitizer | `create-grok-fixture-loader-and-normalizer` |
| 7 | Grok harness | `crates/runie-provider/src/openai/stream.rs`, `crates/runie-testing/src/replay_provider.rs` | Grok headless output is JSONL, not SSE; no translator to `ProviderEvent` | Build Grok JSONL → `ProviderEvent` translator + headless replay | `add-grok-provider-event-translator-and-headless-replay` |
| 8 | Grok harness | `crates/runie-tui/src/tests/mod.rs` | No bridge from captured Grok TUI pane/keystrokes to Runie `TestBackend` events | Keystroke DSL → `Event` translator + tool mapping | `add-grok-tui-keystroke-replay-and-tool-mapping` |
| 9 | Crate replacement | `crates/runie-core/src/persistence.rs:91-128` | Custom temp-name generator for atomic writes | `tempfile::NamedTempFile::new_in(parent)` | `replace-atomic-write-tempname-with-tempfile` |
| 10 | Crate replacement | `crates/runie-core/src/model_catalog/mod.rs:20-110`, `crates/runie-core/src/dialog/panel_split/builders.rs:7-269` | Hand-written consuming builders for `ModelCapabilities`/`ModelInfo` and `Panel` | `derive_builder` / `typed-builder` | `derive-builder-for-model-and-panel-builders` |
| 11 | Agents/MCP | `crates/runie-core/src/config/mcp.rs` | No MCP tool-schema cache or central connection manager | Cache keyed by config fingerprint + `McpConnectionManager` | `add-mcp-tool-schema-cache-and-connection-manager` |
| 12 | Agents/secrets | `crates/runie-core/src/auth/credential.rs` | Credential resolution couples directly to OS keyring; tests need global mocks | `KeyringStore` trait with real + mock backends | `add-keyring-store-trait-with-mock` |

## P2 Findings

| # | Area | File(s) | Issue | Replacement / Plan | Task |
|---|------|---------|-------|--------------------|------|
| 13 | Done-task cleanup | `crates/runie-core/src/proto/mod.rs:24`, `crates/runie-provider/src/config/mod.rs:79-80` | Stale `ProviderConfigBox` re-export; stale backward-compat comment/re-export | Remove re-exports and comments | `fix-agentsmd-layer4-example-and-stale-provider-reexports` |
| 14 | Task registry | `tasks/centralize-provider-credential-resolution.md`, `tasks/consolidate-provider-credential-resolver-duplication.md` | Two `done` task files describe nearly identical resolver work | Consolidate into one canonical file | `consolidate-duplicate-credential-resolution-task-files` |
| 15 | Done-task cleanup | `crates/runie-core/src/config/mod.rs:225`, `crates/runie-tui/src/main.rs:20` | `telemetry_tests` module and `as telemetry` alias left after rename | Rename module/import | `rename-telemetry-tests-and-import-alias` |
| 16 | Crate replacement | `crates/runie-core/src/hooks.rs:117-131` | Manual `match` mapping hook event names | `strum::EnumString` | `use-strum-for-hook-event-parsing` |
| 17 | Crate replacement | `crates/runie-tui/src/theme/loader.rs:152-186` | Hand-coded ANSI16/256 color tables | `ansi_colours::rgb_from_ansi256` | `use-ansicolours-for-theme-ansi256-mapping` |
| 18 | DX/docs | n/a | No sample config, env template, or shell completions | Add `docs/config.example.toml`, `.env.example`, `clap_complete` generator | `add-sample-config-env-and-completions` |
| 19 | DX/docs | `docs/Architecture.md`, `docs/DSL_Vision.md`, `EXECUTE.md` | Docs describe non-existent crates/modes/speculative actors | Audit and update; move speculative sections to Future/R4 | `audit-and-update-architecture-and-execute-docs` |
| 20 | Agents/config | `crates/runie-core/src/config/layers.rs` | No project-local denylist for dangerous config keys | Add denylist for `model_providers`, `openai_base_url`, `profile`, etc. | `add-project-config-denylist-for-sensitive-keys` |

## Already Planned / Partial (confirmed, not duplicated)

These findings confirm existing tasks but do not spawn duplicates:

- `deduplicate-turn-state-between-turnactor-and-appstate` / `merge-agentstate-into-turnstate-projection`
- `collapse-command-types-to-single-command-action-enum`
- `unify-permission-engines-into-ruleset`
- `migrate-session-persistence-to-rusqlite` / `consolidate-session-metadata-and-delete-sessionindex-read-path`
- `derive-durable-and-headless-events-from-canonical-event`
- `offload-agent-turn-from-actor-handler-to-tokio-task` / `route-agent-abort-through-actor-message-not-event-bus`
- `unify-tool-registry-and-dispatch`
- `drop-custom-markdown-ast-for-pulldown-cmark-and-tui-markdown`
- `unify-bash-execution-into-single-shell-module-with-command-group`
- `remove-direct-appstate-mutation-from-tui-handlers`
- `replace-verify-tests-with-cargo-nextest`
- `use-toml-edit-and-file-locks-for-config`
- `replace-layered-config-merge-with-figment`
- `make-trust-and-auth-file-persistence-atomic-and-restricted`
- `propagate-secretstring-through-provider-stack`
- `enrich-provider-trait-with-metadata-and-retry-config`
- `introduce-typed-provider-errors`
- `split-modelclient-and-turnsession`
- `add-project-level-config-layer-and-keyring-secrets`
- `compare-*` Grok tasks (now have concrete sub-tasks)

## Suggested Execution Order

1. **Docs/registry hygiene** — `align-cli-binary-name-with-docs`, `fix-permissionaction-serde-case-and-docs`, `audit-and-update-architecture-and-execute-docs`, `consolidate-duplicate-credential-resolution-task-files`.
2. **Done-task cleanup** — `delete-providerconfigbox-alias-and-rename-dynproviderfactory`, `route-provider-confignone-through-credentialresolver`, `fix-agentsmd-layer4-example-and-stale-provider-reexports`, `rename-telemetry-tests-and-import-alias`.
3. **Grok harness unblockers** — `create-grok-fixture-loader-and-normalizer`, `add-grok-provider-event-translator-and-headless-replay`, `add-grok-tui-keystroke-replay-and-tool-mapping`.
4. **Small crate replacements** — `replace-atomic-write-tempname-with-tempfile`, `use-strum-for-hook-event-parsing`, `use-ansicolours-for-theme-ansi256-mapping`.
5. **Larger crate refactor** — `derive-builder-for-model-and-panel-builders`, `add-clap-derive-to-tui-binary`.
6. **Agents-pattern imports** — `add-keyring-store-trait-with-mock`, `add-project-config-denylist-for-sensitive-keys`, `add-mcp-tool-schema-cache-and-connection-manager`.
7. **DX polish** — `add-sample-config-env-and-completions`.

## Risks & Preconditions

- **Binary rename** is a user-facing breaking change; prefer it only if docs/scripts are updated atomically.
- **`PermissionAction` case change** breaks existing user configs; decide whether to accept breakage or add a deserializer alias.
- **Grok harness** must never invoke live Grok in `cargo test`; keep recorder scripts separate and documented.
- **`derive_builder` adoption** is API-churn heavy for `Panel` builders; do after TUI snapshot baselines are stable.

## New Tasks Created

See `tasks/` for the 20 new task files with unit/e2e/live-tmux acceptance criteria.
