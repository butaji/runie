# Fifth-Pass Architecture & Code Review — 2026-06-28

This document captures the findings from the fifth and final five-round architecture and code review. The goal remains: less code, Pareto (80/20) choices, unification, and replacing custom code with mature crates or OS features.

The fifth pass concentrated on five cross-cutting areas:

1. **Build system, CI, test harness, and dependencies**
2. **Error handling, logging, tracing, and telemetry**
3. **Protocol/IPC leftovers, serialization, and runtime wiring**
4. **Declarative resource loaders and DSLs**
5. **Macros, code generation, and small parser cleanups**

It builds on the four earlier passes recorded in [`2026-06-28-runie-cleanup-roadmap.md`](2026-06-28-runie-cleanup-roadmap.md), [`2026-06-28-less-code-crate-replacements.md`](2026-06-28-less-code-crate-replacements.md), [`2026-06-28-third-pass-crate-review.md`](2026-06-28-third-pass-crate-review.md), and [`2026-06-28-fourth-pass-crate-review.md`](2026-06-28-fourth-pass-crate-review.md).

---

## 1. Build system, CI, test harness, and dependencies

### Findings

- The `schema` feature is used in CI and the `justfile`, but `runie-core` does not declare it (`schemars` is a normal dependency).
- The `mcp` feature in `runie-core` is declared as `mcp = []` but the module is unconditionally compiled.
- `futures` is declared in `runie-cli` but unused.
- `tempfile` appears as both a normal and dev-dependency in `runie-core`.
- `strum`, `unicode-segmentation`, `notify`, `notify-debouncer-mini`, and `tracing` are pinned inline in `runie-core` despite being workspace concerns.
- `Cargo.lock` contains multiple versions of `notify`, `quick-xml`, `similar`, `strum`, `thiserror`, and others.
- The custom `build.rs` linter re-parses every Rust file on every build, has a bypass env var (`RUNIE_SKIP_BUILD_CHECKS`), and duplicates `scripts/check-field-access.sh`.
- `bacon.toml`'s `test` job actually runs the TUI binary, and `just lint-fix` passes contradictory Clippy flags.
- `scripts/verify-tests.sh` hardcodes an expected test count.
- `scripts/tmux-test.sh` is a shell/tmux test with `sleep`, violating AGENTS.md.
- `tokio::time::sleep` remains in `crates/runie-core/src/actors/session/tests.rs`.
- `runie-testing` is good but env-lock / temp-config patterns are duplicated per crate.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Broken `schema` feature in CI/justfile | Remove the flag | `fix-broken-schema-feature-in-ci-and-justfile` |
| Dead `mcp` feature | Gate module or delete feature | `delete-or-fix-dead-mcp-feature-flag` |
| Unused/duplicated/normalized deps | Workspace inheritance + removal | `remove-unused-dependencies-and-normalize-workspace-deps` |
| Custom `build.rs` linter | Clippy + CI | `replace-build-linter-with-clippy-ci` |
| Duplicate transitive deps | `cargo-deny` + targeted upgrades | `introduce-cargo-deny-and-cargo-machete-ci` |
| Broken bacon/just recipes | Fix recipes | `fix-dev-automation-recipes` |
| `scripts/tmux-test.sh` | Ratatui `TestBackend` tests | `replace-tmux-test-with-ratatui-tests` |
| `sleep()` in tests | Deterministic waits | `remove-sleep-from-automatic-tests` |
| Unused dep detection | `cargo-machete` CI | `introduce-cargo-deny-and-cargo-machete-ci` |

---

## 2. Error handling, logging, tracing, and telemetry

### Findings

- `anyhow` is used in library APIs; only `runie-testing` uses `thiserror`.
- Hand-written error types exist in `runie-protocol`, `provider_trait.rs`, `provider_event.rs`, `subagent.rs`, `sanitize.rs`, and `tool/types.rs`.
- No `tracing-subscriber` is initialized in either binary, so `tracing` events are dropped.
- `crates/runie-core/src/telemetry.rs` is an in-memory collector with no flush path and a dead panic hook.
- `println!`/`eprintln!` exist in library code (`event/headless.rs`, `keybindings/mod.rs`).
- Actors use `std::sync::Mutex::lock().unwrap()`, risking poisoning and restart loops.
- Several production `unwrap`/`expect` calls should be recoverable errors.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Hand-written errors + `anyhow` in libraries | `thiserror` | `unify-library-error-types-with-thiserror` |
| Dropped `tracing` events | `tracing-subscriber` in binaries | `initialize-tracing-subscriber-in-binaries` |
| Custom telemetry collector | `tracing` layer | `replace-custom-telemetry-with-tracing-layer` |
| Library `println!`/`eprintln!` | `tracing` events / writer APIs | `replace-custom-telemetry-with-tracing-layer` |
| `std::sync::Mutex` in actors | `tokio::sync::Mutex` / `parking_lot` | `harden-actors-against-mutex-poisoning` |
| Production `unwrap`/`expect` | Typed errors / fallbacks | `eliminate-production-unwrap-expect` |

---

## 3. Protocol/IPC leftovers, serialization, and runtime wiring

### Findings

- `runie-protocol` is still a full crate despite an archived task claiming it was folded into `runie-core`.
- `runie-cli` has two overlapping JSON-RPC servers (`server.rs` and `acp.rs`) with duplicated stdio loops.
- ACP event plumbing is broken: `AcpRuntime.event_tx` sends into a dropped receiver.
- `Leader` actor is unused by TUI/CLI and duplicates bootstrap logic.
- `RactorActor` wrapper and `ConfigHandle` trait are dead.
- `HeadlessEvent::to_json_line` has a hand-rolled JSON fallback.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| `runie-protocol` crate | Fold into `runie-core` | `fold-runie-protocol-into-core` |
| Two JSON-RPC server loops | Shared transport module | `unify-cli-json-rpc-transport-and-remove-dead-acp` |
| Broken ACP plumbing | Route to actor messages or delete ACP | `unify-cli-json-rpc-transport-and-remove-dead-acp` |
| Dead `Leader` actor | Adopt via existing tasks or delete | `expand-leader-start-for-tui-and-cli`, `migrate-tui-and-cli-to-leader-bootstrap` |
| Dead `RactorActor` / `ConfigHandle` | Delete | (fold into actor cleanup tasks) |
| Hand-rolled JSON fallback | Typed error event + `serde_json` | (small fix, can fold into protocol task) |

---

## 4. Declarative resource loaders and DSLs

### Findings

- `subagents/mod.rs` re-implements frontmatter/body scanning instead of using the unified `resource_loader.rs`.
- Command DSL has three overlapping representations (`CommandSpec`, runtime `CommandDef`, declarative `CommandDef`), two category enums, and a leaked global intent map.
- Permission system has two engines (`PermissionSet` and `PermissionManager`) with redundant `PermissionResult`/`PermissionAction`.
- `PermissionManager::new(_mode)` ignores its mode argument.
- MCP config scaffolding exists but no runtime client connects.
- No declarative role/persona loader exists.
- Several custom string parsers (`keybindings`, hooks, trigger lines, categories) could use `strum`.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| Subagent custom scanner | Shared resource loader | `migrate-subagent-loader-to-resource-loader` |
| Three command DSL structs | Single representation | `simplify-slash-command-dsl` |
| Two permission engines | Policy-chain ruleset | `unify-permission-system-rules` |
| MCP config without runtime | Implement `rmcp` client or delete | `implement-or-remove-mcp-runtime-scaffolding` |
| Custom string parsers | `strum` derives | `replace-remaining-custom-parsers-and-macros-with-strum` |

---

## 5. Macros, code generation, and small parser cleanups

### Findings

- `runie-macros` crate does not exist on disk, but the task tracking its deletion was still open.
- `ctor!`, `cmd!`, `with_panel_stack!`, `with_ordering!`, `style_fn!`, `theme_color!` macros exist.
- `Event`/`Intent` name tables and `EVENT_NAMES` are hand-maintained despite `strum` being available.
- `ThinkingLevel`, `CommandCategory`, `PermissionMode`, `PromptMode` implement `FromStr`/`Display` by hand.
- `dry_run.rs` has a hard-coded tool list that does not match `BUILTIN_TOOL_NAMES`.

### Recommended unification

| Custom code | Replacement | Task |
|-------------|-------------|------|
| `runie-macros` crate deletion | Mark done | `delete-dead-runie-macros-crate` |
| `ctor!` / name tables | `strum` derives | `use-strum-for-event-intent-names` |
| `cmd!`, `with_ordering!`, theme macros | Functions / helpers | `replace-remaining-custom-parsers-and-macros-with-strum` |
| Hand-written enum `FromStr`/`Display` | `strum` derives | `replace-remaining-custom-parsers-and-macros-with-strum` |
| Stale dry-run tool list | Canonical `BUILTIN_TOOL_NAMES` | `fix-dry-run-tool-names-discrepancy` |

---

## New task list

The fifth pass added the following tasks to `tasks/index.json`:

| Task | Priority | Area |
|------|----------|------|
| `fix-broken-schema-feature-in-ci-and-justfile` | P0 | Build / CI |
| `delete-or-fix-dead-mcp-feature-flag` | P0 | Dependencies |
| `unify-library-error-types-with-thiserror` | P0 | Architecture / Error Handling |
| `initialize-tracing-subscriber-in-binaries` | P0 | Observability |
| `fix-dry-run-tool-names-discrepancy` | P0 | Core / State |
| `fold-runie-protocol-into-core` | P1 | Architecture |
| `unify-cli-json-rpc-transport-and-remove-dead-acp` | P1 | CLI / IPC |
| `remove-unused-dependencies-and-normalize-workspace-deps` | P1 | Dependencies |
| `fix-dev-automation-recipes` | P1 | Dev automation |
| `replace-custom-telemetry-with-tracing-layer` | P1 | Observability |
| `harden-actors-against-mutex-poisoning` | P1 | Architecture / Actors |
| `migrate-subagent-loader-to-resource-loader` | P1 | Core / State |
| `replace-tmux-test-with-ratatui-tests` | P2 | Test harness |
| `remove-sleep-from-automatic-tests` | P2 | Test harness |
| `implement-or-remove-mcp-runtime-scaffolding` | P2 | Architecture / Tools |
| `replace-remaining-custom-parsers-and-macros-with-strum` | P2 | Core / State |
| `introduce-cargo-deny-and-cargo-machete-ci` | P2 | Build / CI |
| `eliminate-production-unwrap-expect` | P2 | Reliability |

Two tasks from earlier rounds were also reconciled:
- `delete-dead-runie-macros-crate` is now marked **done** because the crate no longer exists.
- `delete-dead-actor-modules-and-custom-trait` remains **done**.

Duplicate task files were removed:
- `tasks/deduplicate-turn-queue-delivery-logic.md` (kept `tasks/dedupe-turn-queue-delivery-logic.md`).
- `tasks/unify-session-store-and-index-with-rusqlite.md` (kept `tasks/unify-session-store-and-index.md`).

---

## Execution notes

- The P0 items are mostly correctness/CI fixes and should land early so the workspace builds cleanly.
- `unify-library-error-types-with-thiserror` unlocks `eliminate-production-unwrap-expect` and makes actor hardening easier.
- `initialize-tracing-subscriber-in-binaries` unlocks `replace-custom-telemetry-with-tracing-layer`.
- `fold-runie-protocol-into-core` unlocks `unify-cli-json-rpc-transport-and-remove-dead-acp`.
- `delete-or-fix-dead-mcp-feature-flag` should be resolved before implementing MCP runtime.
- Every task must include the 4-layer test coverage required by `AGENTS.md`.

## Cross-agent lessons

Peer codebases (`goose`, `jcode`, `OpenFang`, `thClaws`) reinforce the same choices:

- Use `thiserror` for library errors and `anyhow` only at binary boundaries.
- Use `tracing` + `tracing-subscriber` for observability; avoid custom telemetry collectors.
- Keep protocol types in the core crate; avoid thin protocol facade crates.
- Use standard CI tooling (`cargo-deny`, `cargo-machete`, Clippy) instead of custom build scripts.
- Replace shell/tmux tests with deterministic Rust tests.
