# Stop dotenvy from mutating process environment

## Status

`done`

## Context

`CredentialResolver` calls `dotenvy::dotenv()`, which injects `.env` into `std::env` and forces test serialisation via `ENV_LOCK`.

## Goal

Load `.env` into a local `HashMap` without mutating the process environment.

## Acceptance Criteria
- [x] Use `dotenvy::from_filename_iter` or `figment::Env` into local map.
- [x] Preserve priority: env > dotenv > keyring > config.
- [x] Remove or reduce `ENV_LOCK` reliance.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for env override precedence.
- **Layer 2 — Event Handling:** Config-loaded facts reflect merged env.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Parallel tests no longer deadlock on `ENV_LOCK`.
- **Live tmux validation:** `.env` file values load without global side effects.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Implementation Notes

Changed `load_dotenv()` in `crates/runie-core/src/auth/credential.rs` to use
`dotenvy::from_filename_iter` instead of `dotenvy::dotenv`. This reads the .env
file directly into a HashMap without mutating the process environment.

Added test `load_dotenv_does_not_mutate_process_env` to verify the behavior.
