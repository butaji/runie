# EXECUTE — Agent Mode

Implement, refactor, and ship backlog items while keeping the codebase unified,
simple, and event-driven.

## Goal

Implement all tasks listed in `tasks/` in the `runie-tests` repo. Work test-first:
write black-box tests in `runie-tests`, then implement the behavior in `runie`
with the matching unit and E2E tests. Apply **Pareto (80/20)**: minimum change
to reach the desired state, then stop.

## Architecture

Three layers:

1. **IO layer (async)** — files, network, subprocesses, OS. Runs inside dedicated
   actors. Results arrive as events.
2. **Domain layer (pure + actors)** — actors own authoritative state; business
   rules are pure.
3. **UI layer (pure / MVU)** — `draw(&mut Frame, &Snapshot)` is a pure function
   of facts.

Rules:

- Actors are the single source of truth for their state slice.
- Handlers emit **intents**; actors consume intents, update state, and emit
  **facts**.
- No handler, command, dialog, or render function mutates actor-owned state
  directly.
- Blocking IO belongs in IO actors, never in handlers or the render path.
- Complexity is hidden behind small declarative DSLs.

## Before you write code

1. Read `AGENTS.md`, `docs/Architecture.md`, `docs/UI_UX.md`, and this file.
2. Pick the next task from `tasks/` and read its spec.
3. Plan non-trivial changes first (`EnterPlanMode`), then execute.
4. Use parallel subagents for independent sub-tasks.

## How to implement

- One task = one focused commit.
- Prefer deletion, consolidation, and unification over addition.
- No speculative abstraction; concrete DSLs win.
- Follow existing code style and crate boundaries.
- Replace custom code with crates or OS features when there is a clear Pareto win.

## Testing

Every change must be verifiable by `cargo test --workspace`. Follow the 4 layers
in `AGENTS.md`.

Never:

- Use `sleep()` in tests.
- Use shell or tmux tests.
- Test widget internals instead of rendered output.
- Leave `cargo check --workspace` warnings or errors.

## Verification before claiming done

1. Re-read the task requirements and check acceptance criteria against the code.
2. Run `cargo check --workspace` and `cargo test --workspace`.
3. Grep for the old code that was supposed to be removed.

## Commit & push

- Commit after each completed task with a clear, imperative message.
- Ensure `cargo check --workspace` is green before every push.

## Hard no's

- Direct `AppState` mutation outside the fact-projection path.
- Logic duplicated across handlers, actors, and UI.
- Blocking or long-lived work in handlers / render path.
- New runtime dependencies without a concrete Pareto justification.
- Marking tasks `done` before acceptance criteria are satisfied.
