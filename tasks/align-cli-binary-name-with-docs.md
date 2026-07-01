# Align CLI binary name with docs

## Status

`done`

**Completed:** 2026-07-01

## Context

The CLI binary was named `runie-headless`, but README and Architecture docs referred to `runie print`/`runie json`/`runie server`.

## What was done

Chose the **rename approach**: renamed the CLI binary to `runie` and the TUI binary to `runie-tui`.

### Changes

- `crates/runie-cli/Cargo.toml`: renamed binary from `runie-headless` to `runie`
- `crates/runie-tui/Cargo.toml`: renamed binary from `runie` to `runie-tui`
- `README.md`: updated TUI examples from `./target/release/runie` to `./target/release/runie-tui`
- `.cargo/config.toml`: updated `run-tui` alias to use `--bin runie-tui`

### Verification

- `cargo build --release` produces both binaries correctly:
  - `./target/release/runie` — CLI with subcommands (print/inspect/json/server/mcp)
  - `./target/release/runie-tui` — interactive TUI
- `cargo check --workspace` passes
- `cargo test --workspace` passes (one pre-existing flaky test fails in parallel)

## Acceptance Criteria

- [x] Choose rename or doc-update. — **Renamed binaries**
- [x] Apply consistently across `Cargo.toml`, README, docs, scripts. — **Done**
- [x] `cargo build --release` produces expected binaries. — **Verified**
