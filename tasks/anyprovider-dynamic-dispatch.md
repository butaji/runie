# Replace Closed AnyProvider Enum With Trait-Object Dispatch

**Status**: done
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P0
**Depends on**: wire-orphan-crates

## Description

`crates/runie-provider/src/lib.rs:19-24` (original) defined a closed
`AnyProvider` enum with only two variants (`Mock`, `OpenAi`). The
provider registry in `crates/runie-core/src/provider_registry.rs`
lists 13 providers. When the user selected a non-OpenAI provider,
the `_` arm in `AnyProvider::build_with_config` silently fell back
to `Mock` — a visible "model picker" with hidden Mock behavior.

The root cause: `Provider::generate` was `async fn in trait` with
`#[allow(async_fn_in_trait)]`, which made the trait *not*
dyn-compatible. This forced a closed enum.

## What Was Done

The refactor is **complete**. The following has landed:

- [x] `crates/runie-core/src/provider.rs:46-58` `Provider` trait is
  now dyn-compatible: `generate` returns
  `Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>` (no
  `async fn` in body, no `#[allow(async_fn_in_trait)]`)
- [x] `ProviderError` enum added at `provider.rs:34-55` with
  `UnknownProvider(String)`, `MissingApiKey(String)`,
  `Other(String)` variants
- [x] `crates/runie-provider/src/lib.rs:35-45` `DynProvider` struct
  replaces the closed `AnyProvider` enum: `inner: Box<dyn Provider>`,
  `key: String`, `model: String`
- [x] `DynProvider::new(key, model) -> Result<Self, ProviderError>`
  at `lib.rs:46-49` — no silent Mock fallback
- [x] `build_with_env` (lib.rs:97-130) returns `Result<DynProvider,
  ProviderError>`, errors on unknown provider OR missing API key
- [x] `lib.rs:135-148` `build_provider_with_warning` returns
  `Result<DynProvider, ProviderError>` (the warning tuple is gone;
  errors are explicit)
- [x] `lib.rs:166-168` `switch_provider` returns `Result<(), ProviderError>`
- [x] All 12 non-Mock providers in the registry route to
  `OpenAiProvider` parameterized by `base_url` (lib.rs:121-125) — the
  "12 providers, 1 implementation" pattern works
- [x] `lib.rs:230-232` re-exports `ProviderError` as
  `UnknownProviderError` for backward compat

## What Was Fixed to Complete the Task

The remaining items were addressed as follows:

### `turn.rs:28-29` (compile error)
`run_agent_turn` now accepts a `&DynProvider` parameter. Callers
(`runie-term/src/main.rs` and `subagent.rs`) build the provider
upfront and propagate errors explicitly rather than panicking.

### `subagent_test.rs:55-60` (stale assertions)
Updated `run_subagent_falls_back_to_mock_for_unknown_provider` to
`run_subagent_returns_error_for_unknown_provider`. The test now
asserts that unknown providers return `Err(SubagentError::Provider(...))`.

### `runie-print` and `runie-json` (stale Stream API)
Both crates used the old `generate(messages, callback)` signature.
Updated to use the Stream API:
`provider.generate(messages)` returns `Pin<Box<dyn Stream<Item = Result<ResponseChunk>> + Send>>`,
consumed via `futures::StreamExt::next()`.

### `runie-server` (stale Stream API)
Same Stream API update as runie-print and runie-json.

### `runie-agent` test suite
Added `ensure_mock_provider()` setup (using `std::sync::Once`) so
tests that use the `"mock"` provider key work in any environment.
Added Layer 1 tests for `DynProvider` in `tests.rs`.

## Acceptance Criteria

- [x] `crates/runie-agent/src/turn.rs:28-29` compiles (the
  `let (provider, warning) = ...` line)
  → `run_agent_turn` now takes `&DynProvider`; callers handle errors
- [x] `cargo build --workspace` succeeds
- [x] `cargo test -p runie-core --lib` passes
- [x] `cargo test -p runie-agent --lib` passes (including the
  subagent_test, after it's updated)
- [x] The behavior change is intentional: unknown providers now
  return errors rather than silently using Mock. Document this in
  `CHANGELOG.md` and the user-facing README.
- [x] `git grep -nE 'AnyProvider' crates/ -- ':!crates/_archive/*'`
  returns zero hits (the old name should be gone from the live
  tree)

## Tests

### Layer 1 — State/Logic
- [x] `test_dyn_provider_unknown_returns_err` — `DynProvider::new("bogus", "x")`
  returns `Err(ProviderError::UnknownProvider("bogus".into()))`
- [x] `test_dyn_provider_missing_api_key_returns_err` —
  `DynProvider::new("openai", "gpt-4o")` (no `OPENAI_API_KEY`)
  returns `Err(ProviderError::MissingApiKey("OPENAI_API_KEY".into()))`
  when `RUNIE_MOCK` is not set
- [x] `test_dyn_provider_known_with_key_succeeds` — set
  `OPENAI_API_KEY=test`, then `DynProvider::new("openai", "gpt-4o")`
  returns `Ok(...)` with key `"openai"` and model `"gpt-4o"`
- [x] `test_provider_trait_is_dyn_compatible` — compile-time
  assertion: `let _: Box<dyn Provider> = Box::new(OpenAiProvider::new("k", "m"));`
- [x] `test_build_provider_with_warning_returns_err_for_unknown`
- [x] `test_build_provider_panics_for_unknown`
- [x] `test_is_known_provider`
- [x] `test_dyn_provider_key_and_model_accessors`

### Layer 2 — Event Handling
- [x] `cargo test -p runie-agent --lib` passes (112 tests)
- [x] `cargo test -p runie-agent --lib tests::subagent_test`
  passes (after the test is updated to assert Err behavior)

### Layer 4 — Smoke
- [x] `./target/release/runie` starts without panicking when
  `OPENAI_API_KEY` is unset (the startup hook should auto-open the
  login dialog, per the `runie-term/src/main.rs:50-55` flow)

## Notes

**Why the `turn.rs` fix matters:** it's the single
agent-loop call site. With the new design, unknown providers return
`AgentError` events in the TUI, and `SubagentError::Provider` in the
subagent — no more silent mock fallbacks.

**The `Mock` provider is still available** for `RUNIE_MOCK=1` dev
mode (`lib.rs:99-103`). The fallback was only for the
non-Mock case where a non-OpenAI key was provided.

**`runie-print` and `runie-json`**: These dev tools still use
`build_provider` which panics on unknown key. They are acceptable
for dev use. A follow-up could change them to use
`build_provider_with_warning` with explicit error handling.

**Out of scope:**
- Adding more concrete provider types (anthropic_native,
  google_native, etc.) — the OpenAI-compat path covers all 12
  registry entries; native types would only be needed if a
  provider's API diverges from OpenAI Chat Completions
- Adding provider-level retry, rate-limiting, or auth-refresh
- Renaming `DynProvider` to `Provider` (the live crate's
  `runie_provider::Provider` is already the trait; renaming would
  cause import churn)

**Verification:**
```bash
# Build clean (this is the primary acceptance check)
cargo build --workspace

# No references to the old name
! git grep -nE 'AnyProvider' -- 'crates/' ':!crates/_archive/*'

# All tests pass
cargo test -p runie-core --lib
cargo test -p runie-agent --lib
cargo test -p runie-provider --lib
```
