# p20 — Thin actor DSLs and YAML-first tests

**Objective:** keep recurring actor ownership boilerplate small and make
behavior fixtures editable without recompiling Rust.

## Rules

- Macros remain thin and readable in `cargo expand`; they may remove repeated
  wiring but must not hide mailbox commands, state transitions, or event order.
- YAML is the default format for replay and TUI behavior scenarios. Rust tests
  provide the runner and only cover concurrency, timing, macro expansion, and
  serialization contracts that YAML cannot express.
- Every YAML fixture is self-describing and discovered by a test; orphaned or
  malformed fixtures fail loudly.

## Progress

- **TUI actor ownership reuse (2026-08-06):** `UiActor` and `PromptActor`
  now use the shared `runie-core` owned-worker DSL. Their worker lifetimes
  remain attached to cloned actor handles, while the duplicate local
  `JoinHandle`/`Drop` owner wrapper was removed. The actor ownership macro tests
  and full workspace gate verify the migration.

- **TUI mailbox DSL reuse (2026-08-06):** Both actors now also use
  `spawn_actor_worker!` for channel construction, so mailbox capacity and task
  ownership are established by the same thin DSL used by core actors. Command
  types and worker reducers remain explicit at each call site.

- **Subscriber ownership DSL reuse (2026-08-06):** The core loop's event
  subscriber bridge now uses `spawn_owned_worker!` instead of manually wrapping
  its `tokio::spawn` in `TaskOwner`. The loop run handle remains explicit
  because it carries a result and is synchronously awaited by the busy/idle
  coordinator.

- Added `spawn_owned_worker!`, a small internal DSL that constructs an
  aborting `TaskOwner` around an actor worker handle. Mailboxes and worker
  loops remain explicit at each call site.
- Added `spawn_actor_worker!`, a thin mailbox DSL shared by state, queue,
  provider, and tool actors; it centralizes channel creation plus owned worker
  attachment without hiding command enums or event handlers.
- Added `mailbox_call!`, a thin one-shot reply DSL used by both queue actors;
  it centralizes reply-channel creation and closed-mailbox defaults while
  leaving command constructors and actor-owned mutations explicit.
- Added direct mailbox-to-worker expansion coverage for `spawn_actor_worker!`;
  the macro test sends a command through the generated channel and verifies
  the owned worker lifecycle.
- Added `agent_event_kind!`, a readable declarative match shared by replay and
  integration harnesses so YAML event names have one source of truth.
- Added `assistant_event_kind!`, removing the handwritten provider-event
  naming match from the replay harness and covering the macro expansion.
- Existing `.sse.yaml` replay fixtures and TUI `tests/e2e/*.yaml` runners are
  the canonical fast behavior surface; the replay harness discovers all 183
  sidecars dynamically, and the TUI integration suite exercises all 22 YAML
  fixtures dynamically.
- Added the optional `core.ordered_events` YAML oracle and exercised it in the
  simple OpenAI trace; scenario-specific ordering is now editable without
  recompiling Rust.
- Added declarative `Shift+Enter`/`Alt+Enter` prompt steps and a multiline TUI
  fixture, keeping this behavior in the YAML scenario layer.
- Added a declarative `Ctrl+L` step and file-search prompt fixture.
- Added declarative `Shift+Tab` support and `visual-plan.yaml` coverage for the
  plan-mode prompt variant.
- Removed a timing-sensitive footer assertion from the typed visual fixture;
  it now asserts only the stable prompt content.
- Added `visual-tool-structured.yaml` coverage for multiline structured tool
  updates through the real tool executor and event renderer.
- Added a test-level YAML fixture discovery guard so malformed or orphaned TUI
  fixtures fail during `cargo test`.

## Completion

- Fixed declarative multi-tool replay indexing: each YAML `tool_call` now
  receives its event-sequence index instead of every call using index zero,
  preventing tool-call reconstruction from overwriting earlier calls. The
  mixed-activity fixture and full local CI pass.

All current parity additions have YAML scenarios before or alongside compiled
tests, and both thin DSL macros have expansion coverage. The rich markdown YAML
scenario now exercises blockquote gutters, six-level ATX headings, and table
box-drawing rows through the real TUI event path. Replay fixtures remain
runtime-discovered and editable without recompiling the runner.
- **State assertion DSL (2026-08-05):** Added `assert_declared_state!` to
  the replay harness. YAML owns projection values while startup-error,
  abort, and success branches reuse one readable macro expansion for
  `is_streaming`, pending-tool, and error assertions.

- **Submitted-turn state coverage (2026-08-05):** Added declarative state
  assertions to `visual-submitted.yaml`, pinning the post-event state
  (`is_streaming: false`, no pending tools, two messages) alongside its visual
  transcript assertions.
- **Structured-tool state coverage (2026-08-05):** The structured-update
  fixture now also asserts the terminal core state declaratively, keeping tool
  rendering and functional message-count verification in one YAML scenario.

- **Typed registry DSL (2026-08-06):** Added the readable
  `typed_action_registry!` macro in `runie-core` and used it for TUI
  command-palette actions. The expansion generates the enum and label match;
  macro and palette registry tests cover known and unknown labels.

- **Registry SSOT (2026-08-06):** The macro-generated palette labels are now
  consumed directly by the pure command-palette widget; display filtering and
  actor activation no longer maintain separate label lists.
