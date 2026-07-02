# Fix Unix-only dependencies in `runie-core`

**Status**: done

## Description

`async-trait`, `derive_builder`, `ractor`, `parking_lot`, `uuid` are incorrectly placed under `target.'cfg(unix)'`. Move them to regular `[dependencies]` and remove duplicate `tempfile`.

## Changes Made

Moved the following from `[target.'cfg(unix)'.dependencies]` to `[dependencies]`:
- `uuid = { workspace = true, features = ["v4"] }`
- `async-trait.workspace = true`
- `derive_builder.workspace = true`
- `ractor.workspace = true`
- `parking_lot.workspace = true`

Left under `target.'cfg(unix)'` (correctly platform-specific):
- `crossterm.workspace = true`
- `nix.workspace = true`

## Acceptance criteria

- [x] `uuid`, `async-trait`, `derive_builder`, `ractor`, `parking_lot` moved to regular `[dependencies]`.
- [x] `crossterm` and `nix` remain Unix-only.
- [x] `cargo check --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Unit tests
- Target compilation check.

### E2E tests
- Host smoke tests.

### Live tmux tests
- Launch host binary.
