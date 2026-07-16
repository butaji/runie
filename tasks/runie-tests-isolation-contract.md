# runie-tests isolation contract

## Goal

Document and enforce the isolation boundary for `runie-tests` so the suite
remains a dependency-free, black-box test harness for the `runie` TUI/CLI.

## What "isolated" and "dependency-free" mean

- **No Rust crate dependency on `runie`**: `runie-tests` must not import any
  `runie_*` crate. It drives only compiled `runie-tui` and `runie-cli`
  binaries as subprocesses.
- **No API keys or network calls at test runtime**: Deterministic mock and
  replay fixtures provide all model/tool behavior.
- **No host config leakage**: Every test runs with a temporary `$HOME` and
  sanitized `XDG_*` environment variables.
- **Explicit dependency boundary**: The only accepted dependencies are the
  Rust toolchain, `tmux` >= 3.0, and the pinned `runie` git submodule.

## Current state

- `runie-tests/Cargo.toml` declares no `runie_*` dependencies. ✅
- `runie` is a git submodule. ✅
- Tests run inside real `tmux` sessions with a temp `$HOME`. ✅
- `AGENTS.md`, `README.md`, `EXECUTE.md`, and replay task docs now state the
  isolation boundary. ✅
- Code-level gaps remain and are tracked below for a future implementation
  pass. ❌

## Required changes

1. Add an **Isolation contract** section to `AGENTS.md` listing allowed
   dependencies, forbidden patterns, hermetic runtime guarantees, and binary
   override env vars.
2. Update `README.md` requirements to include the `runie` submodule and
   correct the `RUNIE_BIN` / `RUNIE_CLI_BIN` documentation.
3. Update `EXECUTE.md` to note that tests build the submodule, require `tmux`,
   and need no API keys/network.
4. Update `docs/black-box-replay-testing.md` and the task files
   `tasks/black-box-replay-testing.md` / `tasks/black-box-replay-dsl.md` so
   "standalone" is always paired with the actual dependency boundary
   (submodule + tmux + toolchain OK; no crate imports; no runtime network).
5. Add or update the `runie_tests_isolation_contract` entry in
   `tasks/index.json`.

## Acceptance criteria

- [x] `AGENTS.md` contains an Isolation contract section.
- [x] `README.md` lists the submodule requirement and correct binary overrides.
- [x] `EXECUTE.md` describes the runtime environment boundary.
- [x] `docs/black-box-replay-testing.md` and replay task docs align on the
      meaning of "standalone" and "zero code dependency".
- [x] `tasks/index.json` includes the `runie_tests_isolation_contract` task and
      marks it done.
- [x] Task files (`error_state_rendering`, `startup_onboarding`,
      `black-box-replay-testing`, `black-box-replay-dsl`, `file_references`)
      are aligned with the isolation contract.
- [x] Remaining task files include a black-box scope note referencing
      `AGENTS.md`.
- [x] Code gaps are tracked for a follow-up implementation pass and not
      addressed in this docs-only pass:
      - `RUNIE_BIN` override is documented but ignored by the harness.
      - Build artifacts are written to the shared `runie/target/release` dir.
      - `XDG_CONFIG_HOME` / `XDG_DATA_HOME` can leak host config into tests.
      - Real `list_dir` tool side effects in `tests/mock_list_files.rs`.
      - `sleep()` calls in `tests/error_state_rendering.rs`,
        `tests/mock_list_files.rs`, and `tests/tool_permissions.rs`.

## Dependencies

None.
