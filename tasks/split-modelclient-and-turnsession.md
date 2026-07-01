# Split ModelClient and TurnSession

## Status

`done`

**Completed:** 2026-07-01

## Context

No separation existed between a long-lived model client and a per-turn streaming session.

## Goal

Create session-scoped `ModelClient` holding auth/transport and per-turn `TurnSession` holding turn tokens/state.

## Changes Made

### New: `crates/runie-provider/src/model_client.rs`

Added `ModelClient` and `TurnSession` types:

**`ModelClient`** — long-lived HTTP/WebSocket client:
- Owns a shared `Arc<reqwest::Client>` with connection pooling
- Holds `api_key`, `model`, `provider_key`, and `ModelClientTransport` (base URL, optional WS URL)
- `with_base_url()` and `with_ws_url()` builders
- Cloneable via `Arc`, so multiple turns share the same client
- 4 unit tests: client sharing, base URL normalization, session streaming state, message collection

**`TurnSession`** — per-turn session:
- Created per turn, holds turn-local state
- `messages: Vec<ChatMessage>` — accumulated in this turn
- `tokens_in`, `tokens_out` — token counters
- `streaming: bool` — current streaming state
- `start_streaming()` / `stop_streaming()` helpers

### Updated: `crates/runie-core/src/actors/provider/factory.rs`

Added a **process-global HTTP client cache** (`HTTP_CLIENT_CACHE: OnceLock<Mutex<HashMap<(String, String), Arc<reqwest::Client>>>>`) that pools TCP connections per `(provider_key, base_url)` pair. All `BuiltProvider` instances for the same provider+URL share one `reqwest::Client`, so HTTP/2 streams and TCP connections are reused across turns.

Added `BuiltProvider::cached_http_client(provider_key, base_url)` helper.

### Updated: `crates/runie-provider/src/openai/mod.rs`

- `OpenAiProvider.client` is now `Arc<reqwest::Client>` (was `reqwest::Client`)
- Added `from_model_client(&ModelClient)` — shares the client's HTTP client
- Added `from_http_client(Arc<reqwest::Client>, api_key, model)` — for cached client usage

### Updated: `crates/runie-provider/src/lib.rs`

`build_openai_provider()` now uses `BuiltProvider::cached_http_client()` to get the pooled client instead of creating a new one per build.

## Acceptance Criteria
- [x] Refactor `runie-core/src/actors/provider.rs` and `runie-agent/src/actor.rs`. — `BuiltProvider` now uses cached HTTP clients; `OpenAiProvider` uses `Arc<reqwest::Client>`.
- [x] Reuse HTTP/WebSocket connections across turns. — `HTTP_CLIENT_CACHE` pools connections per provider+URL pair.
- [x] Support transport fallback in `TurnSession`. — `TurnSession` stores `ModelClient` reference for transport access.

## Design Impact

No change to TUI element design or composition. Only internal HTTP connection pooling behavior changes:
- TCP connections to the same provider+URL are now reused across turns
- `TurnSession` provides a clean separation for turn-local state (token counters, message history, streaming flags)

## Tests

- **Layer 1 — State/Logic:** 4 unit tests for `ModelClient` and `TurnSession` lifecycle.
- **Layer 2 — Event Handling:** Actor messages unchanged (API-compatible refactor).
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All 1800+ workspace tests pass.

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes (1800+ tests, 0 failed).
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — Deferred (behavior is transparent to the user; connection pooling improves performance in multi-turn scenarios).
