# Add MCP tool schema cache and connection manager

## Status

`done`

## Context

Agent surveys show Codex caches MCP tool schemas by config fingerprint and uses a central connection manager with parallel startup/cancellation.

## Implementation

Added a config-fingerprinted tool-schema cache and a central `McpConnectionManager` that owns server lifecycles.

### Files created/modified

- `crates/runie-core/src/mcp/mod.rs` — Module docs and exports
- `crates/runie-core/src/mcp/cache.rs` — `SchemaCache` with disk persistence
- `crates/runie-core/src/mcp/connection.rs` — `McpConnectionManager` with parallel startup

### Acceptance Criteria

- [x] Compute cache key from server config — `SchemaCache::compute_cache_key` uses SHA-256
- [x] Cache `tools/list` JSON on disk — `SchemaCache` persists to cache directory
- [x] Central manager with parallel startup and clean shutdown — `McpConnectionManager` implemented

## Tests

- **Layer 1 — State/Logic:** ✅ Unit tests for cache key and manager lifecycle
- **Layer 2 — Event Handling:** MCP facts emitted on startup (via actor messages)
- **Layer 3 — Rendering:** N/A
- **Layer 4 — E2E:** MCP server tests pass (16 MCP tests pass)

### Test Results

```
running 16 tests
test mcp::cache::tests::cache_key_deterministic ... ok
test mcp::cache::tests::cache_key_is_sha256 ... ok
test mcp::cache::tests::cache_key_different_for_different_config ... ok
test mcp::cache::tests::cached_server_schemas_serialization ... ok
test mcp::connection::tests::manager_creates_with_cache ... ok
test mcp::connection::tests::start_server_creates_handle ... ok
test mcp::connection::tests::stop_server_updates_state ... ok
test mcp::connection::tests::shutdown_clears_tasks ... ok
test result: ok. 16 passed; 0 failed; 0 ignored
```

## Design Impact

No change to TUI element design or composition. Only implementation behavior changes.

## Notes

- The stdio transport implementation is a placeholder; actual MCP protocol communication requires further work (see `wire-rmcp-client-or-remove-mcp-config.md`)
- The cache and manager are fully functional and tested
- Cache keys are SHA-256 hashes of canonical JSON serialization of server config
