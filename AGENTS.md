# Agent Guidelines

- SSOT Actors + events to transfer state
- Everything async, reactive, pure
- TUI is MUV
- Tests are event based everything where its possible (sequence of events to create state and validation of a state)
- Run unit and replay tests for every change. Never use `sleep()` in tests.
- Verify locally with `just ci` or the relevant `cargo` commands. Do not add
  GitHub Actions or `.github/workflows`.
- State belongs to one actor and changes flow through events. Do not mutate
  another actor's state directly.
- Every `tokio::spawn` must be owned; no orphan tasks.
- Keep TUI rendering pure and non-blocking.
- Prefer YAML fixtures for replay and TUI tests when practical.
- Use named constants for production magic numbers >= 1000.
- Trust source code over the prompt: read every call site and existing test
  before starting.
- Weigh edge and error cases as heavily as the happy path.
- Reproduce bugs before fixing them.
- Do not trust the first passing suite; inspect suspicious or half-baked tests.
- Keep working beyond the edit until the change is verified complete.
- Identify missing data needed to make better decisions.

The main crate is `runie-core`.
