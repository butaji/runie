# Runie Architecture

Runie is a terminal-native harness for LLM-powered coding agents. It is not a chat website and not tied to one provider: it is a local control surface for models that can read, write, edit, search, and run shell commands inside your project.

This document describes the high-level architecture. The code and tests are written as small, declarative DSLs so that the details stay self-explaining.

## Layered architecture

Runie is split into three layers:

```text
┌─────────────────────────────────────────┐
│  UI layer (pure / MVU)                  │
│  - RenderActor: Snapshot → Frame        │
│  - UiActor: facts → Snapshot            │
│  - Input handlers: user action → intent │
├─────────────────────────────────────────┤
│  Domain layer (pure + actors)           │
│  - Actors own state and business rules  │
│  - Intents trigger actor work           │
│  - Facts broadcast state changes        │
├─────────────────────────────────────────┤
│  IO layer (async)                       │
│  - Files, network, subprocesses, OS     │
│  - Results arrive as events             │
└─────────────────────────────────────────┘
```

Rules:

- **IO is async and actor-owned.** Blocking or long-lived IO runs inside dedicated actors (`ConfigActor`, `SessionActor`, `FffIndexerActor`, `IoActor`, `EnvActor`). The rest of the app sees only events.
- **Actors are the single source of truth.** Each mutable state slice lives in exactly one actor. No handler, command, or dialog mutates state directly.
- **State synchronization is event-driven.** Handlers emit **intents** (requests). Actors consume intents, update their authoritative state, and publish **facts** (state changes). The UI layer projects facts into a read-only `Snapshot`/`AppState`.
- **The UI layer is pure.** Rendering is a pure function `draw(&mut Frame, &Snapshot)`. View logic is a pure projection of facts.
- **Complexity is hidden behind declarative DSLs.** Commands, keybindings, and dialog actions compose as small flows: `on(trigger).intent(...).then(...)`.

## Crate map

| Crate | Role |
|-------|------|
| `runie-core` | Events, `AppState`, sessions, config, commands, dialog DSL, harness skills |
| `runie-agent` | Agent turn loop, tool-call parsing, truncation, subagent runner, built-in tools |
| `runie-provider` | LLM provider clients and model catalog (OpenAI-compatible, Anthropic, MiniMax, …) |
| `runie-tui` | CLI entry, Ratatui rendering, panels/forms, theme, terminal setup |
| `runie-server` | RPC / server mode binary |
| `runie-protocol` | Folded into `runie-core` (`crates/runie-core/src/proto/`) |
| `runie-testing` | Test fixtures, mock providers, and harness helpers |
| `runie-macros` | Deleted; no longer a workspace member |

## Runtime

```text
         TUI client      Headless client     ACP/WS client
              │                  │                  │
              └──────────────────┼──────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │      LeaderActor        │  owns the event bus,
                    │  (session, plan, turn,  │  runtime lifecycle,
                    │   MCP, permissions)     │  and durable state
                    └─────────────────────────┘
                                 │
              Intent events      │      Facts
              ──────────────────►│◄──────────────────
                                 │
      ┌──────────┬───────────────┼───────────────┬──────────┐
      │          │               │               │          │
   Config    Session           Turn            Input      View
   Actor     Actor             Actor           Actor      Actor
      │          │               │               │          │
      └──────────┴───────────────┴───────────────┴──────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │  AppState/Snapshot (pure)
                    └─────────────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │     RenderActor (pure)  │  draw(&mut Frame, &Snapshot)
                    └─────────────────────────┘
```

The runtime is centered on a `LeaderActor` that owns the event bus and the long-lived actors. Clients (TUI, headless, ACP, WebSocket) are thin producers of intents and consumers of facts. They do not duplicate runtime logic. This makes it cheap to add new surfaces: a new client only needs to speak the intent/fact protocol.

Actors are plain `tokio` tasks. Each actor owns a slice of authoritative state and communicates through typed intents and facts. There is no central mutable `AppState`; `AppState` is a read-only projection updated by facts.

### Bootstrap and rendering rules

- All startup file I/O (git detection, trust, skills, auth tokens, theme, config) runs inside dedicated actors before the UI event loop starts.
- `AppState` is built by applying facts. It never reads files, blocks, or mutates itself outside the projection path.
- The render path is a pure function `draw(&mut Frame, &Snapshot)`. It never mutates state.
- `InputActor` reads crossterm events and publishes typed input events. Handlers turn those into intents.
- `TurnActor` owns the LLM turn lifecycle, scheduling queues, and token accounting. It consumes streaming events from `AgentActor`/`ProviderActor` and emits session/tool facts.
- `SessionActor` owns the in-memory session and persists durable events append-only.
- `ViewActor` owns derived view/cache state and invalidates the render path.
- `ConfigActor` is the single owner of `~/.runie/config.toml`; it loads, saves, and publishes `ConfigLoaded` facts.
- `ProviderActor` is the single owner of `DynProvider` construction and API-key validation; it resolves credentials through the config actor.

## Core concepts

### Events

`CoreEvent` is the single vocabulary for state transitions. Events are immutable and split into two families:

- **Intents** — fire-and-forget requests to an actor. They describe *what the user or system wants*. Examples: `SetTheme`, `SubmitInput`, `AskPermission`, `RunTurn`.
- **Facts** — broadcast state changes produced by actors. They describe *what changed*. Examples: `ConfigLoaded`, `SessionChanged`, `TurnProgress`, `PermissionResolved`.

Handlers, commands, and keybindings emit intents. Actors consume intents, update their authoritative state, and emit facts. The UI layer projects facts into a read-only `Snapshot`. Durable facts are persisted to the session store; transient facts are UI-only.

### Sessions

A session is a persisted sequence of durable events. Loading a session replays those events into the actors. Branches and forks are represented by replaying up to a point and then continuing from there.

### Commands

Slash commands (`/model`, `/save`, `/compact`, …) are registered in a typed `CommandRegistry`. Each command defines a form or direct handler and emits events.

### Tool model (MCP-first)

All tools are exposed through the Model Context Protocol (MCP). Tool input schemas are derived from Rust structs via `schemars`; execution is handled by the MCP runtime. The legacy `Tool` trait, `ToolRegistry`, and text/markup/inline-JSON tool parsers are deprecated.

Execution flow:

1. Provider emits a native `ToolCallStart` + input deltas.
2. Agent forwards the call to the MCP runtime.
3. Permission policy (an MCP interceptor) decides `Allow`, `Ask`, or `Deny`.
4. MCP runtime executes the tool.
5. Result is emitted as a durable `ToolResult` event.

For text-only providers, a thin shim converts tool calls to/from plain text rather than maintaining a permanent parser stack.

### Harness skills

Skills are default-on, configurable interceptors on the agent turn. They register hooks (`on_turn_start`, `on_tool_call`, `on_turn_end`) to implement cross-cutting harness behavior without changing the base model.

Skills are declared in markdown files with YAML frontmatter:

```markdown
---
name: check-work
description: Verify changes with a subagent.
metadata:
  short-description: "Verify changes with a subagent"
triggers:
  - command: /check-work
  - command: /verify
---
```

The generic loader parses frontmatter and emits `SkillLoaded`. Adding a new skill means adding a file, not editing the engine.

Current built-in skills:

- `HashlineEditSkill` — line-addressed edits with content-hash verification.
- `VerificationLoopSkill` — runs a verification command after the model claims completion.
- `StartupContextSkill` — discovers cwd, tools, and environment before the turn.
- `LoopDetectorSkill` — detects repeated failed tool patterns.
- `ToolSchemaEnricherSkill` — adds examples to tool schemas.

See `crates/runie-core/src/harness_skills/mod.rs` for the trait and hook types.

### Search backend (`fff-search`)

File and content search are backed by the native `fff-search` crate. A long-lived `FffIndexerActor` keeps the index, frecency tracker, and query tracker in memory and serves both agent tools and the TUI `@` picker.

Query syntax:

| Syntax | Example | Effect |
|--------|---------|--------|
| Fuzzy text | `mylib` | Typo-tolerant filename/content search |
| Glob | `*.rs` | Filter by extension or pattern |
| Negation | `!test/` | Exclude matching files |
| Git filter | `git:modified` | Show files with given git status |
| Location | `lib.rs:42:5` | Jump to line/column |
| Quoted | `"exact phrase"` | Match exact phrase in content |

## Execution modes

- **Solo** (default): the user prompt goes directly to `AgentActor` with the session model.
- **Plan-first** (R4): `PlanActor` owns a graph of proposed steps. The agent emits `PlanCreated`; write tools are blocked until the user emits `ApprovePlan`. `TurnActor` executes approved steps and emits `PlanStepCompleted` facts.
- **Team** (R4): `OrchestratorActor` designs a workflow of roles and routes steps to subagents.

Team mode uses the Orchestrator-Harness Protocol (OHP): a typed plan with roles, sequential/parallel steps, and model-trait preferences. The orchestrator resolves traits to concrete models via the catalog.

## External interfaces

Runie exposes its event bus to the outside world through thin clients that talk to the `LeaderActor`:

1. **Streaming JSON headless mode** — `runie -p "task"` emits newline-delimited facts.
2. **JSON-RPC server** — `runie server` provides TCP/stdio JSON-RPC for IDE integrations.
3. **WebSocket server** — `runie agent serve` for IDE/editor integrations.

All clients send intents and receive the same fact stream. The TUI is just one consumer. Example headless event:

```json
{"type":"text","data":"Hello, "}
{"type":"tool_call_start","id":"call_1","name":"bash"}
{"type":"tool_call_end","id":"call_1"}
{"type":"end","stopReason":"EndTurn","sessionId":"...","requestId":"..."}
```

## Provider normalization

All providers emit a provider-agnostic `LLMEvent` stream:

```rust
pub enum LLMEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallInputDelta { id: String, delta: String },
    ToolCallEnd { id: String },
    Error(LLMError),
    Usage { input_tokens: u32, output_tokens: u32 },
    Finish,
}
```

Provider-specific parsing (for example MiniMax XML tool-call delimiters) is isolated in `runie-provider`.

## Declarative configuration

Most runtime behavior is declared in files rather than Rust code. The leader loads these at startup and publishes them as facts.

### Subagent type

```markdown
# resources/agents/explore.md
---
name: explore
description: Fast codebase exploration.
prompt_mode: full
model: inherit
permission_mode: default
agents_md: true
---

You are an expert explorer. Search broadly, then narrow down.
```

### Role / persona

```toml
# resources/roles/implementer.toml
name = "implementer"
description = "Implements the planned changes."
model = "fast"
effort = "high"
system_prompt = "You write clean, tested code."
allowed_tools = ["read_file", "write_file", "bash"]
```

### Model metadata

```yaml
# resources/models/grok-build.yaml
id: grok-build
name: Grok Build
base_url: https://api.x.ai/v1
context_window: 512000
api_backend: responses
supports_backend_search: true
auto_compact_threshold_percent: 80
```

### Permission rule

```toml
[[permissions]]
action = "allow"
tool = "read_file"

[[permissions]]
action = "ask"
tool = "bash"
pattern = "git push"
```

### MCP server

```bash
runie mcp add filesystem npx -y @modelcontextprotocol/server-filesroot ~/Code --transport stdio
```

A generic loader parses frontmatter and emits `SkillLoaded`, `CommandRegistered`, `AgentTypeRegistered`, `PermissionRulesLoaded`, `McpServerLoaded`, etc. Adding a feature usually means adding a file, not editing the engine.

## Code layout

```
crates/runie-core/src/
  event/            # CoreEvent enum and variants
  model/state/      # AppState + sub-states
  session.rs        # Session types
  actors/           # ConfigActor, ProviderActor, SessionActor, actor trait
  commands/         # CommandRegistry and slash handlers
  dialog/           # Panel/Form DSL
  harness_skills/   # Skill trait and implementations
  view/             # Element, Feed, LazyCache (domain projection)
  update/           # Event dispatch

crates/runie-agent/src/
  actor.rs          # AgentActor (interactive turn executor)
  turn.rs           # Agent turn loop
  subagent.rs       # Subagent runner
  tool/             # Built-in tool implementations

crates/runie-tui/src/
  main.rs           # Entry point and event loop
  ui/               # Rendering (Ratatui widgets, draw_snapshot)
  core_ui/          # Re-exports from runie-core::view
  popups/           # Dialog rendering
  theme/            # Theme tokens

crates/runie-provider/src/
  openai/           # OpenAI-compatible providers
  factory.rs        # Provider construction
  mock.rs           # Mock provider for tests
```

## Crate decisions

We prefer maintained crates over custom code when they reduce complexity and improve correctness.

| Area | Choice | Reason |
|------|--------|--------|
| Markdown parsing | `pulldown-cmark` | Standard, well-tested |
| Syntax highlighting | `syntect` | Existing Sublime grammar ecosystem |
| Diff generation | `similar` | Reliable diff output |
| Token counting | `tiktoken-rs` | Matches OpenAI tokenization |
| File search | `fff-search` | Native, typed, frecency-aware |
| Config watcher | `notify` | Removes polling |
| Clipboard | `arboard` | Cross-platform, no temp files |
| Diff parsing | `patch` | Fixes hand-rolled line-number bugs |
| YAML frontmatter | `serde_yml` | Correct YAML parsing |
| Color helpers | `palette` | Color-space correct blending |
| Fuzzy matching | `nucleo-matcher` | Better Unicode/ranking |
| Word wrapping | `textwrap` | Better line breaking |
| Session store | headered JSONL + `fs2` locks | Single file, advisory locking; SQLite deferred for now |

Custom code is kept where it is project-specific or where a drop-in crate does not exist: the text-input widget, command-palette DSL, dialog/form DSL, update dispatcher, actor runtime, and streaming stable/tail buffer.

## Configuration

User settings live in `~/.runie/config.toml`. See [Configuration](Configuration.md) for the full reference.

## Async IO discipline

The TUI runs on a multi-threaded Tokio runtime. Synchronous file or process IO must never run directly on an async task.

- Prefer native async APIs: `tokio::fs` for files, `tokio::process` for subprocesses, async `reqwest` for HTTP.
- Long-lived storage belongs in its own actor (for example `FffIndexerActor` and `SessionActor`).
- Legacy call sites that cannot be fully async yet must wrap blocking work with the helpers in `crates/runie-core/src/async_io.rs`:
  - `run_blocking_if_runtime` — fire-and-forget blocking work on a Tokio blocking thread.
  - `block_in_place_if_runtime` — run a short blocking closure off the async runtime and return the result.
- These helpers fall back to synchronous execution when no runtime is present, which keeps unit tests fast and deterministic.

Concrete remediation order:
1. If the call site can be made `async`, use `tokio::fs` / `tokio::process`.
2. If the caller is a sync function reached from an async actor (e.g., the update dispatcher or a sync skill hook), wrap the IO in `block_in_place_if_runtime`.
3. If the work is fire-and-forget or long-running, use `spawn_blocking` or `run_blocking_if_runtime`.

New code should default to async or event-based actors; the helpers are a tactical bridge, not the preferred pattern.

## Config durability

`~/.runie/config.toml` is the single source of truth for provider credentials, default model, keybindings, and preferences. `ConfigActor` is the only production code that reads or writes this file.

Rules:

- All config mutations are sent to `ConfigActor` as intents (`ConfigMsg`).
- `ConfigActor` performs atomic load → mutate → save under a write lock on a blocking thread, then publishes `ConfigLoaded`.
- `AppState` updates its config projection only in response to `ConfigLoaded` facts.
- No handler, command, dialog, or login flow writes the config file directly.
- `login_config.rs` is being removed; its helpers are replaced by `ConfigActor` messages and a `ConfigStore` trait for tests.
- Do not nest `block_in_place` calls: a function that already runs on a blocking thread must not call `block_in_place_if_runtime` again.
- Prefer atomic updates over fire-and-forget background writes for durable state.

## Build guardrails

`crates/runie-core/build.rs` enforces structural limits on production code:

| Metric | Limit |
|--------|-------|
| File lines | 500 |
| Function lines | 40 |
| Approximate complexity | 10 |

Tests are exempt from function-length and complexity checks so they can stay comprehensive.

## Current cleanup roadmap

The 2026-06-28 architecture and code review found that the implementation had drifted from the documented three-layer model. A second-pass review showed that several planned tasks were already complete on disk and have been archived under `tasks/archive/`. A third five-round review focused on replacing custom code with crates, unification, and Pareto simplification; its findings are recorded in [`docs/superpowers/plans/2026-06-28-less-code-crate-replacements.md`](superpowers/plans/2026-06-28-less-code-crate-replacements.md). A fourth five-round review dug deeper into provider/model/catalog/cache, session/store/index/replay, agent turn/subagent/tool search, TUI capabilities/diff/message/markdown, and DSL/view/dialog/commands; its findings are recorded in [`docs/superpowers/plans/2026-06-28-fourth-pass-crate-review.md`](superpowers/plans/2026-06-28-fourth-pass-crate-review.md). A fifth five-round review focused on build/CI/test harness, error handling/tracing/telemetry, protocol/IPC leftovers, declarative loaders/DSLs, and macros/codegen; its findings are recorded in [`docs/superpowers/plans/2026-06-28-fifth-pass-crate-review.md`](superpowers/plans/2026-06-28-fifth-pass-crate-review.md). A fresh five-round review after many tasks landed found additional post-implementation gaps, especially in actor plumbing, TUI rendering, and build/CI hygiene; its findings are recorded in [`docs/superpowers/plans/2026-06-28-five-round-review-synthesis.md`](superpowers/plans/2026-06-28-five-round-review-synthesis.md). A subsequent verification pass checked whether tasks marked `done` were actually complete; the results are recorded in [`docs/superpowers/plans/2026-06-28-task-verification-report.md`](superpowers/plans/2026-06-28-task-verification-report.md). A final five-round sweep after the backlog was mostly cleared found additional custom code and prematurely closed tasks; its findings are recorded in [`docs/superpowers/plans/2026-06-28-seventh-pass-final-sweep.md`](superpowers/plans/2026-06-28-seventh-pass-final-sweep.md). The active cleanup tasks from that sweep have since been completed; the closure is documented in [`docs/superpowers/plans/2026-06-28-active-tasks-closure.md`](superpowers/plans/2026-06-28-active-tasks-closure.md). The current state is `tasks/index.json` (**108 done**, **3 wontfix**, **0 active**) and is summarized in [`docs/superpowers/plans/2026-06-28-runie-cleanup-roadmap.md`](superpowers/plans/2026-06-28-runie-cleanup-roadmap.md).

### Active tasks

#### Phase 1 — Actor foundation (P0/P1)

1. **Migrate production actors to `ractor`** (`tasks/migrate-production-actors-to-ractor.md`) — `partial`. `InputActor`, `RactorPermissionActor`, and `RactorConfigActor` are already migrated and wired; migrate `ProviderActor`, `IoActor`, `SessionActor`, `FffIndexerActor`, and `AgentActor`. Also update `testing/actor_harness.rs` off the custom trait.
2. **Delete dead actor modules and custom trait** (`tasks/delete-dead-actor-modules-and-custom-trait.md`) — `done`. Removed the legacy `Actor` trait, dead actor modules and aliases, non-functional `RactorHandle::rpc`, and cleaned `ActorHandles` helpers.
3. **Collapse `ActorHandles` to a typed map** (`tasks/collapse-actor-handles-to-typed-map.md`) — replace the helper façade with typed `ractor::ActorRef` handles and reconcile `LeaderHandle`.

#### Phase 2 — Shared bootstrap (P1)

4. **Expand `Leader::start` for TUI and CLI** (`tasks/expand-leader-start-for-tui-and-cli.md`) — make the leader spawn the full actor set, expose actor refs, event subscription, a render snapshot channel, and graceful shutdown. Fix the `RactorPermissionHandle`/`PermissionActorHandle` type mismatch and make `Leader::new()` default to embedded mode.
5. **Migrate TUI and CLI to `Leader::start`** (`tasks/migrate-tui-and-cli-to-leader-bootstrap.md`) — replace duplicated manual bootstrap, remove the duplicate `RactorTurnActor` spawn in the TUI, and fix the ACP event plumbing (currently sends synthetic `Event`s to a dropped receiver and duplicates stdout forwarders).

#### Phase 3 — Event taxonomy (P1)

6. **Annotate `Event` variants to generate `EventKind`, `EventCategory`, and `Intent`** (`tasks/collapse-event-intent-kind-taxonomies.md`) — `Intent` is a semantic projection, not a mechanical mirror. Add attributes to `Event`, generate the taxonomies, delete `intent_impl.rs`, thin `kind/mod.rs`, and generate `names.rs`/`name.rs`.
7. **Use `strum` to derive `Event`/`Intent` names** (`tasks/use-strum-for-event-intent-names.md`) — replace manual `names.rs`/`name.rs`/`EVENT_NAMES` tables with `strum` derives after the taxonomy annotation task lands.

#### Phase 4 — Tool/provider shims (P2)

8. **Finish tool-parser shim: collapse legacy modules and marker stripping** (`tasks/replace-legacy-tool-parsers-with-thin-shim.md`) — `partial`. The shim routes parsing through `quick-xml` MiniMax and single-pass JSON, but still embeds `legacy`/`markup` submodules; inline/delete them, collapse `tool_markers/strip.rs` to two semantic passes, fix the `strip_empty_code_fences` guardrail violation, reconcile MiniMax ownership, and fix the one `cargo check` warning.
9. **Use `pulldown-cmark` for tool-marker stripping** (`tasks/use-pulldown-cmark-for-tool-marker-stripping.md`) — rewrite the stripper as a single `pulldown-cmark` event pass instead of regex passes.
10. **Centralize built-in tool names** (`tasks/centralize-built-in-tool-names.md`) — `done`. The canonical list already exists in `runie-core::tool::BUILTIN_TOOL_NAMES`; switch the remaining consumers to reference it.

#### Phase 5 — Declarative DSL quick wins (P2)

11. **Unify declarative resource loader** (`tasks/unify-declarative-resource-loader.md`) — extract shared directory-scan/frontmatter logic from `skills/load.rs` and `declarative/loader.rs`; decide and unify the frontmatter-vs-section-fallback policy.
12. **Use `pulldown-cmark-frontmatter` for resource loader** (`tasks/use-pulldown-cmark-frontmatter-for-resource-loader.md`) — replace the custom frontmatter/body scanner with `pulldown-cmark-frontmatter` + `serde_yaml` after the loader is unified.

#### Phase 6 — CLI config (P2)

13. **Route CLI config through `ConfigActor`** (`tasks/route-cli-config-through-configactor.md`) — extend `RactorConfigActor` to hold global + project paths, add layered-load and MCP messages, wrap CLI commands in a runtime, and route `inspect`/`mcp` through the actor.

#### Phase 7 — Public API boundary and helper crates (P2)

14. **Replace custom path/glob/fuzzy/keybinding helpers with crates** (`tasks/replace-custom-helpers-with-crates.md`) — delete `glob.rs`, `fuzzy.rs`, `path.rs`, and custom keybinding parsing; use `glob`/`globset`, `nucleo-matcher`/`sublime-fuzzy`, `shellexpand`, `crossterm`, and `tracing`.
15. **Narrow the `runie-core` public API** (`tasks/narrow-runie-core-public-api.md`) — usage-audit first; move shared helpers (`display_width`, `labels`, `sanitize`) to a new `runie-util` crate, delete `path`/`fuzzy`/`glob` if the helper-crate task lands first, and narrow the rest.

#### Phase 8 — Small safe cleanups (P3)

16. **Unify TUI render test helpers** (`tasks/unify-tui-render-test-helpers.md`) — move duplicated `render_content`, `render_chat`, and buffer-to-string loops into a shared test helper module.
17. **Fix keybindings dead-code warning** (`tasks/fix-keybindings-dead-code.md`) — convert `parse_key_combo` to `#[cfg(test)]` or document it.
18. **Clean up small duplicates and dead code** (`tasks/cleanup-small-duplicates-and-dead-code.md`) — `partial`. Remaining items: consolidate skill hooks, remove dead actor-handle fields, clean stale `#[allow(dead_code)]` and repetitive `FIXME` comments, and revisit `telemetry.rs` against `tracing`.

#### Phase 9 — Provider / config / auth crate replacements (P0/P1)

19. **Replace custom retry module with `backon`** (`tasks/replace-custom-retry-with-backon.md`) — delete `runie-provider/src/retry.rs` and use `backon`/`reqwest-retry` while preserving pre-stream retry semantics.
20. **Replace XOR-obfuscated auth storage with OS keyring** (`tasks/replace-xor-auth-with-keyring.md`) — use `keyring` for tokens with a headless/CI fallback.
21. **Replace custom config validator with `jsonschema`** (`tasks/replace-config-validator-with-jsonschema.md`) — validate serialized `Config` against the `schemars`-generated schema.
22. **Unify provider credential resolution with `dotenvy`** (`tasks/unify-provider-credential-resolution-with-dotenvy.md`) — load `.env` once and consolidate env-var fallback logic.
23. **Unify provider-config persistence helpers** (`tasks/unify-provider-config-persistence.md`) — route save/remove/list through a single helper or `RactorConfigActor`.

#### Phase 10 — CLI / commands / permissions simplification (P0/P1)

24. **Use `clap` derive macros for CLI argument parsing** (`tasks/use-clap-derive-for-cli.md`) — replace manual `args[1]` matching with typed subcommands.
25. **Use `notify` directly in `RactorConfigActor`** (`tasks/use-notify-directly-in-config-actor.md`) — remove the std-thread watcher bridge.
26. **Simplify slash-command DSL** (`tasks/simplify-slash-command-dsl.md`) — collapse `CommandSpec`/`CommandDef` into one representation.
27. **Unify permission-system rule engines** (`tasks/unify-permission-system-rules.md`) — merge `PermissionSet` and `PermissionManager` into a single ruleset.

#### Phase 11 — TUI / macros / testing cleanup (P0/P2)

28. **Replace custom TUI widgets with ratatui ecosystem crates** (`tasks/replace-custom-tui-widgets-with-ratatui-ecosystem.md`) — delete custom `Stylize`, input box, popup list, terminal setup sequences, and ANSI quantization in favor of `tui-textarea`, `tui-input`, `ratatui::widgets::List`, `crossterm`, and `ansi_colours`.
29. **Delete the dead `runie-macros` crate** (`tasks/delete-dead-runie-macros-crate.md`) — `done`. The crate no longer exists in the workspace.
30. **Centralize test fixtures and mocks** (`tasks/centralize-test-fixtures-and-mocks.md`) — move MiniMax fixtures, `MockToolSkill`, `ReplayProvider`, and `capture_events` into `runie-testing`.

#### Phase 12 — Final tooling simplification (P2/P3)

31. **Replace custom bash-safety heuristic with `shell-words`** (`tasks/replace-bash-safety-with-shell-words.md`) — tokenize with `shell-words` and apply a static deny-list.
32. **Replace custom build linter with Clippy / CI** (`tasks/replace-build-linter-with-clippy-ci.md`) — move guardrails into `[workspace.lints.clippy]` and a CI file-limit check.

#### Phase 13 — Fourth-pass provider / model / session unification (P0/P1)

33. **Unify session store and replay index into a single persistence backend** (`tasks/unify-session-store-and-index.md`) — replace the two-file JSONL store + `sessions.json` index with one headered JSONL file and `fs2` advisory locks. SQLite is deferred for now.
34. **Type and unify provider/model layer** (`tasks/type-and-unify-provider-model-layer.md`) — replace stringly typed provider/model config with typed structs and a single model catalog.
35. **Deduplicate turn-queue delivery logic** (`tasks/dedupe-turn-queue-delivery-logic.md`) — collapse overlapping queue/dispatch buffering into one queue with explicit delivery ids.
36. **Use channels for subagent result collection** (`tasks/use-channels-for-subagent-result-collection.md`) — replace callbacks/polling with `tokio::sync::mpsc`/`oneshot`.

#### Phase 14 — Fourth-pass parser / markdown unification (P0/P1)

37. **Unify markdown processing around `pulldown-cmark`** (`tasks/unify-markdown-processing-around-pulldown-cmark.md`) — consolidate all markdown parsing on one event stream.
38. **Replace think-block filter with regex** (`tasks/replace-think-filter-with-regex.md`) — replace custom streaming matcher with `regex` and the shared markdown stream.
39. **Extract shared streaming response parser** (`tasks/extract-shared-streaming-response-parser.md`) — provider-agnostic SSE/JSON/delta parser used by all providers.
40. **Delete or merge inspector tool pipeline** (`tasks/delete-or-merge-inspector-tool-pipeline.md`) — remove the duplicate `inspect` rendering path.

#### Phase 15 — Fourth-pass TUI simplification (P2)

41. **Simplify terminal capability detection** (`tasks/simplify-terminal-capability-detection.md`) — use `supports-color`/`supports-hyperlinks` and a single `TermCaps` snapshot.
42. **Unify core and TUI line-count computation** (`tasks/unify-core-and-tui-line-count-computation.md`) — one source of truth for wrapped line counts.
43. **Collapse `DialogState` variants** (`tasks/collapse-dialogstate-variants.md`) — refactor to a small, mutually exclusive state machine.

#### Phase 16 — Fifth-pass CI / build / dependency hygiene (P0/P1)

44. **Fix broken `schema` feature usage in CI and justfile** (`tasks/fix-broken-schema-feature-in-ci-and-justfile.md`) — remove `--features schema` from CI/recipes; `schemars` is already a normal dependency.
45. **Delete or fix dead `mcp` feature flag** (`tasks/delete-or-fix-dead-mcp-feature-flag.md`) — gate `config/mcp.rs` behind the feature or delete the feature.
46. **Remove unused dependencies and normalize workspace deps** (`tasks/remove-unused-dependencies-and-normalize-workspace-deps.md`) — remove `futures` from `runie-cli`, dedupe `tempfile`, and move inline `strum`/`notify`/`tracing`/etc. to workspace inheritance.
47. **Fix dev automation recipes** (`tasks/fix-dev-automation-recipes.md`) — fix `bacon.toml`'s `test` job and `just lint-fix`.
48. **Replace custom build linter with Clippy / CI** (`tasks/replace-build-linter-with-clippy-ci.md`) — port file/function/complexity checks to Clippy and a CI file-limit script.

#### Phase 17 — Fifth-pass error handling and observability (P0/P1)

49. **Unify library error types with `thiserror`** (`tasks/unify-library-error-types-with-thiserror.md`) — convert hand-written errors and library `anyhow` usage to typed `thiserror` enums.
50. **Initialize `tracing` subscriber in CLI/TUI binaries** (`tasks/initialize-tracing-subscriber-in-binaries.md`) — add `tracing-subscriber` and initialize it at startup.
51. **Replace custom telemetry with a `tracing` layer** (`tasks/replace-custom-telemetry-with-tracing-layer.md`) — delete the in-memory telemetry collector; emit model/tool events via `tracing`.
52. **Harden async actors against mutex poisoning** (`tasks/harden-actors-against-mutex-poisoning.md`) — replace `std::sync::Mutex` + `.unwrap()` in actors with `tokio::sync::Mutex` or `parking_lot`.

#### Phase 18 — Fifth-pass protocol / IPC cleanup (P1)

53. **Fold `runie-protocol` into `runie-core`** (`tasks/fold-runie-protocol-into-core.md`) — `done`. The crate no longer exists; types live in `crates/runie-core/src/proto/`.
54. **Unify CLI JSON-RPC transport and remove dead ACP plumbing** (`tasks/unify-cli-json-rpc-transport-and-remove-dead-acp.md`) — shared transport module; fix or delete the broken ACP path.

#### Phase 19 — Fifth-pass declarative loaders / DSL / small parser cleanup (P1/P2)

55. **Migrate subagent loader to unified resource loader** (`tasks/migrate-subagent-loader-to-resource-loader.md`) — replace the custom subagent frontmatter scanner with the shared loader.
56. **Fix dry-run tool-name discrepancy** (`tasks/fix-dry-run-tool-names-discrepancy.md`) — align `dry_run::core_tool_names()` with `BUILTIN_TOOL_NAMES`.
57. **Replace remaining custom parsers and macros with `strum`** (`tasks/replace-remaining-custom-parsers-and-macros-with-strum.md`) — delete `cmd!`, `with_ordering!`, theme macros; derive `FromStr`/`Display` for small enums.
58. **Implement MCP client runtime or remove config scaffolding** (`tasks/implement-or-remove-mcp-runtime-scaffolding.md`) — either wire up `rmcp` or delete unused MCP config.

#### Phase 20 — Fifth-pass test harness hardening (P2)

59. **Replace `scripts/tmux-test.sh` with Ratatui `TestBackend` tests** (`tasks/replace-tmux-test-with-ratatui-tests.md`) — delete the shell/tmux test and port coverage to Rust.
60. **Remove `sleep()` from automatic tests** (`tasks/remove-sleep-from-automatic-tests.md`) — replace sleeps in session actor tests with deterministic waits.
61. **Eliminate production `unwrap`/`expect`** (`tasks/eliminate-production-unwrap-expect.md`) — convert recoverable panics to typed errors.
62. **Introduce `cargo-deny` and `cargo-machete` CI checks** (`tasks/introduce-cargo-deny-and-cargo-machete-ci.md`) — audit dependencies and duplicates automatically.

#### Phase 21 — Post-review actor runtime hardening (P0/P1)

63. **Fix `RactorPermissionActor` reply lifecycle** (`tasks/fix-ractor-permission-actor-reply-lifecycle.md`) — stop auto-denying; resolve pending requests via `ResolvePermission`.
64. **Reconnect or remove TUI `AgentActor` channel** (`tasks/reconnect-tui-agent-actor-channel.md`) — fix the disconnected agent channel in the TUI bootstrap.
65. **Use `ractor` `State` for actor mutable state** (`tasks/use-ractor-state-for-actor-mutable-state.md`) — move state out of interior `Mutex` into `type State`.
66. **Wire or delete `InputActor`** (`tasks/wire-or-delete-input-actor.md`) — decide whether the orphan input actor stays.
67. **Delete dead actor handle wrappers** (`tasks/delete-dead-actor-handle-wrappers.md`) — remove legacy handle structs.
68. **Actually collapse `ActorHandles` to a typed map** (`tasks/actually-collapse-actor-handles-to-typed-map.md`) — finish the handle collapse.

#### Phase 22 — Post-review queue/mutex normalization (P1/P2)

69. **Remove sync `TurnQueue` fallback from `AppState`** (`tasks/remove-sync-turn-queue-fallback-from-app-state.md`) — delete duplicated sync delivery methods.
70. **Normalize remaining `std::sync::Mutex`/`RwLock` to `parking_lot`** (`tasks/normalize-remaining-std-mutex-to-parking_lot.md`) — permissions, fff_indexer, runie-agent.
71. **Delete `EventBusBridge` wrapper** (`tasks/delete-eventbusbridge-wrapper.md`) — hold `EventBus` directly.
72. **Use channels for subagent result collection** (`tasks/use-channels-for-subagent-result-collection.md`) — replace callback/polling.

#### Phase 23 — Post-review provider/config/security (P1/P2)

73. **Store provider API keys in OS keyring, not plain config.toml** (`tasks/store-provider-api-keys-in-keyring-not-config.md`) — move secrets out of TOML.
74. **Add `jsonschema` validation to `ConfigActor` load** (`tasks/add-jsonschema-validation-to-configactor-load.md`) — validate configs at load time.
75. **Actually replace `runie-provider` custom backoff with `backon`** (`tasks/actually-replace-runie-provider-backoff-with-backon.md`) — finish the retry cleanup.
76. **Drop unused `rand` from `runie-provider`** (`tasks/drop-unused-rand-from-runie-provider.md`) — deterministic mock delays.

#### Phase 24 — Post-review TUI/markdown unification (P0/P2)

77. **Replace hand-rolled TUI markdown block layout with `tui-markdown`** (`tasks/replace-tui-markdown-block-layout-with-tui-markdown.md`) — highest-impact TUI simplification.
78. **Unify markdown block parsing and healing on `pulldown-cmark` events** (`tasks/unify-markdown-block-parsing-on-pulldown-cmark-events.md`) — remove marker re-injection.
79. **Unify TUI message wrapping with `textwrap`** (`tasks/unify-tui-message-wrapping-with-textwrap.md`) — display-width-aware wrapping.
80. **Replace TUI keymap combo stringification with `crokey`** (`tasks/replace-tui-keymap-combo-stringification-with-crokey.md`) — round-trip key combos.
81. **Rescope terminal capability task to current module** (`tasks/rescope-terminal-capability-task-to-current-module.md`) — update stale file paths.

#### Phase 25 — Post-review declarative loaders / build / CI (P2/P3)

82. **Deserialize declarative command YAML with typed structs** (`tasks/deserialize-declarative-command-yaml-with-typed-structs.md`) — remove manual YAML walking.
83. **Move built-in slash commands to declarative YAML** (`tasks/move-built-in-slash-commands-to-declarative-yaml.md`) — remove static Rust tables.
84. **Replace session tree with arena crate** (`tasks/replace-session-tree-with-arena-crate.md`) — `indextree` or `ego-tree`.
85. **Move `cargo-deny`/`cargo-machete` from workspace deps to CI** (`tasks/move-cargo-deny-machete-from-workspace-deps-to-ci.md`) — binary tools should not be workspace deps.
86. **Extract shared tracing subscriber init** (`tasks/extract-shared-tracing-subscriber-init.md`) — dedupe subscriber setup.
87. **Clean up `verify-tests.sh` and `just lint-fix`** (`tasks/cleanup-verify-tests-and-just-recipes.md`) — fix brittle dev recipes.
88. **Remove redundant `check-field-access.sh`** (`tasks/remove-redundant-check-field-access-script.md`) — duplicate linter.
89. **Replace leftover macros with functions** (`tasks/replace-leftover-macros-with-functions.md`) — `with_panel_stack!`, `with_ordering!`, test skip macros.
90. **Delete unused glyph constants** (`tasks/delete-unused-glyph-constants.md`) — dead TUI glyphs.

### Archived completed tasks

The following 2026-06-28 review tasks were already complete on disk and are now in `tasks/archive/`:

- `repair-and-canonicalize-dialog-module`
- `delete-empty-runie-domain-and-runie-io-crates`
- `prune-dead-provider-code-and-rig-core-dependency`
- `deduplicate-provider-registry-data`
- `remove-dead-ipc-event-abstractions`
- `merge-diff-modules`
- `rename-core-ui-to-view`
- `inline-tui-ipc-reexport`
- `fold-protocol-into-core`
- `unify-duplicate-module-names-core-tui`

Earlier completed architectural work (actor SSOT, config SSOT, MCP adoption, etc.) is also preserved in `tasks/archive/`.

### Execution order

The plan is phased so that **every merged task leaves `cargo check --workspace` green**:

1. Run the actor-runtime sequence in order (Tasks 1 → 2 → 3).
2. Run the bootstrap sequence in order (Tasks 4 → 5).
3. Land the event-taxonomy annotation/generation (Task 6), then adopt `strum` names (Task 7).
4. Run the tool-parser and centralize-tool-names tasks in either order (Tasks 8–10).
5. Run the declarative-loader tasks in order (Task 11 → 12) in parallel with the above.
6. Route CLI config through the actor (Task 13).
7. Replace custom helpers with crates (Task 14) before narrowing the public API.
8. Narrow the public API (Task 15).
9. Run the small cleanups (Tasks 16–18).
10. Land provider/config/auth crate replacements (Tasks 19–23), after `route-cli-config-through-configactor` for the config-related ones.
11. Land CLI/commands/permissions simplifications (Tasks 24–27), after `use-clap-derive-for-cli` for the slash-command DSL.
12. Land TUI/macros/testing cleanup (Tasks 28–30).
13. Run final tooling simplification (Tasks 31–32).
14. Land fourth-pass provider/model/session unification (Tasks 33–36); `dedupe-turn-queue-delivery-logic` must precede `use-channels-for-subagent-result-collection`.
15. Land fourth-pass parser/markdown unification (Tasks 37–40); `unify-markdown-processing-around-pulldown-cmark` must precede `replace-think-filter-with-regex`, and `type-and-unify-provider-model-layer` must precede `extract-shared-streaming-response-parser`.
16. Land fourth-pass TUI simplification (Tasks 41–43).
17. Land fifth-pass CI/build/dependency hygiene (Tasks 44–48); `fix-broken-schema-feature-in-ci-and-justfile` should land early so CI stays green.
18. Land fifth-pass error handling and observability (Tasks 49–52); `unify-library-error-types-with-thiserror` must precede `eliminate-production-unwrap-expect`.
19. Land fifth-pass protocol/IPC cleanup (Tasks 53–54); `fold-runie-protocol-into-core` must precede `unify-cli-json-rpc-transport-and-remove-dead-acp`.
20. Land fifth-pass declarative loaders / DSL / parser cleanup (Tasks 55–58).
21. Land fifth-pass test harness hardening (Tasks 59–62).
22. Land post-review actor runtime hardening (Tasks 63–68); `fix-ractor-permission-actor-reply-lifecycle` and `reconnect-tui-agent-actor-channel` are P0.
23. Land post-review queue/mutex normalization (Tasks 69–72); `remove-sync-turn-queue-fallback-from-app-state` must precede `use-channels-for-subagent-result-collection`.
24. Land post-review provider/config/security (Tasks 73–76); `store-provider-api-keys-in-keyring-not-config` should land near `type-and-unify-provider-model-layer`.
25. Land post-review TUI/markdown unification (Tasks 77–81); `unify-markdown-block-parsing-on-pulldown-cmark-events` must precede `replace-tui-markdown-block-layout-with-tui-markdown`.
26. Land post-review declarative loaders / build / CI (Tasks 82–90).

## Testing philosophy

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the full 4-layer test taxonomy.
