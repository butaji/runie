# Stop dotenvy from mutating process environment

## Status

`todo`

## Context

`CredentialResolver` calls `dotenvy::dotenv()`, which injects `.env` into `std::env` and forces test serialisation via `ENV_LOCK`.

## Goal

Load `.env` into a local `HashMap` without mutating the process environment.

## Acceptance Criteria
- [ ] Use `dotenvy::from_filename_iter` or `figment::Env` into local map.
- [ ] Preserve priority: env > dotenv > keyring > config.
- [ ] Remove or reduce `ENV_LOCK` reliance.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for env override precedence.
- **Layer 2 — Event Handling:** Config-loaded facts reflect merged env.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Parallel tests no longer deadlock on `ENV_LOCK`.
- **Live tmux validation:** `.env` file values load without global side effects.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
