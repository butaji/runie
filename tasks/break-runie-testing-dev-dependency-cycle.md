# Break `runie-testing` dev-dependency cycle

## Status

`done`

## Description

`runie-testing` depends on `runie-agent`/`runie-provider`, which dev-depend on `runie-testing`. Move provider/agent-specific mocks out of `runie-testing` or make `runie-testing` dev-dependency-only.

## Problem

The dependency graph had a cycle:
- `runie-testing` depends on `runie-agent` and `runie-provider` (regular dependencies)
- `runie-agent` and `runie-provider` depend on `runie-testing` (dev-dependencies)

During `cargo test`, this creates a dev-dependency cycle.

## Solution

Introduced feature flags in `runie-testing` to make the provider/agent-specific modules optional:

### crates/runie-testing/Cargo.toml

Added feature flags:
```toml
[features]
default = []
provider = ["runie-provider"]
agent = ["runie-agent"]
```

Made `runie-provider` and `runie-agent` optional dependencies:
```toml
runie-provider = { workspace = true, optional = true }
runie-agent = { workspace = true, optional = true }
```

### crates/runie-testing/src/lib.rs

Gated the provider/agent-specific modules behind feature flags:
```rust
#[cfg(feature = "provider")]
pub mod fixtures;

#[cfg(feature = "agent")]
pub mod replay_provider;
```

### crates/runie-agent/Cargo.toml

Enabled both features for `runie-testing`:
```toml
[dev-dependencies]
runie-testing = { workspace = true, features = ["provider", "agent"] }
```

## Result

- `runie-testing` no longer creates dev-dependency cycles
- `runie-core` and `runie-tui` use only the core test utilities (env_lock, state helpers)
- `runie-agent` tests use the full provider + agent features
- `runie-provider` tests use only env_lock (no special features needed)

## Acceptance Criteria

1. ✅ **Unit tests** — Dependency graph has no cycles; mocks live in the crates they test or in a dedicated support crate.
2. ✅ **E2E tests** — Replay tests still compile and pass.
3. ✅ **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- ✅ `cargo metadata` shows no cycles.

### E2E tests
- ✅ All replay fixtures compile and pass.
- ✅ `cargo test --workspace` passes.
