# LLM Architecture Findings from ~/Code/agents/* Research

Documenting 10 key findings from litellm and goose architectures for Runie.

---

## Research Summary

### litellm (Python AI Gateway + SDK)
- **Source**: `~/Code/agents/litellm/`
- **Key files**: `ARCHITECTURE.md`, `litellm-rust/`
- **Architecture**: Proxy (gateway) → SDK (provider calls) → LLM API

### litellm-rust (Rust implementation)
- **Source**: `~/Code/agents/litellm/litellm-rust/`
- **Crates**: `ai-gateway` | `core` | `python-bridge`
- **Pattern**: Feature-gated modules (server features)

### goose (Rust agent framework)
- **Source**: `~/Code/agents/goose/crates/goose/`
- **Key modules**: `agents/`, `session/`, `providers/`, `scheduler/`
- **Architecture**: Monolithic agent with modular subsystems

---

## 10 Findings for Runie (Ranked by Impact)

### 1. Provider Transform Pattern
**From**: litellm (`llms/{provider}/chat/transformation.py`)
**Impact**: High

LiteLLM uses per-provider request/response transformers:
```
Handler → Transform: ProviderConfig.transform_request()
Handler → Transform: ProviderConfig.transform_response()
```

**Runie Current**: Provider logic is scattered across modules.
**Recommendation**: Create `provider/transform.rs` with `RequestTransform` and `ResponseTransform` traits for each provider type.

### 2. Hook System Enhancement  
**From**: litellm (`proxy/hooks/`)
**Impact**: High

LiteLLM hooks are composable with priority ordering:
- `max_budget_limiter`
- `parallel_request_limiter`
- `cache_control_check`
- `responses_id_security`

**Runie Current**: `hooks.rs` has AsyncPreRequestHookHandler, AsyncPostRequestHookHandler
**Recommendation**: Add hook priority/ordering and more hook types (budget, rate limiting).

### 3. Hidden Params / Response Cost Tracking
**From**: litellm (`LoggingObj._hidden_params["response_cost"]`)
**Impact**: Medium

LiteLLM tracks cost in `hidden_params` throughout the response lifecycle.

**Runie Current**: `hidden_params.rs` exists with `ResponseCost`
**Recommendation**: Ensure ResponseCost propagates through all provider events and is accessible in UI.

### 4. Streaming Delta Coalescing
**From**: rendering-patterns-survey.md (goose `streaming_buffer.rs`)
**Impact**: Medium

Goose has sophisticated markdown buffer with:
- ParseState tracking 14 state flags
- `push()` returns safe content, `flush()` returns remaining
- `truncate_code_blocks()` with fence detection

**Runie Current**: `streaming_buffer.rs` + `streaming/coalesce.rs` exist
**Recommendation**: Enhance delta coalescing with markdown-aware buffering.

### 5. SSOT Agent Boundaries
**From**: goose (`agents/agent.rs` ~138KB monolithic)
**Impact**: High

Goose has a large monolithic agent file. Runie uses actor-based design with clearer boundaries.

**Runie Current**: `actors/` module with ractor-based actors
**Recommendation**: Strengthen SSOT pattern - each actor owns its state exclusively.

### 6. Router/Factory Pattern for Providers
**From**: litellm (`router.py`)
**Impact**: Medium

LiteLLM Router provides:
- Load balancing across deployments
- Automatic fallback
- Latency-based routing

**Runie Current**: `provider/` module with Provider trait
**Recommendation**: Add `ProviderRouter` for failover/load balancing scenarios.

### 7. Event Taxonomy Classification
**From**: goose event system + rendering-patterns-survey
**Impact**: Medium

Runie has `EventKind` (Intent/Fact/Control) and `EventCategory` classification.

**Runie Current**: Good taxonomy in `event/mod.rs`
**Recommendation**: Ensure all events are classified and use `kind()`/`category()` methods.

### 8. Permission Inspector/Judge Pattern
**From**: goose (`permission/inspector.rs`, `permission_judge.rs`)
**Impact**: High

Goose separates:
- Inspector: Gathers context for decision
- Judge: Makes the decision based on inspector data

**Runie Current**: `permissions/` module exists
**Recommendation**: Add explicit Inspector/Judge separation for better testability.

### 9. MCP Client Integration
**From**: goose (`agents/mcp_client.rs`)
**Impact**: Medium

Goose has sophisticated MCP client with:
- Tool discovery
- Resource management
- Progress tracking

**Runie Current**: `mcp/` module exists
**Recommendation**: Enhance MCP client with connection pooling and better error recovery.

### 10. Session Persistence Strategy
**From**: goose (`session/session_manager.rs` with SQLite)
**Impact**: Medium

Goose uses SQLx + SQLite for:
- Session metadata
- Conversation history
- Token counting
- Extension data

**Runie Current**: `session/` module with SessionStore
**Recommendation**: Consider structured persistence for complex session state.

---

## Architecture Principles for Runie

### Event-Based Architecture ✅
- `Event` enum in `event/mod.rs` is the central bus
- All state transitions happen via events
- View layer subscribes to events

### Async IO ✅
- tokio for async runtime
- `#[async_trait]` for async interfaces
- Streaming with futures

### SSOT Agents ✅
- Actor-based design with ractor
- Each actor owns its state
- Communication via typed messages

### MVU/Pure UI ⚠️
- `view/` module has Elements/Transform
- AppState is the single source of truth
- View is a projection of state

---

## Implementation Recommendations

### Crate Structure (Current - Good)
```
runie-core      - State, Events, Update
runie-tui       - Terminal UI Rendering  
runie-agent     - Tool Execution, Streaming
runie-provider  - LLM Provider Abstraction
runie-cli       - CLI Entrypoint
runie-testing   - Test Helpers
runie-patterns  - Reusable Patterns
```

### Potential Refinements

1. **Extract `provider/transform.rs`**: Dedicated transform traits per provider
2. **Enhance `hooks.rs`**: Add hook priority/ordering system
3. **Strengthen SSOT**: Add explicit state ownership comments
4. **Permission Inspector/Judge**: Separate concerns in `permissions/`
5. **Session persistence**: Consider SQLx for complex session state

---

## Verification Checklist

- [x] Workspace compiles without errors
- [x] Event enum is the central bus
- [x] Actors use typed messages
- [x] AppState is single source of truth
- [x] View is pure projection
- [x] Async IO throughout
- [x] Hidden params tracking works
- [ ] Hooks have priority/ordering
- [ ] Provider transforms are centralized
- [ ] Permission Inspector/Judge separation
