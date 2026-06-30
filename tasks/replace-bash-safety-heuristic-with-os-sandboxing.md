# Replace bash safety heuristic with OS sandboxing

## Status

`todo`

## Context

`crates/runie-core/src/bash_safety.rs` is a 230-line hand-rolled heuristic that tries to detect dangerous bash commands. It is easily bypassed (`bash -c 'exec rm -rf /'`, `perl -e 'unlink...'`, `mv` overwriting `.env`).

## Goal

Shrink the heuristic to a small, auditable regex deny-list, then add optional OS-level sandboxing:
- macOS: `sandbox-exec` profile.
- Linux: `landlock` via the `landlock` crate.
- Windows: job objects / Windows Sandbox.

Start as an opt-in `--sandbox` mode so legitimate commands are unaffected.

## Acceptance Criteria

- [ ] Replace the large heuristic with a small regex deny-list for obviously dangerous strings.
- [ ] Implement platform sandbox profiles that deny writes outside cwd/network/sensitive paths.
- [ ] Gate sandboxing behind `--sandbox` (CLI) and a config flag (TUI).
- [ ] Provide graceful fallback when the OS sandbox is unavailable.
- [ ] Existing safe commands still work without the flag.

## Design Impact

No change to TUI element design or composition. Only bash tool security behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for regex deny-list and sandbox profile generation.
- **Layer 2 — Event Handling:** `PermissionRequest` for sandboxed commands emits the correct response.
- **Layer 3 — Rendering:** Permission dialog shows sandbox context if relevant.
- **Layer 4 — E2E:** Headless CLI with `--sandbox` blocks a destructive command and allows a safe one.
- **Live tmux validation:** Run a destructive command with and without `--sandbox`; verify the sandbox blocks it.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
