# Runie findings ledger

Updated: 2026-08-10

## Architecture

- `runie-core` owns domain actors and asynchronous I/O. TUI projections must
  consume snapshots or events rather than mutating core state.
- Rendering is a pure function of view properties. Animation advances only from
  explicit actor-owned events; tests must not use sleeps.
- YAML/event-sequence replay is the preferred behavioral contract because it
  makes transitions and expected state inspectable.
- Macros are most valuable when they turn typed data into registries,
  dispatch tables, dialog specs, and test cases. They should not conceal
  concurrency or domain decisions.

## Dialog and command surface

- `DialogSpec`, `DialogFrame`, and `DialogStack` are the shared overlay model.
- Commands and skills are palette data. Built-in commands expose descriptions,
  source labels, fuzzy matching, and parameter hints.
- Parameterized commands push `PALETTE_PARAMETERS_DIALOG`; submit maps to a
  core mappable command, and `Esc` returns to the previous frame.
- Files, models, shortcuts, session info, changelog, and command parameters all
  render through the shared dialog widget and specs.

## Grok palette reference

The local Grok source is `/Users/admin/Code/agents/grok-build`.

- Command palette modal sizing is 50% width, max 80, min 44, vertical margin
  4, with two footer lines.
- Search uses `/ to search`; footer shortcuts are navigation, select, and close.
- Picker rows are data with label, description/summary, source, selection, and
  optional expansion. The selected row is visually distinct and the list is
  keyboard navigable.
- A live 120×36 tmux capture was taken from the documented
  `xai-grok-pager-bin` target. The capture is the runtime reference; source
  constants are the stable geometry reference.

Runie intentionally clears the slash from the prompt when opening its palette,
per the product interaction contract. This differs from Grok's inline slash
completion prompt but preserves the requested Runie behavior.

## Verification snapshot

The latest focused verification before this consolidation passed:

- `cargo test --workspace --quiet`: all workspace test binaries passed (the
  largest current groups contain 264, 234, and 185 tests)
- `cargo check --workspace --quiet`
- `cargo run -p lint-check --quiet`: clean across the workspace.

Future parity claims should update the counts and add the exact command/output
or fixture name here rather than creating another free-form task file.

## Reduction implementation evidence

As of 2026-08-09, the reduction work has these verified results:

- `EventMemo` is used by the scrollback and status actors.
- `SharedSnapshot<S>` provides an `Arc`-backed immutable transfer view for
  projections whose owning memo can be consumed.
- `shared_snapshot_alloc.rs` verifies shared clones avoid repeated deep
  projection allocations.
- `runie_core::replay_yaml` parses ordered YAML event sequences into the same
  memoized reducer path, with malformed-input coverage.
- `ReducerActor` and `declare_reducer_actor!` are implemented; `StatusActor`
  and `CommandActor` use the generated runtime handles.
- `EventProjection` is consumed by status and UI actors.
- `FeedSnapshot` now has a sole `FeedFacts` facts boundary; reducer and
  scrollback consumers no longer use flattened fact fields.
- Tool metadata now uses one `ToolRecord` map instead of parallel name and
  argument indexes.
- Tool-card projection accepts the normalized registry directly, avoiding
  temporary name-map reconstruction.
- Feed snapshot navigation projection now uses a typed macro field table for
  copy-versus-clone semantics and derived selection data; `runie-tui-model`
  passes 192 tests after the migration.
- `LineKind` now owns shared tool-header/tool-line predicates used by feed
  mutation, selection, error, and rendering paths; the model suite passes 193
  tests after the consolidation.
- Live tool-header classification is shared by tool-card, row-selection, and
  replacement projections, preserving the distinction between running and
  settled error rows.
- Activity-tool aliases now come from one grouped typed macro, keeping the
  classifier as a data declaration while preserving all existing aliases.
- Transcript-selectability is now a `LineKind` predicate shared by feed
  navigation, completing the central line-classification vocabulary.
- Five numbered feed fragments covering tool-row lifecycle and selection are
  consolidated into `feed_state_tool_rows.rs`; behavior remains covered by
  the 193-test model suite.
- `component_specs!` generates the declarative component ownership table.
- `event_trace!` reduces pure replay-test boilerplate.
- `cargo test --workspace --quiet` passes across the current workspace.
- `cargo test -p runie-tui-model --quiet`: 189 tests pass after moving the
  domain-classifier test out of the oversized test fragment.
- `scripts/tmux-command-smoke.sh` passed all 52 palette commands at
  120x36 in disposable tmux sessions; parameterized commands were submitted
  with a safe `smoke` value and expected picker/dialog transitions were
  observed.
- A separate tmux smoke pass verified `Quit` terminates its session.
- The refreshed matrix also passes the direct stored-session resume-picker
  case and the unconditional Quit lifecycle, for 54/54 TUI-only cases. The
  provider-backed coding prompt path remains unverified because
  `MINIMAX_API_KEY` is absent from the environment.

The active backlog is intentionally not marked complete: normalized feed
records, paint-data rendering, generic YAML traces, and semantic module
consolidation still require source changes and dedicated verification.

## Code-reduction backlog

These are the twelve source-backed reduction findings from the architecture
review. They are tracked individually so implementation and verification can
progress without losing the original scope.

1. `reduction-01-generic-actor-runtime.md` — unify repeated mailbox, snapshot,
   acknowledgement, event-subscription, and owned-task plumbing.
2. `reduction-02-memoized-domain-state.md` — make actor state a pure
   `memo(events)` projection with explicit replayable reducers.
3. `reduction-03-declarative-scrollback-events.md` — replace layered
   `ScrollbackMsg` routing with typed domain event groups and data operations.
4. `reduction-04-normalized-feed-state.md` — store normalized feed records and
   derive tool/activity/navigation projections.
5. `reduction-05-declarative-view-tree.md` — make TUI layout and component
   composition a data-driven scene graph.
6. `reduction-06-event-projection-registry.md` — centralize AgentEvent
   projections instead of repeating matches per consumer.
7. `reduction-07-state-widget-separation.md` — keep actors on immutable model
   snapshots and make widgets pure renderers.
8. `reduction-08-immutable-snapshot-sharing.md` — avoid repeated deep snapshot
   copies through shared immutable versions and memoized projections.
9. `reduction-09-declarative-schema-generation.md` — generate repetitive
   registries, dispatch glue, and metadata from typed data declarations.
10. `reduction-10-event-trace-harness.md` — use one YAML/event-sequence test
    harness across core, projections, and visual behavior.
11. `reduction-11-paint-data-rendering.md` — render widgets from declarative
    paint/layout data rather than hand-written branching.
12. `reduction-12-semantic-module-consolidation.md` — consolidate artificial
    numbered/fragments into semantic modules after behavior is stable.

The reduction implementation is intentionally still open: normalized feed
records, shared snapshot allocation measurement, YAML trace integration, full
paint-adapter migration, and final semantic consolidation remain tracked as
partial work in their individual task files.

## Coding harness backlog

The complete coding-agent feature comparison is tracked in
[`tasks/harness-findings.md`](harness-findings.md). It deliberately separates
existing Runie foundations from missing production capabilities and ranks the
work by user impact.
