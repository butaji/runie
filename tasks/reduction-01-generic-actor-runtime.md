# Reduction 01: generic actor runtime

Status: adopted

## Finding

`UiActor`, `PromptActor`, `ScrollbackActor`, `StatusActor`, and core actors
repeat mailbox, watch snapshot, owned task, acknowledgement, and event-loop
plumbing.

## Existing foundation

`runie-core::task_owner` already provides owned-worker, actor-worker, and
acknowledgement macros. These cover lifecycle safety but do not yet unify the
state/snapshot/reducer loop.

`runie-core::ReducerActor<S, E>` now provides that generic reducer loop with
ordered acknowledged events and watch snapshots. Existing actors can migrate
incrementally while retaining domain-specific command adapters.

`declare_reducer_actor!` generates the mechanical domain handle when a typed
actor has no additional command semantics; it does not hide the reducer or
task lifecycle.

`StatusActor` is the first production actor migrated to the macro-backed
runtime. Its event-bus bridge remains explicit, while mailbox, acknowledgement,
snapshot, and worker boilerplate are removed.

## Target

Provide a small generic actor runtime that owns lifecycle mechanics while each
domain supplies state, commands, events, and a pure reducer.

## Acceptance

- no orphan tasks;
- domain reducers remain explicit and testable;
- at least two current actors use the runtime;
- focused actor tests and workspace checks pass.
