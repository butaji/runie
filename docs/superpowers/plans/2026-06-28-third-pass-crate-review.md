# Third-Pass Crate Review — Five More Rounds of Findings

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement the linked tasks. No code changes in this document itself.

**Goal:** Surface the next layer of Pareto simplifications after the second-pass cleanup plan. Focus on provider/config/auth, message/session/state, dispatch/commands/permissions, TUI/widgets, and testing/build/DSL.

**Architecture:** Keep deleting custom code that mirrors standard crates. Push config/auth/provider plumbing toward `RactorConfigActor` and standard crates (`keyring`, `dotenvy`, `jsonschema`, `backon`, `clap`). Push TUI widgets toward the ratatui ecosystem (`tui-textarea`, `tui-input`, `ansi_colours`). Delete the dead `runie-macros` crate and the custom build linter.

**Tech Stack:** `backon`, `keyring`, `jsonschema`, `clap`, `dotenvy`, `shell-words`, `tui-textarea`, `tui-input`, `ansi_colours`, `notify`, `tracing`, `strum`, `pulldown-cmark`.

---

## How this review was done

- **No code was run.** Five `explore` subagents reviewed `provider/config/auth`, `message/session/state`, `dispatch/commands/permissions`, `tui/rendering/macros`, and `testing/build/dsl`.
- **ctx7** was consulted for `backon`, `jsonschema`, `keyring`, `tui-input`, and `dotenvy`.
- Peer Rust agent projects in `~/Code/agents/` (`goose`, `jcode`, `openfang`, `thClaws`) were used as precedent for crate choices.

---

## Round 1 — Provider / config / auth

| Finding | Files | Replacement | Priority | Est. LOC reduction |
|---|---|---|---|---|
| Custom retry module while `backon` is in workspace deps | `runie-provider/src/retry.rs` | `backon::Retryable` or `reqwest-retry` | P0 | -180 |
| XOR-obfuscated token vault | `runie-core/src/auth.rs` | `keyring` + headless fallback | P0 | -120 |
| Hand-written config validator mirrors schema | `runie-core/src/config/validate.rs` | `jsonschema` against `schemars` output | P0 | -300 |
| Provider credential resolution duplicated + custom `.env` parser | `runie-provider/src/config/mod.rs`, `runie-core/src/config/provider_config.rs` | `dotenvy` + single `ProviderConfig` impl | P1 | -80 |
| Two provider-config persistence helpers | `runie-core/src/provider/config.rs`, `actors/config/file_helpers.rs` | Route through `RactorConfigActor` or single helper | P1 | -150 |
| Custom layered-config merger | `runie-core/src/config/layers.rs` | `config` crate (optional, lower priority) | P2 | -40 |
| API keys as plain `String` | `runie-core/src/config/mod.rs`, `auth.rs` | `zeroize::Zeroizing`/`secrecy` | P2 | minimal |

Key peer lesson: `goose` uses `keyring`, `dotenvy`, `etcetera`; `thClaws` stores secrets in the OS keychain with `.env` fallback.

---

## Round 2 — Message / session / state

The biggest wins here were already tracked after earlier reviews, but the third pass confirmed they remain the largest code-reduction opportunities:

| Finding | Replacement | Priority |
|---|---|---|
| `ActorHandles` 400-line façade over `ractor` | Typed map of `ractor::ActorRef<Msg>` | P0 (tracked) |
| Actors storing mutable state in `Mutex<Self>` instead of `ractor::State` | Move state into `type State = MyState` | P0 |
| Custom `RpcReply` / broken `RactorHandle::rpc` | `ractor::RpcReplyPort` and `call!`/`call_t!` | P1 |
| Manual `Event`/`Intent`/`EventKind`/`EventCategory` mirrors | Attribute generator + `strum` | P0/P1 (tracked) |
| Custom file watcher thread bridge in `RactorConfigActor` | `notify` debouncer with actor `cast` closure | P1 |
| `runie-macros` dead crate with string-based parsing | Delete; use `strum`/`syn` if generator needed | P1 |

---

## Round 3 — Dispatch / commands / permissions

| Finding | Files | Replacement | Priority | Est. LOC reduction |
|---|---|---|---|---|
| Hand-rolled CLI argument parsing | `runie-cli/src/main.rs` | `clap` derive macros | P0 | -60 |
| Slash-command DSL has two representations (`CommandSpec` + `CommandDef`) | `runie-core/src/commands/dsl/*` | Single representation (derive from `clap` or one declarative table) | P1 | -200 |
| Permission system has two parallel rule engines | `runie-core/src/permissions/*` | Unified declarative ruleset | P1 | -250 |
| `bash_safety.rs` hand-rolled heuristic | `runie-core/src/bash_safety.rs` | `shell-words` + static deny-list | P2 | -180 |

---

## Round 4 — TUI / rendering / macros

| Finding | Files | Replacement | Priority | Est. LOC reduction |
|---|---|---|---|---|
| Custom `Stylize` trait | `runie-tui/src/stylize.rs` | `ratatui::style::Stylize` | P0 | -80 |
| Hand-rolled multi-line input box | `runie-tui/src/ui/input.rs` | `tui-textarea` / `tui-input` | P0 | -200 |
| Hand-rolled popup list | `runie-tui/src/popups/panel/list.rs` | `ratatui::widgets::List` | P0 | -150 |
| Hand-written terminal escape sequences | `runie-tui/src/terminal_setup.rs`, `terminal/clipboard.rs` | `crossterm::execute!` commands | P1 | -150 |
| Custom ANSI 256 color quantization | `runie-tui/src/quantize.rs` | `ansi_colours::ansi256_from_rgb` | P1 | -150 |
| Dead `runie-macros` crate | `crates/runie-macros/` | Delete | P1 | -552 (entire crate) |
| Markdown render wrappers around `tui-markdown` | `runie-tui/src/markdown_render.rs`, `message/wrap.rs` | Render `tui-markdown` `Text` directly + `textwrap` | P2 | -150 |

---

## Round 5 — Testing / build / DSL

| Finding | Files | Replacement | Priority | Est. LOC reduction |
|---|---|---|---|---|
| Duplicated MiniMax fixtures and mock helpers | `runie-agent/tests/minimax_turn.rs`, `runie-provider/tests/minimax_replay.rs`, `runie-agent/src/tests/turn.rs` | Move to `runie-testing` | P1 | -80–120 |
| Custom `nanoid()` in test runner | `runie-testing/src/runner.rs` | `uuid::Uuid::new_v4()` | P1 | -15 |
| Custom build.rs linter | `runie-core/build.rs`, `runie-core/src/build_lint.rs` | `[workspace.lints.clippy]` + CI file-limit script | P2 | -554 |
| Custom frontmatter scanner | `runie-core/src/resource_loader.rs` | `pulldown-cmark-frontmatter` + `serde_yaml` | P1 (tracked) |
| Telemetry in-memory collector | `runie-core/src/telemetry.rs` | `tracing` + subscriber layer | P2 (tracked) |

---

## New tasks added

- `tasks/replace-custom-retry-with-backon.md`
- `tasks/replace-xor-auth-with-keyring.md`
- `tasks/replace-config-validator-with-jsonschema.md`
- `tasks/use-clap-derive-for-cli.md`
- `tasks/replace-custom-tui-widgets-with-ratatui-ecosystem.md`
- `tasks/delete-dead-runie-macros-crate.md`
- `tasks/centralize-test-fixtures-and-mocks.md`
- `tasks/unify-provider-credential-resolution-with-dotenvy.md`
- `tasks/replace-bash-safety-with-shell-words.md`
- `tasks/replace-build-linter-with-clippy-ci.md`
- `tasks/use-notify-directly-in-config-actor.md`
- `tasks/unify-provider-config-persistence.md`
- `tasks/simplify-slash-command-dsl.md`
- `tasks/unify-permission-system-rules.md`

---

## Execution order

1. Land the actor runtime/bootstrap sequence (existing tasks).
2. Land the event taxonomy + `strum` tasks (existing).
3. Land P0 provider/config/auth tasks: `replace-custom-retry-with-backon`, `replace-xor-auth-with-keyring`, `replace-config-validator-with-jsonschema`, `use-clap-derive-for-cli`.
4. Land P0 TUI widget task: `replace-custom-tui-widgets-with-ratatui-ecosystem`.
5. Land P1 tasks: `unify-provider-credential-resolution-with-dotenvy`, `use-notify-directly-in-config-actor`, `unify-provider-config-persistence`, `unify-permission-system-rules`, `simplify-slash-command-dsl`, `centralize-test-fixtures-and-mocks`, `delete-dead-runie-macros-crate`.
6. Land P2 tasks: `replace-bash-safety-with-shell-words`, `replace-build-linter-with-clippy-ci`.
7. Final sweep: `cleanup-small-duplicates-and-dead-code`.

---

## Rejected alternatives

- **Keep the custom build linter.** The maintenance cost of a hand-rolled Rust tokenizer in `build.rs` exceeds the precision benefit of Clippy lints.
- **Keep the XOR auth cipher.** It is obfuscation, not encryption; `keyring` is the standard cross-platform solution.
- **Keep manual CLI parsing.** `clap` is already a dependency and is the standard for Rust CLIs.
- **Keep the dead `runie-macros` crate.** Unused proc-macro crates add compile time and brittle string-based parsing.
