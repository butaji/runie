# Eighth Five-Round Architecture & Code Review

## Status

Synthesis complete. Findings ranked and tracked as new `tasks/` files. Backlog status synced.

## Goal

Pareto (80/20) simplification: less code, fewer custom implementations, fewer duplicate dependencies, clearer boundaries, and a trustworthy backlog. If a standard crate, OS feature, or build-tool feature can replace custom code, plan it that way.

## Methodology

- **Five read-only subagents** reviewed disjoint slices (no code executed):
  1. Backlog dependency/execution-order analysis of the remaining 68 `todo` + 5 `blocked` tasks.
  2. Post-implementation consistency check after the recent batch of landings.
  3. Remaining crate/OS replacement opportunities.
  4. Grok Build comparison unblockers.
  5. Config/schema/CLI/docs alignment.
- **~/Code/agents/* survey** focused on: session resumption/branching, context-window compaction, plan mode, MCP lifecycle/schema cache, model fast fallback, subagent delegation.
- **ctx7** was consulted for: `dashmap`, `indexmap`, `validator`, `serde_with`, `textwrap`, `tui-scrollview`.

## Executive Summary

This eighth pass shifted focus from "more tasks" to **high-confidence, concrete bugs and alignment issues** surfaced by the recent implementation batch. The biggest findings:

1. **Backlog drift** — `typed-chatmessage-builder-and-shrink-sanitize.md` is fully implemented but still listed as `todo`.
2. **Two P0 regressions/bugs** introduced by recent commits:
   - Agent actor never clears `current_turn_token`/`handle` after a successful turn, so every subsequent `Run` is rejected.
   - `atomic_write` drops the advisory lock *before* the final `rename`, defeating atomicity under concurrency.
3. **Session persistence gaps** — tree snapshots are not round-tripped through durable events; header updates are not atomic/locked.
4. **Dead code** — `RactorAgentHandleExt`, `InspectReport::build()`, vacuous `print` test.
5. **Crate replacement candidates** — `dashmap` for approval registry, `camino` for internal paths, `indexmap` for stable ordering, `throbber-widgets-tui`, `tui-popup`, `tui-scrollview`, `textwrap`, `validator`, `tinytemplate`, `serde_with`.
6. **Doc/schema/CLI misalignments** — `provider_type` vs `type`, missing permissions/env/keyring docs, wrong `justfile` binary name, TUI clap name, README examples, Architecture.md MCP example, AGENTS.md Layer-4 example.
7. **Agent-pattern imports** — session compaction, structural truncation, plan file artifacts, Codex-style MCP manager.

## Keystone Sequencing Recommendation

The subagent dependency analysis identified a single minimal milestone that unblocks the most downstream work:

**"State + Runtime Foundation" milestone**
1. Backlog hygiene (regenerate `index.json`, mark done tasks correctly).
2. Fix the two P0 bugs above.
3. Finish `merge-agentstate-into-turnstate-projection` (delete remaining sync glue).
4. Land async/shutdown reliability fixes (`await-provider-network-calls`, `fix-deliverqueued-race`, `route-agent-abort`, `store-and-await-headlessruntime`, `graceful-leader-shutdown`, `make-tui-render-loop-async`).
5. Prepare Grok Build reference → build comparison harness → record fixtures.

Only after this milestone should the large config/session/MCP refactors begin.

## Backlog Hygiene Findings

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 1 | P0 | `typed-chatmessage-builder-and-shrink-sanitize.md` is implemented (builder exists, `sanitize.rs` shrunk from 333 to 134 LOC) but still `todo` | Mark file `done` and regenerate `index.json` |
| 2 | P1 | `merge-agentstate-into-turnstate-projection.md` structural half is done; remaining sync glue in `turn_projections.rs:199-349` is the real remaining work | Update task description; create follow-up task for sync-glue deletion |
| 3 | P1 | Several `todo` tasks claim to `blocks` other tasks that are already `done` | Create cleanup task to remove stale `blocks` metadata |
| 4 | P1 | Config/secrets tasks (`figment`, `toml-edit`, project config, keyring, SecretString, denylist) will conflict if executed out of order | Document execution order in this review |
| 5 | P1 | Session persistence comparison depends on durable events + rusqlite migration being stable first | Document execution order |

## P0 / P1 Code Findings

| # | Area | File(s) | Issue | Plan | Task |
|---|------|---------|-------|------|------|
| 6 | Agent | `runie-agent/src/actor.rs:123-187` | Spawned turn task never clears `current_turn_token`/`handle` on success; subsequent `Run` rejected | Send `TurnFinished` message or let task clear state | `clear-inflight-turn-state-on-success` |
| 7 | IO | `runie-core/src/io/atomic_write.rs:28-49` | Advisory lock dropped before `rename`, leaving race window | Hold lock until after `rename` | `hold-advisory-lock-until-atomic-rename-completes` |
| 8 | Session | `runie-core/src/event/durable.rs:367-370` | `DurableCoreEvent::TreeSnapshot` added but never produced or round-tripped | Add `Event` variant and conversions | `round-trip-session-tree-snapshot-through-durable-events` |
| 9 | Session | `runie-core/src/session/store.rs:203-220`, `header.rs:39-47` | Header rewrite is unlocked and not atomic | Use `exclusive_lock` + `atomic_write` | `use-atomic-write-and-lock-for-session-header-updates` |
| 10 | State/tests | `runie-core/src/model/state/turn_projections.rs:7-196` | Tests directly mutate `AgentState` | Rewrite to seed `TurnState` and assert projection | `rewrite-turn-projections-tests-to-not-mutate-agentstate` |
| 11 | Trust/tests | `runie-core/src/trust.rs:113-138` | `save_load_roundtrip` bypasses `TrustManager::save()` atomic path | Use `tm.save()` and assert `0o600` perms | `test-trust-save-through-atomic-write-path` |
| 12 | Provider/Keyring | `runie-provider/src/lib.rs:62-80` | `BuiltProviderFactory` cannot inject `KeyringStore`; tests may hit OS keyring | Add optional `Arc<dyn KeyringStore>` parameter | `plumb-keyringstore-into-builtproviderfactory` |
| 13 | Diff/comparison | n/a | No TUI approve/reject flow; comparison task assumes full parity | Scope to headless unified diff output | `scope-diff-comparison-to-headless-unified-output` |

## P2 Crate Replacement Findings

| # | Area | File(s) | Issue | Replacement | Task |
|---|------|---------|-------|-------------|------|
| 14 | Concurrency | `permissions/approval_registry.rs:15-19` | `Mutex<HashMap>` registry | `dashmap::DashMap` | `use-dashmap-for-approval-registry` |
| 15 | Paths | `path.rs`, `path_complete.rs`, `tool/*.rs`, `trust.rs`, etc. | `PathBuf` + lossy conversions | `camino::Utf8PathBuf` for internal paths | `adopt-camino-utf8pathbuf-for-internal-paths` |
| 16 | Collections | `config/mod.rs`, `trust.rs`, `resource_loader.rs`, `subagents/mod.rs` | `HashMap` with unstable order | `indexmap::IndexMap` where user-visible | `use-indexmap-for-config-trust-and-subagent-maps` |
| 17 | TUI | `status_bar.rs`, `theme/glyph.rs` | Hand-rolled braille spinner | `throbber-widgets-tui` | `use-throbber-widgets-tui-for-spinner` |
| 18 | TUI | `popups.rs`, `popups/panel/`, `popups/permission.rs`, `popups/welcome.rs` | Manual popup rectangles/borders | `tui-popup` | `use-tui-popup-for-popup-shell` |
| 19 | TUI | `ui/scroll.rs`, `ui/messages/`, `message/wrap.rs`, `message/support.rs` | Hand-rolled scrollable feed + line-count math | `tui-scrollview` | `use-tui-scrollview-for-message-feed` |
| 20 | Text | `tool/format.rs`, `message/support.rs` | Manual truncation/wrapping | `textwrap` | `use-textwrap-for-truncate-args-and-blockquote-wrap` |
| 21 | Validation | `update/dialog/form/`, `login_flow/state/`, `config/schema.rs` | Ad-hoc form/config validation | `validator` derive | `use-validator-for-form-and-config-inputs` |
| 22 | Templating | `subagents/mod.rs:67-74` | Custom `{{var}}` replacement | `tinytemplate` | `replace-subagent-template-engine-with-tinytemplate` |
| 23 | Serialization | `provider_event.rs:135-202` | Manual `Serialize`/`Deserialize` for `ModelError` | `serde_with` | `use-serde_with-for-provider-event-serde` |

## Doc / Schema / CLI Alignment Findings

| # | Severity | File(s) | Issue | Fix | Task |
|---|----------|---------|-------|-----|------|
| 24 | P1 | `docs/Configuration.md:28,32,37` | Uses `provider_type` instead of canonical `type` | Update to `type = "..."` | Part of `fix-config-docs-and-readme-misalignments` |
| 25 | P1 | `docs/Configuration.md:27-30`, `config.schema.json:217-241` | Anthropic example lacks `base_url` though schema requires it | Add `base_url` or make schema optional | Part of `fix-config-docs-and-readme-misalignments` |
| 26 | P1 | `docs/Configuration.md` | No permissions section | Add snake_case examples | Part of `fix-config-docs-and-readme-misalignments` |
| 27 | P2 | `docs/Configuration.md` | No env vars / `.env` / keyring section | Add sections | Part of `fix-config-docs-and-readme-misalignments` |
| 28 | P1 | `justfile:40` | `just tui` uses `--bin runie` instead of `runie-tui` | Fix recipe | Part of `fix-config-docs-and-readme-misalignments` |
| 29 | P2 | `crates/runie-tui/src/main.rs:33` | clap program name set to `runie` | Change to `runie-tui` | Part of `fix-config-docs-and-readme-misalignments` |
| 30 | P2 | `README.md:120` | `print` example redirects unread stdin | Fix example | Part of `fix-config-docs-and-readme-misalignments` |
| 31 | P2 | `README.md:133-141` | Modes table omits `inspect`/`mcp` | Add rows | Part of `fix-config-docs-and-readme-misalignments` |
| 32 | P1 | `docs/Architecture.md:282` | Uses unsupported `--transport stdio` flag | Remove flag | Part of `fix-config-docs-and-readme-misalignments` |
| 33 | P1 | `AGENTS.md:62-78` | Layer-4 example uses non-existent types/functions | Rewrite to compile | Part of `rewrite-agentsmd-layer4-example-and-remove-stale-checksum-row` |
| 34 | P2 | `AGENTS.md:135-137` | Claims build script enforces agent manifest checksums (it does not) | Remove row | Part of `rewrite-agentsmd-layer4-example-and-remove-stale-checksum-row` |
| 35 | P2 | `config.schema.json:372` | `PermissionMode` description lists camelCase values | Regenerate schema | Part of `regenerate-config-schema-and-fix-permission-mode-description` |

## Dead Code / Test Drift Findings

| # | Severity | File(s) | Issue | Fix | Task |
|---|----------|---------|-------|-----|------|
| 36 | P2 | `runie-agent/src/actor.rs:265-276`, `ui_actor_agent_handles.rs:23-68` | `RactorAgentHandleExt` exported but unused | Delete trait | `delete-dead-ractoragenthandleext-trait` |
| 37 | P2 | `runie-cli/src/inspect/mod.rs:113-128` | `InspectReport::build()` dead code | Delete method | `delete-dead-inspectreport-build-method` |
| 38 | P2 | `runie-cli/src/print.rs:29-35` | Vacuous `result.is_err() || result.is_ok()` test | Replace with replay test or delete | `replace-vacuous-print-mode-test-with-replay-test` |

## Agents Survey Takeaways

| # | Source | Pattern | Runie application |
|---|--------|---------|-------------------|
| 1 | Goose `session_manager.rs` | SQLite sessions DB with schema_version, migrations, `copy_session` fork | `migrate-session-persistence-to-rusqlite` |
| 2 | gptme `logmanager` | Branch JSONL files + compacted views + directory lock | Session tree branch isolation |
| 3 | Codex `Thread.ts` | `sessionId` + `forkedFromId` + `parentThreadId` | Unified session/subagent branching |
| 4 | Goose `context_mgmt` | Threshold compaction, progressive tool-response removal, async tool-pair summarization | `add-session-compaction-with-threshold-and-tool-pair-summarization` |
| 5 | gptme `reduce.py` | Structural truncation of codeblocks / `<details>` before dropping messages | `add-structural-message-truncation-for-context-window` |
| 6 | kimi-code `session.ts` | Plan persisted as markdown file; `setPlanMode` RPC event | `add-plan-file-artifact-and-plan-mode-rpc` |
| 7 | Codex `McpConnectionManager` | JoinSet startup, cancel tokens, startup events, `wait_for_server_ready` | `add-mcp-tool-schema-cache-and-connection-manager` |
| 8 | Codex `tools.rs` | `tools/list` cache keyed by config fingerprint, name normalization | MCP schema cache |
| 9 | Goose `Provider::complete_fast` | Fast model config + fallback | `enrich-provider-trait-with-metadata-and-retry-config` |
| 10 | Goose `subagent_handler.rs` | `SubagentRunParams` struct + session ID scoping + channel callbacks | `compare-subagent-mcp-support-and-fix-gaps` |

## Suggested Execution Order

1. **Backlog + docs hygiene** — mark `typed-chatmessage-builder...` done; regenerate `index.json`; fix config/docs/schema misalignments; rewrite AGENTS.md example.
2. **P0 bugs** — `clear-inflight-turn-state-on-success`, `hold-advisory-lock-until-atomic-rename-completes`.
3. **State SSOT** — finish `merge-agentstate-into-turnstate-projection` (sync glue); rewrite related tests.
4. **Session persistence substrate** — durable tree snapshot round-trip; atomic header updates; trust save test.
5. **Runtime reliability** — async/shutdown fixes from R7.
6. **Grok harness** — factory injection, testable TUI bootstrap, message origin, diff comparison scope.
7. **Crate replacements** — approval registry, camino, indexmap, TUI widgets, textwrap, validator, tinytemplate, serde_with.
8. **Agent-pattern imports** — session compaction, structural truncation, plan file artifact.

## Risks & Preconditions

- The two P0 bugs should be fixed before any release; they break multi-turn agent usage and concurrent file writes.
- `AgentState`/`TurnState` collapse is still the largest keystone; it blocks TUI/state cleanups and multi-turn comparisons.
- Config/session/MCP refactors must wait for the runtime foundation to stabilize.
- Grok Build binary remains externally blocked; keep the recorder as an optional manual step.

## New Tasks Created

See `tasks/` for 25 new task files with unit/e2e/live-tmux acceptance criteria.
