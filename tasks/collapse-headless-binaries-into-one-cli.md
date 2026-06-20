# Collapse three headless binaries into one CLI crate

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: extract-headless-cli-helper
**Blocks**: none

## Description

Three near-identical binary crates exist for non-interactive execution:

| Crate | Binary | LOC | What it does |
|-------|--------|-----|--------------|
| `runie-print` | `runie-print` | 95 | `spawn_headless_runtime()` → `run_print()` → `println!` |
| `runie-json` | `runie-json` | 274 | `spawn_headless_runtime()` → read JSON stdin → stream JSONL → final JSON |
| `runie-server` | `runie-server` | 283 | `spawn_headless_runtime()` → TCP/stdio JSON-RPC server loop |

All three call `runie_provider::spawn_headless_runtime()` and `runie_agent::run_headless_turn()`; the only variance is the I/O framing (plain text / JSON / RPC). Three `Cargo.toml`, three link steps, triplicated `runie-{core,agent,provider}` edge declarations. `extract-headless-cli-helper` deduplicates the setup boilerplate but keeps three crates.

YAGNI: collapse into one `runie-cli` crate with subcommands (`runie print`, `runie json`, `runie server`) or a `--mode {print,json,server}` flag. One crate, one manifest, one link, one dep edge set. The TUI binary stays separate (`runie-tui` → `runie`).

## Acceptance Criteria

- [ ] `crates/runie-cli/` exists with a single `main.rs` dispatching on `argv[1]` (`print` | `json` | `server`) or a `--mode` flag.
- [ ] `crates/runie-print/`, `crates/runie-json/`, `crates/runie-server/` deleted; their `main.rs` logic moved into `runie-cli/src/{print,json,server}.rs` modules.
- [ ] `Cargo.toml` workspace members list `runie-cli` instead of the three deleted crates.
- [ ] One set of `{runie-core,runie-agent,runie-provider,runie-protocol,tokio,serde_json,anyhow}` dep declarations (union of the three former crates' features).
- [ ] `./target/release/runie-cli print "find unused imports" < src/main.rs` behaves identically to the former `runie-print`.
- [ ] `./target/release/runie-cli json` reads stdin / emits JSONL identically to former `runie-json`.
- [ ] `./target/release/runie-cli server` binds TCP / speaks the protocol identically to former `runie-server`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dispatch_print_mode` — `dispatch(["print", "hi"])` routes to the print module.
- [ ] `dispatch_json_mode` — `dispatch(["json"])` routes to the json module.
- [ ] `dispatch_server_mode` — `dispatch(["server"])` routes to the server module.
- [ ] `dispatch_unknown_mode_errors` — `dispatch(["bogus"])` prints usage and exits non-zero.

### Layer 2 — Event Handling
- N/A — CLI dispatch, no TUI events.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_subcommand_one_shot` — `runie-cli print "hello"` against a mock provider prints the response.
- [ ] `smoke_json_subcommand_round_trip` — piping a JSON request to `runie-cli json` yields JSONL + final JSON.
- [ ] `smoke_server_subcommand_stdio` — `runie-cli server` over stdio handles an `initialize` + `complete` request pair.
- [ ] `smoke_help_lists_all_modes` — `runie-cli --help` lists `print`, `json`, `server`.

## Files touched

- `crates/runie-cli/` (new crate: `Cargo.toml`, `src/main.rs`, `src/print.rs`, `src/json.rs`, `src/server.rs`)
- `crates/runie-print/` (deleted)
- `crates/runie-json/` (deleted)
- `crates/runie-server/` (deleted)
- `Cargo.toml` (workspace `members` array)
- `README.md` (modes table: update commands)

## Notes

Supersedes `extract-headless-cli-helper` — the helper is still worth extracting (into `runie-agent` or `runie-core::headless_runtime`), but it lives inside the single `runie-cli` rather than being called from three crates. If a future IDE integration needs `runie-server` as a distinct distributable, a cargo feature `server` can gate the server module so the default `runie-cli` build stays slim. Coordinate with `fold-protocol-into-core` — if protocol folds in, `runie-cli`'s server module imports it from the new location. Keep backward-compatible shim symlinks/scripts only if external tooling invokes the old binary names (check `dev.sh`, `justfile`, `scripts/` before deleting the names).
