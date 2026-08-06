# Agent Guidelines

Every change needs fast automatic tests (unit + replay). No `sleep()` in tests.

There is **no GitHub CI** for this repository. Do not create, restore, or
reference GitHub Actions or any `.github/workflows` configuration; remove it if
encountered. All verification is local via the `justfile` (`just ci`:
fmt-check + clippy + lint + test) or `cargo` directly.

## Architecture Principles

- **Actors, single-source-of-truth (SSOT).** Each state slice is owned by exactly one actor. The only change mechanism is events published by the owning actor. Handlers, tools, and tests never mutate another actor's state directly.
- **Events-based.** Everything that changes flows through events on a shared bus. Read-only projections / snapshots are rebuilt from events, never stored alongside the source of truth.
- **Async, reactive, pure functions.** State transitions are pure functions of the event; side effects live in actors; the whole system is async and reactive (no blocking waits, no polling loops).
- **TUI is MVU + actors.** The TUI is a Model-View-Update loop: the model (view state) is held by actors, the view is a pure function of the model, and updates are driven by events/messages. The TUI does not mutate core state directly.
- Every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event). No orphan `tokio::spawn`.
- TUI render paths are pure reads: resolve filesystem/process metadata before
  entering the redraw loop and cache it; never spawn commands or block while
  drawing a frame.

## File Structure

```
crates/
└── runie-core/    # Port of pi-agent-core: agent loop, state, events, tools, queues
lint-check/        # Build-script-style linter enforcing project rules
tasks/             # Implementation plan: index.json + NN-name.md per step
```

## Linter Rules

`lint-check/src/main.rs` enforces, across `crates/runie-core/src/**.rs`:

| Check | Scope |
|-------|-------|
| Magic numbers (>= 1000) | all production code |
| Orphan `tokio::spawn` calls | all production code |

**Magic numbers** — use named constants for buffer sizes, timeouts, and thresholds. Exempt: numbers below 1000, underscore-separated, hex, HTTP/JSON-RPC codes, test code.

**Orphan spawns** — every `tokio::spawn` must be owned by an actor (handle stored in `JoinSet` or actor mailbox).

**Guidelines**: keep files small, functions focused, and complexity low. Split files around ~400 lines and functions around ~60 lines.

## Tooling: DSL + macros to cut boilerplate

Prefer small internal DSLs and `macro_rules!`/proc macros over hand-written
boilerplate for the three recurring patterns:

- **Actors.** A macro to define an actor from a mailbox command enum + a pure
  `async fn` handler: derives the `mpsc` worker, the `Clone` handle, and the
  per-command `reply` wiring. Usage: `actor!(ProviderActor, ProviderCommand,
  handle)` etc. Keep the macro thin — one struct, one trait, one worker loop.
- **Events.** A derive/macro for the event enum that synthesizes
  `kind()`/`is_*` predicates, `Clone`, serde, and a `subscribe`/`publish`
  convenience on the bus, so event handling reads declaratively.
- **TUI (MVU).** A macro for the update function that maps
  `(State, Msg) -> (State, Cmds)` and wires the view as a pure function of
  state, so components are declared as data + a reducer instead of imperative
  loops.

Rules: macros must stay in `runie-core` (or a `runie-macros` crate) and be
covered by tests; no "magic" invisible behavior — an expansion must be
readable via `cargo expand`. If a macro adds more complexity than it removes,
write it out by hand instead.

## Tests as YAML instructions

Prefer data-driven YAML tests over compiled Rust tests whenever the behavior
can be expressed declaratively:

- **Replay / event-sequence tests** live in `tests/traces/` as `.sse` +
  `.sse.yaml` pairs (already the convention): the YAML declares the expected
  provider events, core events, tool requirements, and state, so new traces
  need no recompilation.
- **TUI scenario tests** live in `crates/runie-tui/tests/e2e/*.yaml`: the YAML
  declares the scenario (prompt, tools, event script) and the visual/transcript
  assertions, driven by `yaml_runner` without recompiling.
- Only write a compiled Rust test when the behavior cannot be pinned in YAML
  (e.g. concurrency, timing, macros, serde round-trips).
- Every YAML fixture must be self-describing (name, description, expectation)
  and must be exercised by at least one test that fails loudly if a fixture is
  orphaned or malformed.

## Naming

The crate is named `runie-core` (a port of `@earendil-works/pi-agent-core`).
This recycles the name from a prior wipe; the previous role ("Config, providers,
permissions") is gone and not part of this build.
