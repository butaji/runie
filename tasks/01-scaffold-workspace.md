# Step 01: Scaffold workspace from scratch

**Status:** pending
**Depends on:** none

## Goal
Create a minimal, complete Cargo workspace + lint scaffold from an empty working tree (no restore).

## Changes
- `Cargo.toml`: workspace manifest with member `crates/runie-core` and `lint-check`. Workspace edition 2021. Minimal `[workspace.dependencies]` (tokio, async-trait, serde, serde_json, futures, thiserror).
- `.gitignore`: `target/`, `__pycache__/`, `*.rlib`, `.future/`, `.worktrees/`.
- `AGENTS.md`: condensed version of the original agent guidelines (testing strategy layers 1-4, SSOT actor principles, file structure, linter rules).
- `clippy.toml`: `too-many-lines-threshold = 40`.
- `lint-check/Cargo.toml` + `lint-check/src/main.rs`: minimal build-script-style linter enforcing AppState-field-accessor, magic-number (>=1000), and orphan-`tokio::spawn` rules via `regex` over `crates/runie-core/src/**/*.rs`.

## Verification
- `cargo check --workspace` → exit 0.
- `cargo run -p lint-check` → exit 0 with empty file set.
- `ls -A` shows `Cargo.toml`, `Cargo.lock`, `target/`, `.gitignore`, `AGENTS.md`, `clippy.toml`, `crates/`, `lint-check/`, `tasks/`, `.git/`.

## Notes
- Started from empty working tree. Do NOT run `git checkout df499e1f -- .` — that would undo the user's "start from scratch" instruction.
- Workspace dependencies kept intentionally small; add more per-step as needed in `crates/runie-core/Cargo.toml`.