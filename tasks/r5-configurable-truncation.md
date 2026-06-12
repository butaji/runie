# Configurable Tool-Output Truncation

**Status**: done
**Milestone**: R5
**Category**: Tools

## Description

Output truncation (`truncate_head` / `truncate_tail`) was already wired
into all four tools that return user-controlled data (`bash`, `read`,
`grep`, `find`), but the limits were hard-coded (2000 lines / 50 KB).

This change makes the limits configurable via `[truncation]` in
`config.toml` and threads the policy through the tool execution path so each
call honors the configured limits.

## Acceptance Criteria

- [x] `[truncation]` section in `config.toml` (max_lines, max_bytes)
- [x] Missing section → defaults (2000 lines / 50 KB)
- [x] All four tools honor the configured policy
- [x] `TruncationConfig` is TOML-parseable; `TruncationPolicy` is the
      runtime type
- [x] `Tool::execute_with_policy(&policy)` API for callers that need
      explicit policy control

## Out of scope

- Hot-reload of `[truncation]` while the agent is running (would risk
  in-flight tools using a different limit mid-call)
- Middle-truncation strategy (codex-style; only head/tail today)
- Per-tool policy overrides (global policy for now)

## Files

- `crates/runie-agent/src/truncate.rs` — `TruncationConfig` (TOML) +
  `TruncationPolicy` (runtime) + `policy_from_section()` constructor
- `crates/runie-agent/src/tools.rs` — `Tool::execute_with_policy` and
  policy threaded through `run_bash`, `run_grep`, `run_find`, `list_dir`
- `crates/runie-core/src/config_reload.rs` — `TruncationSection`
- `crates/runie-core/src/state.rs` — `ConfigState.truncation`
- `crates/runie-term/src/main.rs` — `init_truncation` wires the parsed
  config into the AgentCommand sent to the agent loop
