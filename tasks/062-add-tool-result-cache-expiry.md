# Add tool result cache with TTL expiry

**Status**: done
**Milestone**: R2
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Add a tool-result cache to `runie-core` that stores results of read-only tool calls keyed by `tool_name + SHA-256(args)`. Cache entries carry a configurable TTL; expired entries are evicted lazily on access and a background task periodically sweeps stale entries. The cache lives in the `IoActor` so it is accessible from tool execution.

## Motivation

Read-only tools like `list_dir`, `grep`, `find`, and `fetch_docs` are deterministic and expensive to re-execute. Caching their results reduces latency and API calls. TTL expiry prevents stale data from confusing the agent.

## Design

- **Cache key**: `Sha256(tool_name + canonical_json(args))` — stable across serializations.
- **Storage**: in-memory `DashMap` owned by `IoActor`, persisted to disk as JSON lines in the session journal.
- **TTL**: configurable via `RunieConfig::cache_ttl_secs` (default 300 s = 5 min). Set to `0` to disable.
- **Scope**: only read-only tools (`BUILTIN_TOOL_NAMES` subset `READ_ONLY_TOOL_NAMES`).
- **Eviction**: lazy on read; background sweep every `cache_ttl_secs / 2`.
- **Thread-safety**: `parking_lot::RwLock` over `DashMap<u64, CacheEntry>`.

## Acceptance Criteria

- [x] `ToolResultCache` struct in `runie-core/src/tool/cache.rs` with `get`, `put`, `evict_expired`, `sweep` methods.
- [x] `CacheEntry` struct with `output: String`, `cached_at: UnixTime`, `tool_name: String`, `bytes_transferred: Option<u64>`.
- [x] `Sha256` cache key computed from `tool_name + canonical_json(args)`.
- [x] `ToolResultCache::new(cache_ttl_secs: u64) -> Arc<Self>` constructor.
- [x] `evict_expired` called lazily inside `get` before returning.
- [x] Background `tokio::spawn` sweep task started in `IoActor::start`.
- [x] Cache plumbed into `tool_runner.rs::execute_tool_call` via `IoActorHandle`.
- [x] Cache disabled (no-op) when `cache_ttl_secs == 0`.
- [x] `cargo test --workspace` passes after the change.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `cache_hit_returns_stored_output` — store then retrieve same key.
- [x] `cache_miss_returns_none` — get non-existent key returns None.
- [x] `cache_key_deterministic` — same name+args produce same key.
- [x] `cache_key_different_for_different_args` — different args produce different key.
- [x] `cache_expiry_returns_none_after_ttl` — entry expired after TTL.
- [x] `evict_expired_removes_stale_entries` — expired entries removed.
- [x] `sweep_removes_all_expired` — periodic sweep removes all stale entries.
- [x] `cache_disabled_when_ttl_zero` — new with ttl=0 returns None on get.

### Layer 2 — Event Handling
- [x] `tool_cache_integrated_in_execute_tool_call` — cache hit skips dispatch.

### Layer 3 — Rendering
N/A — pure logic module.

### Layer 4 — Smoke / Crash
- [x] `tool_cache_smoke_test` — creates cache, puts entry, gets it, evicts.

### Live Tmux Testing Session
- [x] Live session with a directory listing and `grep` confirmed cached on second invocation.

## Files touched

- `crates/runie-core/src/tool/cache.rs` — new file
- `crates/runie-core/src/tool/mod.rs` — re-export cache
- `crates/runie-agent/src/tool_runner.rs` — integrate cache lookup
- `crates/runie-core/src/actors/io/io_actor.rs` — start background sweep
- `crates/runie-core/src/actors/io/mod.rs` — re-export IoActorHandle cache accessor
- `crates/runie-core/src/config.rs` — add `cache_ttl_secs` field
- `crates/runie-testing/src/mock_tool_skill.rs` — update for cache integration
- `crates/runie-core/src/tests/` — add Layer 1 tests

## Notes

- MCP tool schema cache (in `mcp/cache.rs`) is separate from this tool-result cache.
- Does NOT affect tool execution permission flow; cache lookup happens after permission check passes.
- Cache entries do NOT include duration since cached results are served instantly.
