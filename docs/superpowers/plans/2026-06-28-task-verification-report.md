# Task Completion Verification Report

**Date:** 2026-06-28  
**Scope:** All tasks marked `done` in `tasks/index.json`, plus the small set marked active.  
**Method:** Read-only code inspection by five focused `explore` subagents, plus `cargo check --workspace` and `cargo test --workspace`.

## Baseline

- `cargo check --workspace`: **passed**
- `cargo test --workspace`: **715 passed, 0 failed, 3 ignored**

A green build/test run is necessary but not sufficient: many tasks were marked `done` while the corresponding code still contained the old behavior, only partially implemented the acceptance criteria, or left the AC checkboxes unchecked.

## Summary of findings

| Area | Tasks reviewed | Truly done | Partially done | Not done / reopen |
|------|----------------|------------|----------------|-------------------|
| Actor / bootstrap / lifecycle | 15 | 13 | 1 | 1 |
| Provider / config / auth | 14 | 9 | 1 | 4 |
| Session / loaders / DSL | 18 | 11 | 1 | 6 |
| TUI / markdown / input | 20 | 14 | 2 | 4 |
| Build / CI / errors / telemetry | 20 | 13 | 2 | 5 |
| **Total** | **87** | **60** | **7** | **20** |

*(The remaining 3 of the 90 tasks were already in non-done states: `wontfix` or active with notes.)*

## Status corrections applied

The following tasks were marked `done` but are **not done**; their status has been reset to `todo`:

1. `migrate-tui-and-cli-to-leader-bootstrap` — CLI does not use `Leader::start` at all.
2. `remove-sync-turn-queue-fallback-from-app-state` — sync test-mode branches remain in `AppState::deliver_queued`/`queue_follow_up`.
3. `add-jsonschema-validation-to-configactor-load` — `RactorConfigActor::pre_start`/`reload` never validates against `config.schema.json`.
4. `replace-config-validator-with-jsonschema` — `validate.rs` module still exists despite the hand-written validator being gone.
5. `route-cli-config-through-configactor` — only `inspect.rs` routes through the actor; other CLI paths do not.
6. `store-provider-api-keys-in-keyring-not-config` — `api_key` is still `String`, no migration moves plaintext keys to keyring.
7. `use-clap-derive-for-cli` — `config`/`mcp` subcommands from the AC are missing.
8. `centralize-built-in-tool-names` — tool names still hard-coded in `tool_runner.rs`, `turn/mod.rs`, tests.
9. `collapse-event-intent-kind-taxonomies` — `event/variants.rs` is 545 lines, violating the 500-line guardrail.
10. `remove-sleep-from-automatic-tests` — four `tokio::time::sleep` calls remain in `ractor_config.rs` tests.
11. `replace-custom-helpers-with-crates` — `path.rs` wrapper and `keybindings` test-only `parse_key_combo` remain.
12. `simplify-slash-command-dsl` — both `CommandSpec`/`CommandDef` still exist; leaked `INTENT_EVENTS` map remains.
13. `unify-permission-system-rules` — `PermissionSet` ruleset still separate from `PermissionManager` policy chain.
14. `use-strum-for-event-intent-names` — `event/name.rs` still relies on generated `EVENT_NAMES` table.
15. `extract-shared-tracing-subscriber-init` — shared init exists but the AC test is missing.
16. `initialize-tracing-subscriber-in-binaries` — subscriber exists but the AC test is missing.
17. `replace-custom-tui-widgets-with-ratatui-ecosystem` — input/popup/form widgets remain custom; `tui-textarea` unused.
18. `unify-core-and-tui-line-count-computation` — `render_lines.rs` still computes line count independently.
19. `cleanup-small-duplicates-and-dead-code` — 27 `FIXME: Audit environment access` comments and undocumented `#[allow(dead_code)]` remain.
20. `eliminate-production-unwrap-expect` — theme-loader `expect`s remain in production code.
21. `introduce-cargo-deny-and-cargo-machete-ci` — claims about removed deps are stale; `cargo machete` results need reconciliation.
22. `narrow-runie-core-public-api` — `runie-core/src/lib.rs` still exports many internal modules publicly.
23. `replace-build-linter-with-clippy-ci` — workspace lints are not inherited (`[lints] workspace = true` missing); `build.rs` is 854 lines.
24. `use-notify-directly-in-config-actor` — still spawns a thread + tokio mpsc instead of calling `actor_ref.cast` from a debouncer closure.

The following tasks were marked `done` but are **partially done**; their status has been reset to `todo` because the remaining work is non-trivial:

- `remove-sync-turn-queue-fallback-from-app-state`
- `replace-config-validator-with-jsonschema`
- `route-cli-config-through-configactor`
- `store-provider-api-keys-in-keyring-not-config`
- `use-clap-derive-for-cli`
- `centralize-built-in-tool-names`
- `collapse-event-intent-kind-taxonomies`
- `replace-custom-helpers-with-crates`
- `unify-permission-system-rules`
- `use-strum-for-event-intent-names`
- `extract-shared-tracing-subscriber-init`
- `initialize-tracing-subscriber-in-binaries`
- `replace-custom-tui-widgets-with-ratatui-ecosystem`
- `unify-core-and-tui-line-count-computation`
- `cleanup-small-duplicates-and-dead-code`
- `eliminate-production-unwrap-expect`
- `introduce-cargo-deny-and-cargo-machete-ci`
- `narrow-runie-core-public-api`
- `replace-build-linter-with-clippy-ci`

*(Note: for simplicity in `tasks/index.json`, partial and not-done tasks are both stored as `todo`; the nuance is recorded in this report and in updated task-file notes.)*

The following task was already marked `wontfix` and remains so:

- `use-pulldown-cmark-for-tool-marker-stripping` — tool-marker stripping intentionally remains string-based.

## Key remaining risks

1. **CLI is not on the leader bootstrap.** `migrate-tui-and-cli-to-leader-bootstrap` being not-done means the CLI and TUI have divergent actor lifecycles.
2. **Permission actor was auto-denying everything until recently.** The verifier found the reply lifecycle fix in place, but the fact it was marked done before the fix landed suggests task-status discipline needs tightening.
3. **Config validation is not enforced at load time.** Invalid configs can still be accepted silently.
4. **Provider API keys are still plaintext.** Security task is incomplete.
5. **500-line file guardrail is violated.** `event/variants.rs` (545 lines) and `runie-core/build.rs` (854 lines) exceed the documented limit.
6. **Workspace Clippy lints are not inherited.** The `[workspace.lints.clippy]` block has no effect until each crate opts in.

## Recommendations

1. Adopt a stricter definition of "done": every AC checkbox must be checked, the relevant code must be present, and the workspace must still pass `cargo test`.
2. For tasks that are intentionally scoped down, update the AC and status to `partial` or `wontfix` rather than marking `done`.
3. Before the next large planning pass, reconcile `tasks/index.json` with the actual file statuses (this report does that).
4. Prioritize the reopened P0/P1 tasks, especially the CLI leader bootstrap, config validation, and keyring storage.

## Files touched by this verification

- `tasks/index.json` — regenerated from actual task-file statuses.
- `docs/superpowers/plans/2026-06-28-task-verification-report.md` — this report.
- `docs/Architecture.md` and `docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md` — active-task count updated.
- Individual task files whose statuses were corrected (24 files).
