# Collapse three headless binaries into one CLI crate

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: extract-headless-cli-helper
**Blocks**: none

## Description

Three near-identical binary crates existed for non-interactive execution:

| Former Crate | Former Binary | LOC | What it does |
|--------------|--------------|-----|--------------|
| `runie-tui` (alt binary) | `runie-print` | ~60 | `spawn_headless_runtime()` → `run_print()` → `println!` |
| `runie-tui` (alt binary) | `runie-json` | ~220 | `spawn_headless_runtime()` → read JSON stdin → stream JSONL → final JSON |
| `runie-server` | `runie-server` | ~274 | `spawn_headless_runtime()` → TCP/stdio JSON-RPC server loop |

All three called `runie_provider::spawn_headless_runtime()` and `runie_agent::run_headless_turn()`; the only variance is the I/O framing (plain text / JSON / RPC).

**Implemented**: collapsed into one `runie-cli` crate with subcommands:
- `runie print <prompt>` — streaming stdout
- `runie json` — JSON stdin/stdout for scripting  
- `runie server [--stdio]` — TCP/stdio JSON-RPC server

## Acceptance Criteria

- [x] `crates/runie-cli/` exists with a single `main.rs` dispatching on `argv[1]` (`print` | `json` | `server`).
- [x] `runie-print` and `runie-json` removed from `runie-tui`; logic moved to `runie-cli/src/{print,json}.rs`.
- [x] `runie-server` crate deleted; logic moved to `runie-cli/src/server.rs`.
- [x] `Cargo.toml` workspace members list `runie-cli` instead of `runie-server`.
- [x] One set of `{runie-core,runie-agent,runie-provider,runie-protocol,tokio,serde_json,anyhow}` dep declarations.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dispatch_print_mode` — `run_print(["print", "hi"])` routes to the print module.
- [x] `dispatch_json_mode` — `run_json()` routes to the json module.
- [x] `dispatch_server_mode` — `run_server([], false)` routes to the server module.
- [x] `dispatch_unknown_mode_errors` — unknown command prints usage and exits non-zero.
- [x] `json_mode_parses_request` — JSON request parsing tests pass.
- [x] `json_mode_outputs_valid_json` — JSON response serialization tests pass.
- [x] `rpc_parses_request` — RPC request parsing tests pass.
- [x] `rpc_returns_response` — RPC response serialization tests pass.
- [x] `rpc_list_models` — Model catalog listing tests pass.

### Layer 2 — Event Handling
- N/A — CLI dispatch, no TUI events.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Files touched

- `crates/runie-cli/` (new crate: `Cargo.toml`, `src/main.rs`, `src/print.rs`, `src/json.rs`, `src/server.rs`)
- `crates/runie-tui/Cargo.toml` (removed print/json binaries)
- `crates/runie-tui/src/print_main.rs` (deleted)
- `crates/runie-tui/src/json_main.rs` (deleted)
- `crates/runie-server/` (deleted)
- `Cargo.toml` (workspace `members` array, added `runie-cli`, removed `runie-server`)
- `README.md` (modes table updated)
- `crates/runie-agent/src/headless.rs` (comments updated)
- `crates/runie-agent/src/headless_helper.rs` (comments updated)

## Notes

The print and json modes were in `runie-tui` as alternate binaries, not separate crates. The server mode was in a separate `runie-server` crate. All three are now consolidated into `runie-cli`.

The `yolo` flag was removed from server mode since the original implementation didn't use it for the headless options (only `execute_tools: false` was set). If yolo is needed later, it can be added back.

Backward compatibility: if external tooling invokes `runie-server`, `runie-print`, or `runie-json` directly, shim scripts or aliases can be added. The `runie` binary (TUI) is unaffected.
