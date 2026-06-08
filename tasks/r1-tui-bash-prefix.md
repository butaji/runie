# !command bash prefix

**Status**: done

**Milestone**: R1

**Category**: TUI Improvements

## Description

Run bash and show output, don't send to agent.

## Acceptance Criteria

- [x] ! prefix detection
- [x] Run bash command
- [x] Display output
- [x] Don't add to message queue

## Implementation

### Files Modified

- `crates/runie-core/src/event_bus.rs` — Added `BashOutput` domain event
- `crates/runie-core/src/update/mod.rs` — Added bash module
- `crates/runie-core/src/update/bash.rs` — New module for bash execution
- `crates/runie-core/src/update/input.rs` — Updated submit() to detect '!' prefix

### Architecture

1. **Submit handler** checks for '!' prefix before slash commands
2. **bash::execute_bash()** runs command via `std::process::Command`
3. Output formatted with stdout, stderr, and exit code
4. **add_system_msg()** displays output without sending to agent
5. **request_queue** is not modified for bash commands

### Usage

```
!pwd                    # Run pwd and show output
!ls -la                # Run ls with args
!echo "Hello"          # Echo with output
```

## Tests

### Layer 1 — State/Logic (bash.rs)
- [x] `execute_echo_command` — Basic echo execution
- [x] `execute_pwd_command` — pwd execution
- [x] `command_not_found` — Error handling
- [x] `format_empty_output` — Exit code display
- [x] `format_stdout_only` — stdout formatting
- [x] `format_stderr_included` — stderr handling
- [x] `format_combined_output` — Combined output

### Layer 1 — Integration (input_grapheme.rs)
- [x] `bash_prefix_runs_command` — Command execution on submit
- [x] `bash_prefix_not_sent_to_agent` — Not queued for agent
- [x] `regular_submit_still_works` — Regular submit unchanged

### Layer 2 — Event Handling
- [x] Key mappings (Input chars) work correctly

All 9 bash tests pass.
