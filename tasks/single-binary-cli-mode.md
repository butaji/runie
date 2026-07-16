# Single binary for TUI and CLI modes

## Objective

Unify `runie-tui` and `runie` into a single binary. Launch the TUI by default; launch CLI mode when non-interactive flags (e.g., `--prompt`) are provided.

## Agent landscape finding

codex uses an npm shim around a native binary. gemini-cli and kimi-code ship a single binary that behaves as CLI or TUI depending on invocation.

## runie current state

Runie builds two separate binaries: `runie-tui` and `runie`. The test harness treats them as distinct (`RUNIE_BIN` and `RUNIE_CLI_BIN`).

## Required runie changes

- Merge `runie-tui` and `runie` entry points into one binary.
- Default behavior: start the TUI.
- CLI mode triggers: non-TTY stdin, `--prompt <msg>`, `--output-format json`, or other non-interactive flags.
- Keep backward-compatible symlinks or aliases (`runie-tui` → `runie`) during transition.
- Update `runie-tests` harness to locate the single binary and exercise both modes.

## Test scenarios

1. **Default launch is TUI**
   - Command: `runie`
   - Assert: TUI renders with `Type a message` or onboarding.

2. **Prompt mode streams output**
   - Command: `runie --prompt "say hello" --mock`
   - Assert: response printed to stdout; process exits.

3. **JSON output format**
   - Command: `runie --prompt "say hello" --mock --output-format json`
   - Assert: JSON lines emitted on stdout.

4. **Existing TUI tests still pass**
   - Assert: `AppTest::mock()` and other black-box tests work with the unified binary.

## Edge / negative cases

- Conflicting flags (e.g., `--prompt` with interactive-only flags) produce a clear error.
- CLI mode respects permission mode and does not auto-approve in untrusted projects.

## Dependencies

- `cli_replay_dsl`
- `configured_startup`

## Acceptance checklist

- [ ] Single binary builds and runs in both modes.
- [ ] TUI black-box tests pass.
- [ ] CLI black-box tests pass.
- [ ] No `sleep()` in resulting Rust tests.
