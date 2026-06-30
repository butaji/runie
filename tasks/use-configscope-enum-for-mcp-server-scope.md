# Use ConfigScope enum for McpServer scope

**Status**: done

## Context

`crates/runie-core/src/config/mcp.rs` stored `McpServer.scope` as a `String` ("user"/"project"), while `crates/runie-core/src/actors/config/messages.rs` used a typed `ConfigScope` enum. The CLI `main.rs` manually converted strings, leading to duplicate validation and invalid-value risk.

## Implementation

1. **Moved `ConfigScope` to `runie-core/src/config/scope.rs`** with proper derives: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default`, `Serialize`, `Deserialize`, `Display`, `EnumString`, `JsonSchema`.

2. **Changed `McpServer.scope` from `String` to `ConfigScope`** in `crates/runie-core/src/config/mcp.rs`.

3. **Updated CLI to use typed scope** via `ConfigScopeValue` wrapper in `crates/runie-cli/src/scope.rs` that implements `FromStr` and `Default`.

4. **Removed manual string conversions** from CLI `main.rs` and `mcp.rs`.

5. **Regenerated `config.schema.json`** to reflect the new `ConfigScope` type.

**Note:** `clap::ValueEnum` cannot be derived in `runie-core` because `runie-core` doesn't depend on `clap`. Instead, a `ConfigScopeValue` wrapper is used in the CLI that implements `FromStr` for argument parsing.

## Acceptance Criteria

- [x] Change `McpServer.scope` from `String` to `ConfigScope`.
- [x] Use typed scope in CLI (via wrapper that implements FromStr).
- [x] Update CLI `main.rs` to use the typed flag.
- [x] Update TOML serialization/deserialization to use the same string values ("global"/"project").

## Tests

- [x] **Layer 1 — State/Logic:** Unit tests for `ConfigScope` serialization round-trip.
- [x] **Layer 1:** Invalid scope string fails deserialization with a clear error (via FromStr).
- [x] **Layer 1:** CLI tests verify `ConfigScopeValue` parsing.
- [x] **Layer 2 — Event Handling:** `ConfigMsg::AddMcpServer` carries a typed scope.
- [x] **Layer 3 — Rendering:** `McpServer.scope` display shows "global" or "project" correctly.
- [x] **Layer 4 — E2E:** `runie mcp add --scope project ...` uses typed scope internally.

## Files changed

- `crates/runie-core/src/config/scope.rs` (new)
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/src/config/mcp.rs`
- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/config/config_handle.rs`
- `crates/runie-core/src/actors/config/handlers.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-cli/src/main.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-cli/src/scope.rs` (new)
- `config.schema.json`
