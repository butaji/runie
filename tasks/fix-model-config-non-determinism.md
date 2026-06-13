# Fix Non-Deterministic `selected_models` in Login Flow

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Description

`crates/runie-core/src/login_flow.rs:34` defines
`LoginFlowState::selected_models: HashSet<String>`. The
`HashSet<String>` has non-deterministic iteration order, which
causes `config.toml` to be rewritten with the `models` array in
a different order on every save.

This is a subtle bug: a user adds a provider with 3 models, the
file gets `[A, B, C]`. They save again (without changing
anything), the file gets `[C, A, B]`. Git diffs on `config.toml`
become noisy, and `cargo fmt` / linters that sort files will
keep shuffling the order.

## Root Cause

In `crates/runie-core/src/login_flow.rs:31-34`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LoginFlowState {
    pub step: LoginStep,
    pub provider: String,
    pub key: String,
    pub available_models: Vec<String>,  // ordered
    pub selected_models: HashSet<String>,  // unordered
}
```

The `selected_models` is a `HashSet` to support the "toggle"
semantics (insert/remove in O(1)). But when it's serialized
to `config.toml` via `login_config::save_provider_config` (line
39), the iteration order is unstable.

In `login_flow.rs:80-87`:

```rust
pub fn with_key_and_defaults(self, key: String, default_models: Vec<String>) -> Self {
    let selected_models: HashSet<String> = default_models.iter().cloned().collect();
    Self { ..., selected_models, ... }
}
```

The `default_models` Vec has order, but it's collected into a
HashSet, losing the order.

## Acceptance Criteria

- [ ] `LoginFlowState::selected_models` is `BTreeSet<String>` (or
  `IndexSet<String>` if the `indexmap` crate is acceptable)
- [ ] The `with_key_and_defaults` constructor preserves the
  order from `default_models` (the first model stays first)
- [ ] The `with_fetched_models` constructor preserves the order
  of `fetched: Vec<String>` and adds new models in the order
  they appear
- [ ] `save_provider_config` produces the same `models` array
  order across two consecutive saves (no shuffle)
- [ ] All existing login flow tests still pass
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] `test_selected_models_preserves_order` — given
  `default_models = vec!["A", "B", "C"]`, after
  `with_key_and_defaults`, the iteration order is `["A", "B", "C"]`
- [ ] `test_with_fetched_models_preserves_order` — given
  `fetched = vec!["C", "A"]` (different order), the result has
  models in the order `["C", "A"]`
- [ ] `test_save_provider_config_is_deterministic` — saving the
  same state twice produces byte-identical TOML output
- [ ] `test_login_flow_s13_empty_key_still_shows_defaults` — the
  empty-key scenario still works (regression check)
- [ ] `test_fetched_models_disjoint_list` — the S12 scenario
  still works (regression check)

### Layer 4 — Smoke
- [ ] `crates/_archive/update-orphans/login_flow.rs:790` (the S13
  test from the archived login flow) — but this is in archive,
  not in the test build. Skip; the regression test above
  covers the same behavior.

## Notes

**`BTreeSet` vs `IndexSet`:**
- `BTreeSet<String>`: no new dependency, alphabetical order
- `IndexSet<String>`: preserves insertion order, requires
  `indexmap` crate (not currently a workspace dep)

The semantic intent is "the order in which the user toggled
models" or "the order in which the API returned them". Neither
is "alphabetical". So `BTreeSet` may be wrong if the user
expects "first-toggled-first" order.

For now, the simplest fix is to add an `ordering` field:

```rust
pub struct LoginFlowState {
    pub step: LoginStep,
    pub provider: String,
    pub key: String,
    pub available_models: Vec<String>,
    pub selected_models: HashSet<String>,  // unchanged
    pub selection_order: Vec<String>,    // NEW: preserves order
}
```

And update `save_provider_config` to iterate `selection_order`
instead of `selected_models`. This preserves insertion order
without changing the toggle semantics.

The minimal fix is option 1 (`BTreeSet`). The semantically
correct fix is option 2 (add `selection_order`).

**Out of scope:**
- Reordering the existing tests
- Changing the `available_models: Vec<String>` ordering (it's
  already ordered)
- Refactoring `with_fetched_models` to use a different merge
  strategy
- Removing the HashSet (it serves the O(1) toggle semantics)

**Verification:**
```bash
# Same input produces same output (byte-identical)
cargo test -p runie-core --lib login_flow
# Check that the S12 test still passes

# Save determinism
cargo test -p runie-core --lib login_config
# Add the determinism test as above
```
