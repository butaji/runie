# Remove unused workspace dependencies

## Status

**done** — Removed unused dependencies based on `cargo machete` analysis.

## Description

Removed unused workspace and crate dependencies identified by `cargo machete`:

### Removed from workspace (Cargo.toml)
- `winnow = "1.0"` — unused across all crates
- `serde_with = "1"` — unused across all crates

### Removed from runie-core (runie-core/Cargo.toml)
- `winnow.workspace = true` — unused
- `serde_with.workspace = true` — unused

### Removed from runie-agent (runie-agent/Cargo.toml)
- `reqwest.workspace = true` — unused

### Removed from runie-tui (runie-tui/Cargo.toml)
- `base64.workspace = true` — unused
- `ratatui-textarea = "0.9.2"` — unused (replaced with custom ratatui rendering)

## Acceptance criteria

- [x] **Unit tests** — Workspace builds with no unused workspace-level deps.
- [x] **E2E tests** — Smoke tests pass.
- [x] **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- Grep confirms no crate uses the removed deps.
- `cargo machete` no longer reports these deps.

### E2E tests
- Full workspace build passes.
- All tests pass.

### Live tmux tests
- N/A.
